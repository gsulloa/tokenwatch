//! Incremental JSONL ingestion for Claude usage logs.
//!
//! Walks `~/.claude/projects/**/*.jsonl`, parses only `type:"assistant"` lines
//! that contain `message.usage`, deduplicates by `"{message.id}:{requestId}"`,
//! and stores events in the SQLite database.
//!
//! Incremental state is tracked in the `ingest_files` table so only new lines
//! (appended since last run) are re-parsed on subsequent polls.

use std::{
    cell::RefCell,
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::SystemTime,
};

use chrono::Utc;
use rusqlite::Connection;
use serde::Deserialize;

use crate::{db, pricing};

// ---------------------------------------------------------------------------
// Serde structs — lax, all optional, tolerates unknown fields
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct RawUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Deserialize, Default)]
struct RawMessage {
    pub id: Option<String>,
    pub model: Option<String>,
    pub usage: Option<RawUsage>,
}

#[derive(Debug, Deserialize, Default)]
struct RawRecord {
    #[serde(rename = "type")]
    pub record_type: Option<String>,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    pub message: Option<RawMessage>,
}

// ---------------------------------------------------------------------------
// Project name derivation
// ---------------------------------------------------------------------------

/// Split a path into its significant components (drops root / prefix).
fn path_components(path: &str) -> Vec<&str> {
    Path::new(path)
        .components()
        .filter(|c| {
            !matches!(
                c,
                std::path::Component::RootDir | std::path::Component::Prefix(_)
            )
        })
        .filter_map(|c| c.as_os_str().to_str())
        .collect()
}

/// Non-Paseo derivation rules applied to already-split components.
fn derive_from_components(components: &[&str]) -> String {
    if components.is_empty() {
        return "unknown".to_owned();
    }

    // Ephemeral agent worktrees (".claude/worktrees/..." or
    // "worktrees/agent-*") → group them all under "unknown".
    for i in 0..components.len() {
        let claude_pair = components[i] == ".claude" && components.get(i + 1) == Some(&"worktrees");
        let agent_pair = components[i] == "worktrees"
            && components
                .get(i + 1)
                .is_some_and(|c| c.starts_with("agent-"));
        if claude_pair || agent_pair {
            return "unknown".to_owned();
        }
    }

    // Conductor layout: "conductor" → "workspaces" → <repo>. Return the repo.
    for i in 0..components.len().saturating_sub(1) {
        if components[i] == "conductor" && components[i + 1] == "workspaces" {
            return match components.get(i + 2) {
                Some(repo) => (*repo).to_owned(),
                None => "unknown".to_owned(),
            };
        }
    }

    // Fallback: last 1–2 significant segments.
    match components.len() {
        1 => components[0].to_owned(),
        n => format!("{}/{}", components[n - 2], components[n - 1]),
    }
}

/// Derive a project name from a path using the non-Paseo rules.
fn derive_non_paseo_name(path: &str) -> String {
    derive_from_components(&path_components(path))
}

// ---------------------------------------------------------------------------
// Paseo worktree resolution
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct RawPaseoWorkspace {
    #[serde(default, rename = "worktreeRoot")]
    worktree_root: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default, rename = "mainRepoRoot")]
    main_repo_root: Option<String>,
}

/// Index of Paseo worktrees → main repo, built from Paseo's internal
/// (undocumented) `~/.paseo/projects/workspaces.json`.
///
/// Not `Sync` (uses `RefCell` for a `.git` fallback cache); only meant for
/// sequential use within a single ingest/backfill run.
#[derive(Debug, Default)]
pub struct PaseoIndex {
    roots: HashMap<String, String>,
    git_cache: RefCell<HashMap<String, Option<String>>>,
}

fn normalize(p: &str) -> String {
    let t = p.trim();
    let stripped = t.trim_end_matches('/');
    if stripped.is_empty() && t.starts_with('/') {
        "/".to_owned()
    } else {
        stripped.to_owned()
    }
}

fn read_gitdir_main_repo(worktree_root: &Path) -> Option<String> {
    let git = worktree_root.join(".git");
    if !git.is_file() {
        return None;
    }
    let contents = std::fs::read_to_string(git).ok()?;
    let rest = contents.trim().strip_prefix("gitdir:")?.trim();
    let idx = rest.find("/.git/")?;
    if idx > 0 {
        Some(rest[..idx].to_owned())
    } else {
        None
    }
}

