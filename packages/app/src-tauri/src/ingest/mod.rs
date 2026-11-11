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
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::{Duration, SystemTime},
};

use chrono::Utc;
use rusqlite::{Connection, OpenFlags};
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

/// `.claude/worktrees/...` or `worktrees/agent-*` (ephemeral agent worktrees).
fn is_agent_worktree(components: &[&str]) -> bool {
    (0..components.len()).any(|i| {
        let claude_pair = components[i] == ".claude" && components.get(i + 1) == Some(&"worktrees");
        let agent_pair = components[i] == "worktrees"
            && components
                .get(i + 1)
                .is_some_and(|c| c.starts_with("agent-"));
        claude_pair || agent_pair
    })
}

/// Non-Paseo derivation rules applied to already-split components.
fn derive_from_components(components: &[&str]) -> String {
    if components.is_empty() {
        return "unknown".to_owned();
    }

    // Ephemeral agent worktrees (".claude/worktrees/..." or
    // "worktrees/agent-*") → group them all under "unknown".
    if is_agent_worktree(components) {
        return "unknown".to_owned();
    }

    // Conductor layout: "conductor" → "workspaces" → <repo>. Return the repo.
    // Unresolved fallback; `derive_project_name` resolves Conductor first.
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
#[derive(Debug, Default)]
pub struct PaseoIndex {
    roots: HashMap<String, String>,
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
        Self { roots }
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

    fn lookup(&self, worktree_root: &str, cwd: &str) -> Option<String> {
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
        None
    }

    fn main_roots(&self) -> impl Iterator<Item = &str> {
        self.roots.values().map(String::as_str)
    }
}

// ---------------------------------------------------------------------------
// Conductor workspace resolution
// ---------------------------------------------------------------------------

/// Index of Conductor workspaces → main repo root, read from Conductor's
/// internal (undocumented) SQLite DB
/// `~/Library/Application Support/com.conductor.app/conductor.db`
/// (tables `repos(id, name, root_path)` and
/// `workspaces(repository_id, workspace_path)`). Best effort: any failure
/// yields an empty index.
#[derive(Debug, Default)]
pub struct ConductorIndex {
    /// normalized workspace_path -> normalized repo root_path
    workspaces: HashMap<String, String>,
    /// repo dir segment (component after `conductor/workspaces`) -> root_path; unambiguous only
    repos: HashMap<String, String>,
}

impl ConductorIndex {
    pub fn empty() -> Self {
        Self::default()
    }

    /// `workspaces`: (workspace_path, repo_root_path); `repos`: (repo_name, repo_root_path).
    pub fn from_entries<W, R>(workspaces: W, repos: R) -> Self
    where
        W: IntoIterator<Item = (String, String)>,
        R: IntoIterator<Item = (String, String)>,
    {
        let mut ws_map: HashMap<String, String> = HashMap::new();
        let mut candidates: HashMap<String, HashSet<String>> = HashMap::new();

        for (ws, root) in workspaces {
            if ws.trim().is_empty() || root.trim().is_empty() {
                continue;
            }
            let ws = normalize(&ws);
            let root = normalize(&root);
            let comps = path_components(&ws);
            for i in 0..comps.len().saturating_sub(1) {
                if comps[i] == "conductor"
                    && comps[i + 1] == "workspaces"
                    && comps.get(i + 2).is_some()
                {
                    candidates
                        .entry(comps[i + 2].to_owned())
                        .or_default()
                        .insert(root.clone());
                    break;
                }
            }
            ws_map.insert(ws, root);
        }
        for (name, root) in repos {
            if name.trim().is_empty() || root.trim().is_empty() {
                continue;
            }
            candidates
                .entry(name.trim().to_owned())
                .or_default()
                .insert(normalize(&root));
        }

        let repos_map = candidates
            .into_iter()
            .filter_map(|(k, v)| {
                if v.len() == 1 {
                    v.into_iter().next().map(|r| (k, r))
                } else {
                    None
                }
            })
            .collect();

        Self {
            workspaces: ws_map,
            repos: repos_map,
        }
    }

    pub fn load_from_db(path: &Path) -> Self {
        if !path.exists() {
            tracing::debug!(path = %path.display(), "Conductor DB not found");
            return Self::empty();
        }
        match read_conductor_db(path) {
            Ok((ws, repos)) => Self::from_entries(ws, repos),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "cannot read Conductor DB; ignoring");
                Self::empty()
            }
        }
    }

    pub fn load_from_home(home: &Path) -> Self {
        Self::load_from_db(
            &home
                .join("Library")
                .join("Application Support")
                .join("com.conductor.app")
                .join("conductor.db"),
        )
    }

    fn workspace_main(&self, ws_root: &str) -> Option<&str> {
        self.workspaces.get(ws_root).map(String::as_str)
    }

    fn repo_main(&self, repo: &str) -> Option<&str> {
        self.repos.get(repo).map(String::as_str)
    }

    fn main_roots(&self) -> impl Iterator<Item = &str> {
        self.workspaces
            .values()
            .chain(self.repos.values())
            .map(String::as_str)
    }
}

