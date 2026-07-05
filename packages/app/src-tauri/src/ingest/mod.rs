//! Incremental JSONL ingestion for Claude usage logs.
//!
//! Walks `~/.claude/projects/**/*.jsonl`, parses only `type:"assistant"` lines
//! that contain `message.usage`, deduplicates by `"{message.id}:{requestId}"`,
//! and stores events in the SQLite database.
//!
//! Incremental state is tracked in the `ingest_files` table so only new lines
//! (appended since last run) are re-parsed on subsequent polls.

use std::{
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

/// Derive a human-readable project name from a `cwd` path.
///
/// Rules (in order):
/// 1. `cwd` is `None` or empty → `"unknown"`.
/// 2. If the path contains any `worktrees` component (e.g. an ephemeral
///    `.../.claude/worktrees/agent-XXXX`), it can't be attributed to a project
///    → `"unknown"`, so all such worktrees are grouped together.
/// 3. If the path contains the consecutive pair `conductor` → `workspaces`,
///    return the component right after `workspaces` (the `<repo>`), collapsing
///    workspace city, subpaths, and submodules into the repo name.
/// 4. Otherwise → last 1–2 significant segments.
///
/// Examples:
/// - `/Users/x/conductor/workspaces/tub2/chengdu-v4` → `tub2`
/// - `/Users/x/conductor/workspaces/argus/cairo/packages/app` → `argus`
/// - `/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd` → `unknown`
/// - `/Users/x/dev/inventures/tub2` → `inventures/tub2`
pub fn derive_project_name(cwd: Option<&str>) -> String {
    let cwd = match cwd {
        Some(c) if !c.is_empty() => c,
        _ => return "unknown".to_owned(),
    };

    let path = Path::new(cwd);
    // Filter out the root component ("/") so paths like "/project" yield "project".
    let components: Vec<&str> = path
        .components()
        .filter(|c| {
            !matches!(
                c,
                std::path::Component::RootDir | std::path::Component::Prefix(_)
            )
        })
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if components.is_empty() {
        return "unknown".to_owned();
    }

    // Ephemeral agent worktrees (any "worktrees" component, e.g.
    // .../.claude/worktrees/agent-XXXX) → group them all under "unknown".
    if components.contains(&"worktrees") {
        return "unknown".to_owned();
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
    let home = std::env::var("HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| directories::UserDirs::new().map(|d| d.home_dir().to_path_buf()))?;
    Some(home.join(".claude").join("projects"))
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
    ingest_from(conn, &base)
}

/// Run a full incremental ingest from an arbitrary base directory.
///
/// Designed so tests can point at a fixture directory without touching the
/// real `~/.claude/projects`.
pub fn ingest_from(conn: &Connection, base_dir: &Path) -> anyhow::Result<IngestSummary> {
    let pattern = base_dir.join("**").join("*.jsonl");
    let pattern_str = pattern.to_string_lossy();

    let mut summary = IngestSummary::default();

    let paths: Vec<_> = glob::glob(&pattern_str)
        .map_err(|e| anyhow::anyhow!("glob error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    for path in paths {
        let path_display = path.display().to_string();
        match ingest_file(conn, &path) {
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
fn ingest_file(conn: &Connection, path: &Path) -> anyhow::Result<(bool, u64)> {
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
        let project_name = derive_project_name(record.cwd.as_deref());

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
            derive_project_name(Some("/Users/x/conductor/workspaces/tub2/chengdu-v4")),
            "tub2"
        );
    }

    #[test]
    fn test_derive_conductor_city_with_version_suffix() {
        // /Users/x/conductor/workspaces/argus/belo-horizonte-v1 → argus
        assert_eq!(
            derive_project_name(Some(
                "/Users/x/conductor/workspaces/argus/belo-horizonte-v1"
            )),
            "argus"
        );
    }

    #[test]
    fn test_derive_conductor_submodule_deep_path() {
        // .../tub2/dili/FRONT/e2e/operator-assignment → tub2
        assert_eq!(
            derive_project_name(Some(
                "/Users/x/conductor/workspaces/tub2/dili/FRONT/e2e/operator-assignment"
            )),
            "tub2"
        );
    }

    #[test]
    fn test_derive_conductor_monorepo_subpath() {
        // .../argus/cairo/packages/app/src-tauri → argus
        assert_eq!(
            derive_project_name(Some(
                "/Users/x/conductor/workspaces/argus/cairo/packages/app/src-tauri"
            )),
            "argus"
        );
    }

    #[test]
    fn test_derive_worktrees_component() {
        // Any path with a "worktrees" component → grouped under "unknown"
        assert_eq!(
            derive_project_name(Some(
                "/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd"
            )),
            "unknown"
        );
        assert_eq!(
            derive_project_name(Some("/Users/x/worktrees/agent-1234/packages/app")),
            "unknown"
        );
    }

    #[test]
    fn test_derive_non_conductor_path_keeps_fallback() {
        // /Users/x/dev/inventures/tub2 → inventures/tub2
        assert_eq!(
            derive_project_name(Some("/Users/x/dev/inventures/tub2")),
            "inventures/tub2"
        );
    }

    // A Conductor path resolves to the repo, even at two segments after ws.
    #[test]
    fn test_derive_project_name_two_segments() {
        assert_eq!(
            derive_project_name(Some("/Users/x/conductor/workspaces/backend/madrid")),
            "backend"
        );
    }

    #[test]
    fn test_derive_project_name_one_segment() {
        // "/project" has only one meaningful segment after stripping the root → "project"
        assert_eq!(derive_project_name(Some("/project")), "project");
    }

    #[test]
    fn test_derive_project_name_none() {
        assert_eq!(derive_project_name(None), "unknown");
    }

    #[test]
    fn test_derive_project_name_empty() {
        assert_eq!(derive_project_name(Some("")), "unknown");
    }

    #[test]
    fn test_ingest_fixture_produces_expected_rows() {
        let tmp = TempDir::new().unwrap();
        let content = format!(
            "{FIXTURE_LINE_1}\n{FIXTURE_USER_LINE}\n{FIXTURE_MALFORMED}\n{FIXTURE_NO_USAGE}\n"
        );
        make_fixture_dir(&tmp, "proj/session.jsonl", &content);

        let conn = test_conn();
        let summary = ingest_from(&conn, tmp.path()).unwrap();

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
        let summary = ingest_from(&conn, tmp.path()).unwrap();

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
        ingest_from(&conn, tmp.path()).unwrap();

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
        ingest_from(&conn, tmp.path()).unwrap();

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
        let s1 = ingest_from(&conn, tmp.path()).unwrap();
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

        let s2 = ingest_from(&conn, tmp.path()).unwrap();
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
        let s1 = ingest_from(&conn, tmp.path()).unwrap();
        assert_eq!(s1.files_skipped, 0);
        assert_eq!(s1.files_processed, 1);

        // Second run — file unchanged.
        let s2 = ingest_from(&conn, tmp.path()).unwrap();
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
        let summary = ingest_from(&conn, &fixture_dir).unwrap();

        // The fixture has 5 valid assistant lines (lines 1,3,4,5,6) — line 6 has unknown model
        // but still inserts (cost=0). Malformed JSON line and user line are skipped.
        assert!(
            summary.events_inserted >= 4,
            "expected at least 4 events from fixture, got {}",
            summary.events_inserted
        );
    }
}