impl PaseoIndex {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_json_str(json: &str) -> Self {
        let entries = match serde_json::from_str::<Vec<RawPaseoWorkspace>>(json) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "invalid Paseo workspaces.json; ignoring");
                return Self::empty();
            }
        };
        let mut roots = HashMap::new();
        for e in entries {
            let main = match e.main_repo_root.as_deref().map(str::trim) {
                Some(m) if !m.is_empty() => normalize(m),
                _ => continue,
            };
            for key in [e.worktree_root.as_deref(), e.cwd.as_deref()]
                .into_iter()
                .flatten()
            {
                if !key.trim().is_empty() {
                    roots.insert(normalize(key), main.clone());
                }
            }
        }
        Self {
            roots,
            git_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn load_from_home(home: &Path) -> Self {
        let path = home.join(".paseo").join("projects").join("workspaces.json");
        if !path.exists() {
            tracing::debug!(path = %path.display(), "Paseo workspaces.json not found");
            return Self::empty();
        }
        match std::fs::read_to_string(&path) {
            Ok(contents) => Self::from_json_str(&contents),
            Err(e) => {
                tracing::warn!(error = %e, "cannot read Paseo workspaces.json; ignoring");
                Self::empty()
            }
        }
    }

    pub fn load_default() -> Self {
        match home_dir() {
            Some(h) => Self::load_from_home(&h),
            None => Self::empty(),
        }
    }

    fn resolve_main_repo(&self, worktree_root: &str, cwd: &str) -> Option<String> {
        let mut probe = cwd.to_owned();
        loop {
            if let Some(v) = self.roots.get(&probe) {
                return Some(v.clone());
            }
            if probe.len() <= worktree_root.len() {
                break;
            }
            match probe.rfind('/') {
                Some(idx) => probe.truncate(idx),
                None => break,
            }
        }
        if let Some(cached) = self.git_cache.borrow().get(worktree_root) {
            return cached.clone();
        }
        let r = read_gitdir_main_repo(Path::new(worktree_root));
        self.git_cache
            .borrow_mut()
            .insert(worktree_root.to_owned(), r.clone());
        r
    }
}

// ---------------------------------------------------------------------------
// Project name derivation
// ---------------------------------------------------------------------------

/// Derive a human-readable project name from a `cwd` path.
///
/// Rules (in order):
/// 1. `cwd` is `None` or empty → `"unknown"`.
/// 2. Paseo worktree (`.paseo/worktrees/<id>/<slug>[/...]`): resolved to the
///    main repo via the [`PaseoIndex`], then the worktree's `.git` file, then
///    the `<slug>`. Without a slug → `"unknown"`.
/// 3. Ephemeral agent worktree (`.claude/worktrees/...` or
///    `worktrees/agent-*`) → `"unknown"`.
/// 4. If the path contains the consecutive pair `conductor` → `workspaces`,
///    return the component right after `workspaces` (the `<repo>`).
/// 5. Otherwise → last 1–2 significant segments.
///
/// Examples:
/// - `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal` → `freelance/tokenwatch` (with index)
/// - `/Users/x/conductor/workspaces/tub2/chengdu-v4` → `tub2`
/// - `/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd` → `unknown`
/// - `/Users/x/dev/inventures/tub2` → `inventures/tub2`
pub fn derive_project_name(cwd: Option<&str>, paseo: &PaseoIndex) -> String {
    let cwd = match cwd {
        Some(c) if !c.is_empty() => c,
        _ => return "unknown".to_owned(),
    };

    let components = path_components(cwd);
    if components.is_empty() {
        return "unknown".to_owned();
    }

    for i in 0..components.len().saturating_sub(1) {
        if components[i] == ".paseo" && components[i + 1] == "worktrees" {
            let slug = match components.get(i + 3) {
                Some(s) => *s,
                None => return "unknown".to_owned(),
            };
            let worktree_root = format!("/{}", components[..=i + 3].join("/"));
            let cwd_norm = format!("/{}", components.join("/"));
            return match paseo.resolve_main_repo(&worktree_root, &cwd_norm) {
                Some(main) => derive_non_paseo_name(&main),
                None => slug.to_owned(),
            };
        }
    }

    derive_from_components(&components)
}