type ConductorRows = (Vec<(String, String)>, Vec<(String, String)>);

fn query_conductor(conn: &Connection) -> rusqlite::Result<ConductorRows> {
    conn.busy_timeout(Duration::from_millis(250))?;
    let mut stmt = conn.prepare(
        "SELECT w.workspace_path, r.root_path
         FROM workspaces w JOIN repos r ON r.id = w.repository_id
         WHERE w.workspace_path IS NOT NULL AND w.workspace_path <> ''
           AND r.root_path IS NOT NULL AND r.root_path <> ''",
    )?;
    let ws = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut stmt = conn.prepare(
        "SELECT name, root_path FROM repos
         WHERE name IS NOT NULL AND name <> '' AND root_path IS NOT NULL AND root_path <> ''",
    )?;
    let repos = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok((ws, repos))
}

/// Open Conductor's DB strictly read-only. Conductor uses WAL; when it is not
/// running there is no `-shm`, and a plain read-only open fails. In that case
/// (and only if no `-wal` exists, i.e. no writer has pending frames) retry
/// with `immutable=1`. Never opens read-write, never creates sidecar files.
fn read_conductor_db(path: &Path) -> rusqlite::Result<ConductorRows> {
    let ro = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let first = Connection::open_with_flags(path, ro).and_then(|c| query_conductor(&c));
    match first {
        Ok(v) => Ok(v),
        Err(e) => {
            let mut wal = path.as_os_str().to_owned();
            wal.push("-wal");
            match sqlite_immutable_uri(path) {
                Some(uri) if !Path::new(&wal).exists() => {
                    let c = Connection::open_with_flags(uri, ro | OpenFlags::SQLITE_OPEN_URI)?;
                    query_conductor(&c)
                }
                _ => Err(e),
            }
        }
    }
}

/// `file:` URI with percent-encoding for everything outside `[A-Za-z0-9-._~/]`.
fn sqlite_immutable_uri(path: &Path) -> Option<String> {
    use std::fmt::Write;
    let s = path.to_str()?;
    let mut out = String::from("file:");
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out.push_str("?mode=ro&immutable=1");
    Some(out)
}

// ---------------------------------------------------------------------------
// Combined worktree index
// ---------------------------------------------------------------------------

/// All external worktree knowledge used to attribute sessions to a main repo.
/// Loaded once per ingest run / backfill. Not `Sync` (RefCell cache).
#[derive(Debug, Default)]
pub struct WorktreeIndex {
    paseo: PaseoIndex,
    conductor: ConductorIndex,
    /// basename(main root) -> main root, over Paseo mainRepoRoots ∪ Conductor roots; unique only
    known_roots: HashMap<String, String>,
    git_cache: RefCell<HashMap<String, Option<String>>>,
}