// ---------------------------------------------------------------------------
// Core ingest logic
// ---------------------------------------------------------------------------

/// Summary returned after a single ingest run.
#[derive(Debug, Default, Clone)]
pub struct IngestSummary {
    /// Number of new events inserted (excluding ignored duplicates).
    pub events_inserted: u64,
    /// Number of files processed (any file that was opened, even if 0 new events).
    pub files_processed: u64,
    /// Number of files skipped (unchanged size+mtime).
    pub files_skipped: u64,
}

/// Locate the Claude projects directory.
///
/// Returns `~/.claude/projects`. Uses `HOME` env var (or `dirs` home_dir).
pub fn claude_projects_dir() -> Option<std::path::PathBuf> {
    Some(home_dir()?.join(".claude").join("projects"))
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| directories::UserDirs::new().map(|d| d.home_dir().to_path_buf()))
}

/// Run a full incremental ingest from the default `~/.claude/projects` directory.
pub fn ingest(conn: &Connection) -> anyhow::Result<IngestSummary> {
    let base = match claude_projects_dir() {
        Some(d) => d,
        None => {
            tracing::warn!("cannot determine home directory; skipping ingest");
            return Ok(IngestSummary::default());
        }
    };
    let paseo = PaseoIndex::load_default();
    ingest_from(conn, &base, &paseo)
}