impl WorktreeIndex {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new(paseo: PaseoIndex, conductor: ConductorIndex) -> Self {
        let mut candidates: HashMap<String, HashSet<String>> = HashMap::new();
        for root in paseo.main_roots().chain(conductor.main_roots()) {
            let base = Path::new(root)
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .filter(|b| !b.is_empty());
            if let Some(base) = base {
                candidates
                    .entry(base.to_owned())
                    .or_default()
                    .insert(root.to_owned());
            }
        }
        let known_roots = candidates
            .into_iter()
            .filter_map(|(k, v)| {
                if v.len() == 1 {
                    v.into_iter().next().map(|r| (k, r))
                } else {
                    None
                }
            })
            .collect();
        Self {
            paseo,
            conductor,
            known_roots,
            git_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn load_from_home(home: &Path) -> Self {
        Self::new(
            PaseoIndex::load_from_home(home),
            ConductorIndex::load_from_home(home),
        )
    }

    pub fn load_default() -> Self {
        match home_dir() {
            Some(h) => Self::load_from_home(&h),
            None => Self::empty(),
        }
    }

    fn git_main_repo(&self, worktree_root: &str) -> Option<String> {
        if let Some(cached) = self.git_cache.borrow().get(worktree_root) {
            return cached.clone();
        }
        let r = read_gitdir_main_repo(Path::new(worktree_root));
        self.git_cache
            .borrow_mut()
            .insert(worktree_root.to_owned(), r.clone());
        r
    }

    fn resolve_paseo(&self, worktree_root: &str, cwd: &str) -> Option<String> {
        self.paseo
            .lookup(worktree_root, cwd)
            .or_else(|| self.git_main_repo(worktree_root))
    }

    fn resolve_conductor(&self, repo: &str, ws_root: Option<&str>) -> Option<String> {
        ws_root
            .and_then(|r| self.conductor.workspace_main(r))
            .map(str::to_owned)
            .or_else(|| self.conductor.repo_main(repo).map(str::to_owned))
            .or_else(|| ws_root.and_then(|r| self.git_main_repo(r)))
            .or_else(|| self.known_roots.get(repo).cloned())
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
/// 4. Conductor workspace (`conductor/workspaces/<repo>[/<ws>...]`): resolved
///    to the main repo via Conductor's DB, the `<repo>` map, the workspace's
///    `.git` file, then a unique known-root basename; otherwise `<repo>`.
/// 5. Otherwise → last 1–2 significant segments.
///
/// Examples:
/// - `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal` → `freelance/tokenwatch` (with index)
/// - `/Users/x/conductor/workspaces/tub2/chengdu-v4` → `inventures/tub2` (with
///   index) or `tub2` (unresolved)
/// - `/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd` → `unknown`
/// - `/Users/x/dev/inventures/tub2` → `inventures/tub2`
pub fn derive_project_name(cwd: Option<&str>, idx: &WorktreeIndex) -> String {
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
            return match idx.resolve_paseo(&worktree_root, &cwd_norm) {
                Some(main) => derive_non_paseo_name(&main),
                None => slug.to_owned(),
            };
        }
    }

    if is_agent_worktree(&components) {
        return "unknown".to_owned();
    }

    for i in 0..components.len().saturating_sub(1) {
        if components[i] == "conductor" && components[i + 1] == "workspaces" {
            let repo = match components.get(i + 2) {
                Some(r) => *r,
                None => return "unknown".to_owned(),
            };
            let ws_root =
                (components.len() > i + 3).then(|| format!("/{}", components[..=i + 3].join("/")));
            return match idx.resolve_conductor(repo, ws_root.as_deref()) {
                Some(main) => derive_non_paseo_name(&main),
                None => repo.to_owned(),
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
    let idx = WorktreeIndex::load_default();
    ingest_from(conn, &base, &idx)
}

/// Run a full incremental ingest from an arbitrary base directory.
///
/// Designed so tests can point at a fixture directory without touching the
/// real `~/.claude/projects`.
pub fn ingest_from(
    conn: &Connection,
    base_dir: &Path,
    idx: &WorktreeIndex,
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
        match ingest_file(conn, &path, idx) {
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
fn ingest_file(conn: &Connection, path: &Path, idx: &WorktreeIndex) -> anyhow::Result<(bool, u64)> {
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
        let project_name = derive_project_name(record.cwd.as_deref(), idx);

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
    fn test_derive_conductor_unresolved_workspace_root_falls_back_to_repo() {
        // Unresolved (empty index, path not on disk): falls back to <repo>.
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/tub2/chengdu-v4"),
                &WorktreeIndex::empty()
            ),
            "tub2"
        );
    }

    #[test]
    fn test_derive_conductor_unresolved_city_with_version_suffix() {
        // Unresolved: .../argus/belo-horizonte-v1 → argus
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/argus/belo-horizonte-v1"),
                &WorktreeIndex::empty()
            ),
            "argus"
        );
    }

    #[test]
    fn test_derive_conductor_unresolved_submodule_deep_path() {
        // Unresolved: .../tub2/dili/FRONT/e2e/operator-assignment → tub2
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/tub2/dili/FRONT/e2e/operator-assignment"),
                &WorktreeIndex::empty()
            ),
            "tub2"
        );
    }