/// Run a full incremental ingest from an arbitrary base directory.
///
/// Designed so tests can point at a fixture directory without touching the
/// real `~/.claude/projects`.
pub fn ingest_from(
    conn: &Connection,
    base_dir: &Path,
    paseo: &PaseoIndex,
) -> anyhow::Result<IngestSummary> {
    let pattern = base_dir.join("**").join("*.jsonl");
    let pattern_str = pattern.to_string_lossy();

    let mut summary = IngestSummary::default();

    let paths: Vec<_> = glob::glob(&pattern_str)
        .map_err(|e| anyhow::anyhow!("glob error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    for path in paths {
        let path_display = path.display().to_string();
        match ingest_file(conn, &path, paseo) {
            Ok((skipped, inserted)) => {
                if skipped {
                    summary.files_skipped += 1;
                } else {
                    summary.files_processed += 1;
                    summary.events_inserted += inserted;
                }
            }
            Err(e) => {
                tracing::warn!(path = %path_display, error = %e, "failed to ingest file; skipping");
            }
        }
    }

    Ok(summary)
}

/// Ingest a single JSONL file incrementally.
///
/// Returns `(skipped, inserted)` where `skipped=true` means the file was
/// unchanged and not read.
fn ingest_file(conn: &Connection, path: &Path, paseo: &PaseoIndex) -> anyhow::Result<(bool, u64)> {
    let path_str = path.to_string_lossy().to_string();

    let metadata = std::fs::metadata(path)?;
    let size_bytes = metadata.len();
    let mtime = metadata
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    // Check previous ingest state.
    let prev = db::get_ingest_file(conn, &path_str)?;
    let lines_already_ingested: u64 = if let Some((prev_size, prev_mtime, prev_lines)) = prev {
        if prev_size == size_bytes && prev_mtime == mtime {
            // File unchanged — skip entirely.
            return Ok((true, 0));
        }
        // File grew (or metadata changed) — start from previous offset.
        prev_lines
    } else {
        0
    };

    // Open and skip already-ingested lines.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let now = Utc::now().to_rfc3339();
    let mut line_number: u64 = 0;
    let mut inserted: u64 = 0;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!(path = %path.display(), line = line_number, "read error: {e}");
                line_number += 1;
                continue;
            }
        };
        line_number += 1;

        // Skip lines already processed.
        if line_number <= lines_already_ingested {
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse — tolerate malformed JSON by skipping.
        let record: RawRecord = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(path = %path.display(), line = line_number, "skipping malformed JSON: {e}");
                continue;
            }
        };

        // Only process assistant messages with usage data.
        if record.record_type.as_deref() != Some("assistant") {
            continue;
        }
        let message = match record.message {
            Some(m) => m,
            None => continue,
        };
        let raw_usage = match message.usage {
            Some(u) => u,
            None => continue,
        };

        let model = message.model.unwrap_or_default();
        // Claude Code emits synthetic assistant records (interruptions, local
        // API-error placeholders) with model `<synthetic>`. These are not real
        // API usage — skip them so they don't pollute the chart or spam the
        // unknown-model cost log.
        if model == "<synthetic>" {
            continue;
        }
        let msg_id = message.id.unwrap_or_default();
        let req_id = record.request_id.unwrap_or_default();

        // Build dedup key — use whatever IDs are available.
        let dedup_key = match (msg_id.is_empty(), req_id.is_empty()) {
            (false, false) => format!("{msg_id}:{req_id}"),
            (false, true) => msg_id.clone(),
            (true, false) => req_id.clone(),
            (true, true) => {
                // No usable ID — skip to avoid false deduplication of unrelated rows.
                tracing::debug!(path = %path.display(), line = line_number, "skipping record with no id");
                continue;
            }
        };

        let session_id = record.session_id.unwrap_or_default();
        let timestamp = record.timestamp.unwrap_or_default();
        if timestamp.is_empty() {
            tracing::debug!(path = %path.display(), line = line_number, "skipping record with no timestamp");
            continue;
        }

        let project_path = record.cwd.clone().unwrap_or_default();
        let project_name = derive_project_name(record.cwd.as_deref(), paseo);

        let usage = pricing::Usage {
            input_tokens: raw_usage.input_tokens,
            output_tokens: raw_usage.output_tokens,
            cache_creation_tokens: raw_usage.cache_creation_input_tokens,
            cache_read_tokens: raw_usage.cache_read_input_tokens,
        };
        let cost = pricing::cost(&model, &usage);
        let total_tokens = usage.input_tokens
            + usage.output_tokens
            + usage.cache_creation_tokens
            + usage.cache_read_tokens;

        let event_row = db::UsageEventRow {
            dedup_key: &dedup_key,
            session_id: &session_id,
            project_path: &project_path,
            project_name: &project_name,
            model: &model,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_creation_tokens: usage.cache_creation_tokens,
            cache_read_tokens: usage.cache_read_tokens,
            total_tokens,
            cost,
            timestamp: &timestamp,
            git_branch: record.git_branch.as_deref(),
            ingested_at: &now,
        };

        db::insert_event(conn, &event_row)?;
        inserted += 1;
    }

    // Update ingest file progress.
    db::upsert_ingest_file(conn, &path_str, size_bytes, mtime, line_number, &now)?;

    Ok((false, inserted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::io::Write;
    use tempfile::TempDir;

    fn test_conn() -> Connection {
        db::open_at(":memory:").expect("in-memory db")
    }

    /// Create a fake `.claude/projects/<proj>/<sess>.jsonl` tree under `tmp`.
    fn make_fixture_dir(tmp: &TempDir, rel_path: &str, content: &str) -> std::path::PathBuf {
        let full = tmp.path().join(rel_path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        let mut f = File::create(&full).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        full
    }

    const FIXTURE_LINE_1: &str = r#"{"type":"assistant","requestId":"req_001","sessionId":"sess_a","cwd":"/Users/x/conductor/workspaces/backend/madrid","timestamp":"2026-07-01T10:00:00.000Z","gitBranch":"main","message":{"id":"msg_001","model":"claude-opus-4-8","usage":{"input_tokens":10000,"output_tokens":200,"cache_creation_input_tokens":5000,"cache_read_input_tokens":2000}}}"#;

    const FIXTURE_LINE_2: &str = r#"{"type":"assistant","requestId":"req_002","sessionId":"sess_a","cwd":"/Users/x/conductor/workspaces/backend/madrid","timestamp":"2026-07-02T08:00:00.000Z","message":{"id":"msg_002","model":"claude-sonnet-4-6","usage":{"input_tokens":500,"output_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":50}}}"#;

    // A user-type line that must be ignored
    const FIXTURE_USER_LINE: &str = r#"{"type":"user","requestId":"req_usr","sessionId":"sess_a","cwd":"/Users/x/proj","timestamp":"2026-07-01T09:00:00.000Z","message":{"content":"hello"}}"#;

    // A malformed line that must be skipped
    const FIXTURE_MALFORMED: &str = r#"{"type":"assistant","requestId":BROKEN_JSON"#;

    // An assistant line without usage
    const FIXTURE_NO_USAGE: &str = r#"{"type":"assistant","requestId":"req_nousage","sessionId":"sess_b","timestamp":"2026-07-01T11:00:00.000Z","message":{"id":"msg_nousage","model":"claude-opus-4-8"}}"#;

    // A line without cwd → project_name should be "unknown"
    const FIXTURE_NO_CWD: &str = r#"{"type":"assistant","requestId":"req_nocwd","sessionId":"sess_c","timestamp":"2026-07-01T12:00:00.000Z","message":{"id":"msg_nocwd","model":"claude-haiku-4-5","usage":{"input_tokens":100,"output_tokens":20,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;

    // A synthetic assistant record (Claude Code interruption/placeholder) → skipped
    const FIXTURE_SYNTHETIC: &str = r#"{"type":"assistant","requestId":"req_synth","sessionId":"sess_d","cwd":"/Users/x/proj","timestamp":"2026-07-01T13:00:00.000Z","message":{"id":"msg_synth","model":"<synthetic>","usage":{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;

    // -----------------------------------------------------------------------
    // derive_project_name — Conductor-aware tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_derive_conductor_workspace_root() {
        // /Users/x/conductor/workspaces/tub2/chengdu-v4 → tub2
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/tub2/chengdu-v4"),
                &PaseoIndex::empty()
            ),
            "tub2"
        );
    }

    #[test]
    fn test_derive_conductor_city_with_version_suffix() {
        // /Users/x/conductor/workspaces/argus/belo-horizonte-v1 → argus
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/argus/belo-horizonte-v1"),
                &PaseoIndex::empty()
            ),
            "argus"
        );
    }

    #[test]
    fn test_derive_conductor_submodule_deep_path() {
        // .../tub2/dili/FRONT/e2e/operator-assignment → tub2
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/tub2/dili/FRONT/e2e/operator-assignment"),
                &PaseoIndex::empty()
            ),
            "tub2"
        );
    }

    #[test]
    fn test_derive_conductor_monorepo_subpath() {
        // .../argus/cairo/packages/app/src-tauri → argus
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/argus/cairo/packages/app/src-tauri"),
                &PaseoIndex::empty()
            ),
            "argus"
        );
    }

    #[test]
    fn test_derive_worktrees_component() {
        // Any path with a "worktrees" component → grouped under "unknown"
        assert_eq!(
            derive_project_name(
                Some("/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd"),
                &PaseoIndex::empty()
            ),
            "unknown"
        );
        assert_eq!(
            derive_project_name(
                Some("/Users/x/worktrees/agent-1234/packages/app"),
                &PaseoIndex::empty()
            ),
            "unknown"
        );
    }

    const PASEO_JSON: &str = r#"[
      {"workspaceId":"wks_1","kind":"worktree","cwd":"/Users/x/.paseo/worktrees/3qkyl933/resolute-seal","worktreeRoot":"/Users/x/.paseo/worktrees/3qkyl933/resolute-seal","mainRepoRoot":"/Users/x/dev/freelance/tokenwatch","archivedAt":null},
      {"workspaceId":"wks_2","kind":"worktree","cwd":"/Users/x/.paseo/worktrees/abcd1234/upbeat-bear","worktreeRoot":"/Users/x/.paseo/worktrees/abcd1234/upbeat-bear","mainRepoRoot":"/Users/x/dev/meki/backend","archivedAt":"2026-10-01T00:00:00Z","foo":1},
      {"workspaceId":"wks_3","kind":"local_checkout","cwd":"/Users/x/dev/meki/console","worktreeRoot":"/Users/x/dev/meki/console","mainRepoRoot":null}
    ]"#;

    fn paseo_idx() -> PaseoIndex {
        PaseoIndex::from_json_str(PASEO_JSON)
    }

    #[test]
    fn test_derive_paseo_worktree_root_uses_main_repo() {
        assert_eq!(
            derive_project_name(
                Some("/Users/x/.paseo/worktrees/3qkyl933/resolute-seal"),
                &paseo_idx()
            ),
            "freelance/tokenwatch"
        );
    }

    #[test]
    fn test_derive_paseo_subdir_uses_main_repo() {
        let idx = paseo_idx();
        assert_eq!(
            derive_project_name(
                Some("/Users/x/.paseo/worktrees/abcd1234/upbeat-bear/dev-wiki"),
                &idx
            ),
            "meki/backend"
        );
        assert_eq!(
            derive_project_name(
                Some("/Users/x/.paseo/worktrees/abcd1234/upbeat-bear/a/b/c"),
                &idx
            ),
            "meki/backend"
        );
    }

    #[test]
    fn test_derive_paseo_trailing_slash_in_index() {
        let idx = PaseoIndex::from_json_str(
            r#"[{"worktreeRoot":"/h/.paseo/worktrees/zz/slug/","mainRepoRoot":"/h/dev/a/b/"}]"#,
        );
        assert_eq!(
            derive_project_name(Some("/h/.paseo/worktrees/zz/slug"), &idx),
            "a/b"
        );
    }

    fn make_git_worktree(tmp: &TempDir, git_contents: Option<&str>) -> std::path::PathBuf {
        let root = tmp.path().join(".paseo/worktrees/id1/brave-otter");
        std::fs::create_dir_all(root.join("src")).unwrap();
        if let Some(c) = git_contents {
            std::fs::write(root.join(".git"), c).unwrap();
        }
        root
    }

    #[test]
    fn test_derive_paseo_git_file_fallback() {
        let tmp = TempDir::new().unwrap();
        let root = make_git_worktree(
            &tmp,
            Some("gitdir: /Users/x/dev/meki/backend/.git/worktrees/brave-otter\n"),
        );
        let idx = PaseoIndex::empty();
        assert_eq!(
            derive_project_name(Some(root.to_str().unwrap()), &idx),
            "meki/backend"
        );
        assert_eq!(
            derive_project_name(Some(root.join("src").to_str().unwrap()), &idx),
            "meki/backend"
        );
    }

    #[test]
    fn test_derive_paseo_slug_last_resort() {
        let idx = PaseoIndex::empty();
        assert_eq!(
            derive_project_name(
                Some("/nonexistent-tw-test/.paseo/worktrees/zz99/brave-otter"),
                &idx
            ),
            "brave-otter"
        );
        assert_eq!(
            derive_project_name(
                Some("/nonexistent-tw-test/.paseo/worktrees/zz99/brave-otter/sub/dir"),
                &idx
            ),
            "brave-otter"
        );
    }

    #[test]
    fn test_derive_paseo_incomplete_path_is_unknown() {
        let idx = PaseoIndex::empty();
        assert_eq!(
            derive_project_name(Some("/Users/x/.paseo/worktrees"), &idx),
            "unknown"
        );
        assert_eq!(
            derive_project_name(Some("/Users/x/.paseo/worktrees/zz99"), &idx),
            "unknown"
        );
    }

    #[test]
    fn test_derive_paseo_git_file_without_marker_falls_back_to_slug() {
        let tmp = TempDir::new().unwrap();
        let root = make_git_worktree(&tmp, Some("gitdir: /weird/path"));
        assert_eq!(
            derive_project_name(Some(root.to_str().unwrap()), &PaseoIndex::empty()),
            "brave-otter"
        );

        let tmp2 = TempDir::new().unwrap();
        let root2 = make_git_worktree(&tmp2, None);
        std::fs::create_dir(root2.join(".git")).unwrap();
        assert_eq!(
            derive_project_name(Some(root2.to_str().unwrap()), &PaseoIndex::empty()),
            "brave-otter"
        );
    }

    #[test]
    fn test_derive_claude_agent_worktrees_still_unknown() {
        let idx = paseo_idx();
        for p in [
            "/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd",
            "/Users/x/conductor/workspaces/tub2/kelowna-v1/FRONT/.claude/worktrees/agent-ad31",
            "/Users/x/worktrees/agent-1234/packages/app",
        ] {
            assert_eq!(derive_project_name(Some(p), &idx), "unknown", "{p}");
        }
    }

    #[test]
    fn test_derive_generic_worktrees_component_uses_fallback() {
        assert_eq!(
            derive_project_name(
                Some("/Users/x/dev/worktrees/feature-x"),
                &PaseoIndex::empty()
            ),
            "worktrees/feature-x"
        );
    }

    #[test]
    fn test_paseo_index_malformed_json_is_empty() {
        for json in ["{not json", "{\"a\":1}", "[{\"mainRepoRoot\": 5}]"] {
            let idx = PaseoIndex::from_json_str(json);
            assert_eq!(
                derive_project_name(Some("/nonexistent-tw-test/.paseo/worktrees/i/slug-a"), &idx),
                "slug-a",
                "{json}"
            );
        }
    }

    #[test]
    fn test_paseo_index_load_from_home() {
        let cwd = "/Users/x/.paseo/worktrees/3qkyl933/resolute-seal";

        let tmp = TempDir::new().unwrap();
        let idx = PaseoIndex::load_from_home(tmp.path());
        assert_eq!(derive_project_name(Some(cwd), &idx), "resolute-seal");

        let dir = tmp.path().join(".paseo/projects");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("workspaces.json"), PASEO_JSON).unwrap();
        let idx = PaseoIndex::load_from_home(tmp.path());
        assert_eq!(derive_project_name(Some(cwd), &idx), "freelance/tokenwatch");

        std::fs::write(dir.join("workspaces.json"), "\u{0}garbage").unwrap();
        let idx = PaseoIndex::load_from_home(tmp.path());
        assert_eq!(derive_project_name(Some(cwd), &idx), "resolute-seal");
    }

    #[test]
    fn test_ingest_paseo_cwd_attributed_to_main_repo() {
        let tmp = TempDir::new().unwrap();
        let cwd = "/Users/x/.paseo/worktrees/3qkyl933/resolute-seal";
        let line = FIXTURE_LINE_1.replace("/Users/x/conductor/workspaces/backend/madrid", cwd);
        make_fixture_dir(&tmp, "proj/sess.jsonl", &line);
        let conn = test_conn();
        let summary = ingest_from(&conn, tmp.path(), &paseo_idx()).unwrap();
        assert_eq!(summary.events_inserted, 1);
        let (name, path): (String, String) = conn
            .query_row(
                "SELECT project_name, project_path FROM usage_events",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "freelance/tokenwatch");
        assert_eq!(path, cwd);
    }

    #[test]
    fn test_derive_non_conductor_path_keeps_fallback() {
        // /Users/x/dev/inventures/tub2 → inventures/tub2
        assert_eq!(
            derive_project_name(Some("/Users/x/dev/inventures/tub2"), &PaseoIndex::empty()),
            "inventures/tub2"
        );
    }

    // A Conductor path resolves to the repo, even at two segments after ws.
    #[test]
    fn test_derive_project_name_two_segments() {
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/backend/madrid"),
                &PaseoIndex::empty()
            ),
            "backend"
        );
    }

    #[test]
    fn test_derive_project_name_one_segment() {
        // "/project" has only one meaningful segment after stripping the root → "project"
        assert_eq!(
            derive_project_name(Some("/project"), &PaseoIndex::empty()),
            "project"
        );
    }

    #[test]
    fn test_derive_project_name_none() {
        assert_eq!(derive_project_name(None, &PaseoIndex::empty()), "unknown");
    }

    #[test]
    fn test_derive_project_name_empty() {
        assert_eq!(
            derive_project_name(Some(""), &PaseoIndex::empty()),
            "unknown"
        );
    }

    #[test]
    fn test_ingest_fixture_produces_expected_rows() {
        let tmp = TempDir::new().unwrap();
        let content = format!(
            "{FIXTURE_LINE_1}\n{FIXTURE_USER_LINE}\n{FIXTURE_MALFORMED}\n{FIXTURE_NO_USAGE}\n"
        );
        make_fixture_dir(&tmp, "proj/session.jsonl", &content);

        let conn = test_conn();
        let summary = ingest_from(&conn, tmp.path(), &PaseoIndex::empty()).unwrap();

        // Only FIXTURE_LINE_1 is a valid assistant line with usage
        assert_eq!(summary.events_inserted, 1, "expected 1 event from fixture");
        assert_eq!(summary.files_processed, 1);
        assert_eq!(summary.files_skipped, 0);

        let (model, project_name): (String, String) = conn
            .query_row(
                "SELECT model, project_name FROM usage_events WHERE dedup_key = 'msg_001:req_001'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();

        assert_eq!(model, "claude-opus-4-8");
        // The Conductor-aware rule extracts the repo ("backend") from the
        // path /Users/x/conductor/workspaces/backend/madrid.
        assert_eq!(project_name, "backend");
    }

    #[test]
    fn test_ingest_skips_synthetic_model() {
        let tmp = TempDir::new().unwrap();
        let content = format!("{FIXTURE_LINE_1}\n{FIXTURE_SYNTHETIC}\n");
        make_fixture_dir(&tmp, "proj/session.jsonl", &content);

        let conn = test_conn();
        let summary = ingest_from(&conn, tmp.path(), &PaseoIndex::empty()).unwrap();

        // Only FIXTURE_LINE_1 is real; the <synthetic> record is skipped.
        assert_eq!(summary.events_inserted, 1);
        let synthetic_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM usage_events WHERE model = '<synthetic>'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(synthetic_rows, 0, "synthetic records must not be stored");
    }

    #[test]
    fn test_ingest_dedup_same_id_twice() {
        let tmp = TempDir::new().unwrap();
        // Same line twice → should produce exactly 1 row
        let content = format!("{FIXTURE_LINE_1}\n{FIXTURE_LINE_1}\n");
        make_fixture_dir(&tmp, "proj/session.jsonl", &content);

        let conn = test_conn();
        ingest_from(&conn, tmp.path(), &PaseoIndex::empty()).unwrap();

        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "duplicate lines must produce exactly 1 row");
    }

    #[test]
    fn test_ingest_project_name_unknown_when_no_cwd() {
        let tmp = TempDir::new().unwrap();
        make_fixture_dir(&tmp, "proj/session.jsonl", &format!("{FIXTURE_NO_CWD}\n"));

        let conn = test_conn();
        ingest_from(&conn, tmp.path(), &PaseoIndex::empty()).unwrap();

        let project_name: String = conn
            .query_row(
                "SELECT project_name FROM usage_events WHERE dedup_key = 'msg_nocwd:req_nocwd'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(project_name, "unknown");
    }

    #[test]
    fn test_incremental_ingest_grown_file_no_duplicates() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("proj").join("session.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        // Write first line only.
        {
            let mut f = File::create(&path).unwrap();
            writeln!(f, "{FIXTURE_LINE_1}").unwrap();
        }

        let conn = test_conn();
        let s1 = ingest_from(&conn, tmp.path(), &PaseoIndex::empty()).unwrap();
        assert_eq!(s1.events_inserted, 1);

        // Touch file time to force re-check even if size didn't change much.
        // Append a second line to grow the file.
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            writeln!(f, "{FIXTURE_LINE_2}").unwrap();
        }

        let s2 = ingest_from(&conn, tmp.path(), &PaseoIndex::empty()).unwrap();
        assert_eq!(
            s2.events_inserted, 1,
            "only the new line should be inserted"
        );

        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2, "total should be 2 after incremental ingest");
    }

    #[test]
    fn test_incremental_unchanged_file_skipped() {
        let tmp = TempDir::new().unwrap();
        make_fixture_dir(&tmp, "proj/session.jsonl", &format!("{FIXTURE_LINE_1}\n"));

        let conn = test_conn();
        let s1 = ingest_from(&conn, tmp.path(), &PaseoIndex::empty()).unwrap();
        assert_eq!(s1.files_skipped, 0);
        assert_eq!(s1.files_processed, 1);

        // Second run — file unchanged.
        let s2 = ingest_from(&conn, tmp.path(), &PaseoIndex::empty()).unwrap();
        assert_eq!(s2.files_skipped, 1, "unchanged file must be skipped");
        assert_eq!(s2.files_processed, 0);
    }

    #[test]
    fn test_real_fixture_file() {
        // Uses the fixture under tests/fixtures/ — verifies the shared fixture parses correctly.
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");

        let conn = test_conn();
        let summary = ingest_from(&conn, &fixture_dir, &PaseoIndex::empty()).unwrap();

        // The fixture has 5 valid assistant lines (lines 1,3,4,5,6) — line 6 has unknown model
        // but still inserts (cost=0). Malformed JSON line and user line are skipped.
        assert!(
            summary.events_inserted >= 4,
            "expected at least 4 events from fixture, got {}",
            summary.events_inserted
        );
    }
}