    #[test]
    fn test_derive_conductor_unresolved_monorepo_subpath() {
        // Unresolved: .../argus/cairo/packages/app/src-tauri → argus
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/argus/cairo/packages/app/src-tauri"),
                &WorktreeIndex::empty()
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
                &WorktreeIndex::empty()
            ),
            "unknown"
        );
        assert_eq!(
            derive_project_name(
                Some("/Users/x/worktrees/agent-1234/packages/app"),
                &WorktreeIndex::empty()
            ),
            "unknown"
        );
    }

    const PASEO_JSON: &str = r#"[
      {"workspaceId":"wks_1","kind":"worktree","cwd":"/Users/x/.paseo/worktrees/3qkyl933/resolute-seal","worktreeRoot":"/Users/x/.paseo/worktrees/3qkyl933/resolute-seal","mainRepoRoot":"/Users/x/dev/freelance/tokenwatch","archivedAt":null},
      {"workspaceId":"wks_2","kind":"worktree","cwd":"/Users/x/.paseo/worktrees/abcd1234/upbeat-bear","worktreeRoot":"/Users/x/.paseo/worktrees/abcd1234/upbeat-bear","mainRepoRoot":"/Users/x/dev/meki/backend","archivedAt":"2026-10-01T00:00:00Z","foo":1},
      {"workspaceId":"wks_3","kind":"local_checkout","cwd":"/Users/x/dev/meki/console","worktreeRoot":"/Users/x/dev/meki/console","mainRepoRoot":null}
    ]"#;

    fn paseo_idx() -> WorktreeIndex {
        WorktreeIndex::new(
            PaseoIndex::from_json_str(PASEO_JSON),
            ConductorIndex::empty(),
        )
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
        let idx = WorktreeIndex::new(
            PaseoIndex::from_json_str(
                r#"[{"worktreeRoot":"/h/.paseo/worktrees/zz/slug/","mainRepoRoot":"/h/dev/a/b/"}]"#,
            ),
            ConductorIndex::empty(),
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
        let idx = WorktreeIndex::empty();
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
        let idx = WorktreeIndex::empty();
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
        let idx = WorktreeIndex::empty();
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
            derive_project_name(Some(root.to_str().unwrap()), &WorktreeIndex::empty()),
            "brave-otter"
        );

        let tmp2 = TempDir::new().unwrap();
        let root2 = make_git_worktree(&tmp2, None);
        std::fs::create_dir(root2.join(".git")).unwrap();
        assert_eq!(
            derive_project_name(Some(root2.to_str().unwrap()), &WorktreeIndex::empty()),
            "brave-otter"
        );
    }

    #[test]
    fn test_derive_claude_agent_worktrees_still_unknown() {
        // Agent rule runs before Conductor resolution, even when `tub2` resolves.
        let idx = conductor_idx();
        for p in [
            "/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd",
            "/Users/x/conductor/workspaces/tub2/kelowna-v1/FRONT/.claude/worktrees/agent-ad31",
            "/Users/x/worktrees/agent-1234/packages/app",
        ] {
            assert_eq!(derive_project_name(Some(p), &idx), "unknown", "{p}");
        }
    }

    fn conductor_only(ws: &[(&str, &str)], repos: &[(&str, &str)]) -> WorktreeIndex {
        let c = ConductorIndex::from_entries(
            ws.iter().map(|(a, b)| (a.to_string(), b.to_string())),
            repos.iter().map(|(a, b)| (a.to_string(), b.to_string())),
        );
        WorktreeIndex::new(PaseoIndex::empty(), c)
    }

    fn conductor_idx() -> WorktreeIndex {
        conductor_only(
            &[
                (
                    "/Users/x/conductor/workspaces/tub2/chengdu-v4",
                    "/Users/x/dev/inventures/tub2",
                ),
                (
                    "/Users/x/conductor/workspaces/backend/madrid",
                    "/Users/x/dev/meki/backend/",
                ),
            ],
            &[
                ("tub2", "/Users/x/dev/inventures/tub2"),
                ("backend", "/Users/x/dev/meki/backend"),
                ("argus", "/Users/x/dev/freelance/argus"),
            ],
        )
    }

    fn make_conductor_db(path: &Path, wal: bool) {
        let conn = Connection::open(path).unwrap();
        if wal {
            conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))
                .unwrap();
        }
        conn.execute_batch(
            "CREATE TABLE repos (id TEXT PRIMARY KEY, name TEXT, root_path TEXT);
             CREATE TABLE workspaces (local_id TEXT PRIMARY KEY, repository_id TEXT, workspace_path TEXT, state TEXT);
             INSERT INTO repos VALUES ('r1','backend','/Users/x/dev/meki/backend'),('r2','ghost',NULL);
             INSERT INTO workspaces VALUES
               ('w1','r1','/Users/x/conductor/workspaces/backend/brasilia','ready'),
               ('w2','r1',NULL,'archived'),
               ('w3','r2','/Users/x/conductor/workspaces/ghost/x','archived');",
        )
        .unwrap();
        drop(conn);
    }

    #[test]
    fn test_derive_conductor_exact_workspace_uses_main_repo() {
        let idx = conductor_idx();
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/tub2/chengdu-v4"), &idx),
            "inventures/tub2"
        );
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/backend/madrid"), &idx),
            "meki/backend"
        );
    }

    #[test]
    fn test_derive_conductor_subpath_uses_main_repo() {
        let idx = conductor_idx();
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/tub2/dili/FRONT/e2e/operator-assignment"),
                &idx
            ),
            "inventures/tub2"
        );
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/argus/cairo/packages/app/src-tauri"),
                &idx
            ),
            "freelance/argus"
        );
    }

    #[test]
    fn test_derive_conductor_archived_workspace_uses_repo_map() {
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/backend/lima-v9"),
                &conductor_idx()
            ),
            "meki/backend"
        );
    }

    #[test]
    fn test_derive_conductor_repo_only_path() {
        let idx = conductor_idx();
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/backend"), &idx),
            "meki/backend"
        );
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces"), &idx),
            "unknown"
        );
    }

    #[test]
    fn test_derive_conductor_unknown_repo_falls_back() {
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/clarus/oslo"),
                &conductor_idx()
            ),
            "clarus"
        );
    }

    #[test]
    fn test_derive_conductor_git_file_fallback() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("conductor/workspaces/argus/cairo");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join(".git"),
            "gitdir: /Users/x/dev/freelance/argus/.git/worktrees/cairo\n",
        )
        .unwrap();
        let idx = WorktreeIndex::empty();
        assert_eq!(
            derive_project_name(Some(root.to_str().unwrap()), &idx),
            "freelance/argus"
        );
        assert_eq!(
            derive_project_name(Some(root.join("src").to_str().unwrap()), &idx),
            "freelance/argus"
        );
    }

    #[test]
    fn test_derive_conductor_known_root_basename_fallback() {
        assert_eq!(
            derive_project_name(
                Some("/nonexistent-tw-test/conductor/workspaces/tokenwatch/oslo"),
                &paseo_idx()
            ),
            "freelance/tokenwatch"
        );
    }

    #[test]
    fn test_derive_conductor_ambiguous_basename_not_resolved() {
        let idx = WorktreeIndex::new(
            PaseoIndex::from_json_str(
                r#"[{"worktreeRoot":"/p/.paseo/worktrees/a/s1","mainRepoRoot":"/a/x/backend"},{"worktreeRoot":"/p/.paseo/worktrees/b/s2","mainRepoRoot":"/b/y/backend"}]"#,
            ),
            ConductorIndex::empty(),
        );
        assert_eq!(
            derive_project_name(
                Some("/nonexistent-tw-test/conductor/workspaces/backend/oslo"),
                &idx
            ),
            "backend"
        );
    }

    #[test]
    fn test_conductor_index_ambiguous_repo_segment_dropped() {
        let idx = conductor_only(&[], &[("api", "/a/one/api"), ("api", "/b/two/api")]);
        assert_eq!(
            derive_project_name(
                Some("/nonexistent-tw-test/conductor/workspaces/api/x"),
                &idx
            ),
            "api"
        );
    }

    #[test]
    fn test_conductor_index_blank_entries_ignored() {
        let idx = conductor_only(
            &[
                ("", "/a/b"),
                ("/nonexistent-tw-test/conductor/workspaces/r/w", " "),
            ],
            &[("", "/x/y")],
        );
        assert_eq!(
            derive_project_name(Some("/nonexistent-tw-test/conductor/workspaces/r/w"), &idx),
            "r"
        );
    }

    #[test]
    fn test_ingest_conductor_cwd_attributed_to_main_repo() {
        let tmp = TempDir::new().unwrap();
        make_fixture_dir(&tmp, "proj/session.jsonl", &format!("{FIXTURE_LINE_1}\n"));
        let conn = test_conn();
        let summary = ingest_from(&conn, tmp.path(), &conductor_idx()).unwrap();
        assert_eq!(summary.events_inserted, 1);
        let (name, path): (String, String) = conn
            .query_row(
                "SELECT project_name, project_path FROM usage_events",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "meki/backend");
        assert_eq!(path, "/Users/x/conductor/workspaces/backend/madrid");
    }

    #[test]
    fn test_conductor_index_load_from_db() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("conductor.db");
        make_conductor_db(&p, false);
        let idx = WorktreeIndex::new(PaseoIndex::empty(), ConductorIndex::load_from_db(&p));
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/backend/brasilia"), &idx),
            "meki/backend"
        );
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/backend/other"), &idx),
            "meki/backend"
        );
        assert_eq!(
            derive_project_name(
                Some("/nonexistent-tw-test/conductor/workspaces/ghost/x"),
                &idx
            ),
            "ghost"
        );
    }

    #[test]
    fn test_conductor_index_load_from_wal_db_without_sidecars() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("conductor.db");
        make_conductor_db(&p, true);
        assert!(
            !tmp.path().join("conductor.db-wal").exists(),
            "precondition: no -wal sidecar after the last connection closed"
        );
        let idx = WorktreeIndex::new(PaseoIndex::empty(), ConductorIndex::load_from_db(&p));
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/backend/brasilia"), &idx),
            "meki/backend"
        );
    }

    #[test]
    fn test_conductor_index_load_is_read_only() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("conductor.db");
        make_conductor_db(&p, false);
        let before = std::fs::read(&p).unwrap();
        let _ = ConductorIndex::load_from_db(&p);
        let after = std::fs::read(&p).unwrap();
        assert_eq!(before, after);
        for suffix in ["-wal", "-shm", "-journal"] {
            assert!(!tmp.path().join(format!("conductor.db{suffix}")).exists());
        }
    }

    #[test]
    fn test_conductor_index_missing_or_invalid_db_is_empty() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("missing.db");
        let garbage = tmp.path().join("garbage.db");
        std::fs::write(&garbage, b"\0garbage").unwrap();
        let other = tmp.path().join("other.db");
        {
            let c = Connection::open(&other).unwrap();
            c.execute_batch("CREATE TABLE t(x)").unwrap();
        }
        for p in [missing, garbage, other] {
            let idx = WorktreeIndex::new(PaseoIndex::empty(), ConductorIndex::load_from_db(&p));
            assert_eq!(
                derive_project_name(Some("/Users/x/conductor/workspaces/backend/brasilia"), &idx),
                "backend",
                "{}",
                p.display()
            );
        }
    }

    #[test]
    fn test_worktree_index_load_from_home() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let paseo_dir = home.join(".paseo/projects");
        std::fs::create_dir_all(&paseo_dir).unwrap();
        std::fs::write(paseo_dir.join("workspaces.json"), PASEO_JSON).unwrap();
        let cdir = home.join("Library/Application Support/com.conductor.app");
        std::fs::create_dir_all(&cdir).unwrap();
        make_conductor_db(&cdir.join("conductor.db"), false);

        let idx = WorktreeIndex::load_from_home(home);
        assert_eq!(
            derive_project_name(
                Some("/Users/x/.paseo/worktrees/3qkyl933/resolute-seal"),
                &idx
            ),
            "freelance/tokenwatch"
        );
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/backend/brasilia"), &idx),
            "meki/backend"
        );

        let empty = TempDir::new().unwrap();
        let idx = WorktreeIndex::load_from_home(empty.path());
        assert_eq!(
            derive_project_name(
                Some("/Users/x/.paseo/worktrees/3qkyl933/resolute-seal"),
                &idx
            ),
            "resolute-seal"
        );
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/backend/brasilia"), &idx),
            "backend"
        );
    }

    #[test]
    fn test_derive_generic_worktrees_component_uses_fallback() {
        assert_eq!(
            derive_project_name(
                Some("/Users/x/dev/worktrees/feature-x"),
                &WorktreeIndex::empty()
            ),
            "worktrees/feature-x"
        );
    }

    #[test]
    fn test_paseo_index_malformed_json_is_empty() {
        for json in ["{not json", "{\"a\":1}", "[{\"mainRepoRoot\": 5}]"] {
            let idx = WorktreeIndex::new(PaseoIndex::from_json_str(json), ConductorIndex::empty());
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
        let idx = WorktreeIndex::new(
            PaseoIndex::load_from_home(tmp.path()),
            ConductorIndex::empty(),
        );
        assert_eq!(derive_project_name(Some(cwd), &idx), "resolute-seal");

        let dir = tmp.path().join(".paseo/projects");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("workspaces.json"), PASEO_JSON).unwrap();
        let idx = WorktreeIndex::new(
            PaseoIndex::load_from_home(tmp.path()),
            ConductorIndex::empty(),
        );
        assert_eq!(derive_project_name(Some(cwd), &idx), "freelance/tokenwatch");

        std::fs::write(dir.join("workspaces.json"), "\u{0}garbage").unwrap();
        let idx = WorktreeIndex::new(
            PaseoIndex::load_from_home(tmp.path()),
            ConductorIndex::empty(),
        );
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
            derive_project_name(
                Some("/Users/x/dev/inventures/tub2"),
                &WorktreeIndex::empty()
            ),
            "inventures/tub2"
        );
    }

    // A Conductor path resolves to the repo, even at two segments after ws.
    #[test]
    fn test_derive_conductor_unresolved_two_segments() {
        // Unresolved Conductor workspace falls back to the repo segment.
        assert_eq!(
            derive_project_name(
                Some("/Users/x/conductor/workspaces/backend/madrid"),
                &WorktreeIndex::empty()
            ),
            "backend"
        );
    }

    #[test]
    fn test_derive_project_name_one_segment() {
        // "/project" has only one meaningful segment after stripping the root → "project"
        assert_eq!(
            derive_project_name(Some("/project"), &WorktreeIndex::empty()),
            "project"
        );
    }

    #[test]
    fn test_derive_project_name_none() {
        assert_eq!(
            derive_project_name(None, &WorktreeIndex::empty()),
            "unknown"
        );
    }

    #[test]
    fn test_derive_project_name_empty() {
        assert_eq!(
            derive_project_name(Some(""), &WorktreeIndex::empty()),
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
        let summary = ingest_from(&conn, tmp.path(), &WorktreeIndex::empty()).unwrap();

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
        // Unresolved Conductor workspace falls back to the repo segment.
        assert_eq!(project_name, "backend");
    }

    #[test]
    fn test_ingest_skips_synthetic_model() {
        let tmp = TempDir::new().unwrap();
        let content = format!("{FIXTURE_LINE_1}\n{FIXTURE_SYNTHETIC}\n");
        make_fixture_dir(&tmp, "proj/session.jsonl", &content);

        let conn = test_conn();
        let summary = ingest_from(&conn, tmp.path(), &WorktreeIndex::empty()).unwrap();

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
        ingest_from(&conn, tmp.path(), &WorktreeIndex::empty()).unwrap();

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
        ingest_from(&conn, tmp.path(), &WorktreeIndex::empty()).unwrap();

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
        let s1 = ingest_from(&conn, tmp.path(), &WorktreeIndex::empty()).unwrap();
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

        let s2 = ingest_from(&conn, tmp.path(), &WorktreeIndex::empty()).unwrap();
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
        let s1 = ingest_from(&conn, tmp.path(), &WorktreeIndex::empty()).unwrap();
        assert_eq!(s1.files_skipped, 0);
        assert_eq!(s1.files_processed, 1);

        // Second run — file unchanged.
        let s2 = ingest_from(&conn, tmp.path(), &WorktreeIndex::empty()).unwrap();
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
        let summary = ingest_from(&conn, &fixture_dir, &WorktreeIndex::empty()).unwrap();

        // The fixture has 5 valid assistant lines (lines 1,3,4,5,6) — line 6 has unknown model
        // but still inserts (cost=0). Malformed JSON line and user line are skipped.
        assert!(
            summary.events_inserted >= 4,
            "expected at least 4 events from fixture, got {}",
            summary.events_inserted
        );
    }
}
