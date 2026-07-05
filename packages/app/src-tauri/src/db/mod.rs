//! SQLite persistence layer for TokenWatch usage events.
//!
//! The database lives in the OS app-data directory as `tokenwatch.db`.
//! Schema migrations are keyed by `schema_version` stored in the `meta` table;
//! new versions apply their DDL and bump the version number.

use rusqlite::{params, Connection, Result as SqlResult};

use crate::config::app_identity::DB_FILENAME;

/// Open the production database in the OS app-data directory.
///
/// Uses `directories::ProjectDirs` (bundle id `com.tokenwatch.app`) to locate
/// the data dir, creates it if needed, then opens/migrates the database.
pub fn open_default() -> SqlResult<Connection> {
    let data_dir = directories::ProjectDirs::from("com", "tokenwatch", "TokenWatch")
        .expect("cannot determine app data directory")
        .data_dir()
        .to_path_buf();

    std::fs::create_dir_all(&data_dir)
        .map_err(|_| rusqlite::Error::InvalidPath(data_dir.clone()))?;

    let db_path = data_dir.join(DB_FILENAME);
    open_at(&db_path.to_string_lossy())
}

/// Open (or create) a database at an explicit path. Pass `":memory:"` for tests.
pub fn open_at(path: &str) -> SqlResult<Connection> {
    let conn = Connection::open(path)?;
    // Enable WAL for better concurrent read/write performance.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

// ---------------------------------------------------------------------------
// Migrations
// ---------------------------------------------------------------------------

fn migrate(conn: &Connection) -> SqlResult<()> {
    // Ensure meta table exists before reading schema_version.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;

    let version: u32 = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v.parse().unwrap_or(0))
        .unwrap_or(0);

    if version < 1 {
        apply_v1(conn)?;
        set_meta(conn, "schema_version", "1")?;
    }

    if version < 2 {
        apply_v2(conn)?;
        set_meta(conn, "schema_version", "2")?;
    }

    if version < 3 {
        apply_v3(conn)?;
        set_meta(conn, "schema_version", "3")?;
    }

    if version < 4 {
        // D13: write the literal "4", not CURRENT_SCHEMA_VERSION, so a DB at v3
        // does not skip v5 if it is opened again before that migration runs.
        // Note: apply_v4 calls backfill_project_names which runs its own internal
        // transaction, so we must not wrap it in an outer transaction here.
        apply_v4(conn)?;
        set_meta(conn, "schema_version", "4")?;
    }

    if version < 5 {
        // DDL only — safe to wrap in a transaction.
        let tx = conn.unchecked_transaction()?;
        apply_v5(&tx)?;
        set_meta(&tx, "schema_version", "5")?;
        tx.commit()?;
    }

    Ok(())
}

fn apply_v1(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS usage_events (
            dedup_key              TEXT PRIMARY KEY,
            session_id             TEXT NOT NULL,
            project_path           TEXT NOT NULL,
            project_name           TEXT NOT NULL,
            model                  TEXT NOT NULL,
            input_tokens           INTEGER NOT NULL DEFAULT 0,
            output_tokens          INTEGER NOT NULL DEFAULT 0,
            cache_creation_tokens  INTEGER NOT NULL DEFAULT 0,
            cache_read_tokens      INTEGER NOT NULL DEFAULT 0,
            total_tokens           INTEGER NOT NULL DEFAULT 0,
            cost                   REAL    NOT NULL DEFAULT 0,
            timestamp              TEXT NOT NULL,
            git_branch             TEXT,
            ingested_at            TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_ue_timestamp ON usage_events(timestamp);
        CREATE INDEX IF NOT EXISTS idx_ue_project   ON usage_events(project_name);
        CREATE INDEX IF NOT EXISTS idx_ue_model     ON usage_events(model);

        CREATE TABLE IF NOT EXISTS ingest_files (
            path           TEXT PRIMARY KEY,
            size_bytes     INTEGER NOT NULL,
            mtime          INTEGER NOT NULL,
            lines_ingested INTEGER NOT NULL,
            updated_at     TEXT NOT NULL
        );
        ",
    )
}

/// Migration v2 — backfill `project_name` using the Conductor-aware rule.
///
/// Reads every row's `project_path` (the raw cwd already stored) and recomputes
/// `project_name` via `crate::ingest::derive_project_name`.
fn apply_v2(conn: &Connection) -> SqlResult<()> {
    backfill_project_names(conn)
}

/// Migration v3 — re-run the `project_name` backfill to pick up an updated
/// worktree rule.
fn apply_v3(conn: &Connection) -> SqlResult<()> {
    backfill_project_names(conn)
}

/// Migration v4 — re-run the backfill again after correcting the derivation
/// rule: Conductor `workspaces` paths resolve back to the repo (e.g. `tub2`,
/// `argus`), and only ephemeral `worktrees` (`.../worktrees/agent-XXXX`)
/// collapse to `"unknown"`. This repairs DBs where v3 had over-grouped
/// Conductor projects into `"unknown"`.
fn apply_v4(conn: &Connection) -> SqlResult<()> {
    backfill_project_names(conn)
}

/// Migration v5 — create `project_groups` and `project_group_members` tables
/// for the per-group budget feature. Aditiva: no toca `usage_events`.
fn apply_v5(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS project_groups (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT NOT NULL UNIQUE,
            budget_basis TEXT,
            budget_value REAL,
            created_at   TEXT NOT NULL,
            CHECK (budget_basis IN ('share', 'usd') OR budget_basis IS NULL),
            CHECK (budget_basis IS NULL OR (budget_value > 0 AND (budget_basis <> 'share' OR budget_value <= 100)))
        );
        CREATE TABLE IF NOT EXISTS project_group_members (
            project_name TEXT PRIMARY KEY,
            group_id     INTEGER NOT NULL REFERENCES project_groups(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_pgm_group_id ON project_group_members(group_id);
        ",
    )
}

/// Recompute `project_name` for every row from its stored `project_path`,
/// updating in place within a single transaction (atomic). Idempotent: running
/// on already-correct data is a no-op.
///
/// **D19 contract**: any future re-derivation that changes the `project_name`
/// rule MUST also remap `project_group_members.project_name` with the same
/// old→new mapping, or group memberships will silently fall through to "otros".
fn backfill_project_names(conn: &Connection) -> SqlResult<()> {
    // Collect all (dedup_key, project_path) pairs first, then update in a
    // single transaction so the backfill is atomic.
    let mut stmt = conn.prepare("SELECT dedup_key, project_path FROM usage_events")?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<SqlResult<_>>()?;

    let tx = conn.unchecked_transaction()?;
    for (dedup_key, project_path) in rows {
        let new_name = crate::ingest::derive_project_name(Some(&project_path));
        tx.execute(
            "UPDATE usage_events SET project_name = ?1 WHERE dedup_key = ?2",
            params![new_name, dedup_key],
        )?;
    }
    tx.commit()?;

    Ok(())
}

fn set_meta(conn: &Connection, key: &str, value: &str) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Public write helpers
// ---------------------------------------------------------------------------

/// Row to insert into `usage_events`. All fields required.
#[derive(Debug)]
pub struct UsageEventRow<'a> {
    pub dedup_key: &'a str,
    pub session_id: &'a str,
    pub project_path: &'a str,
    pub project_name: &'a str,
    pub model: &'a str,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_tokens: u64,
    pub cost: f64,
    pub timestamp: &'a str,
    pub git_branch: Option<&'a str>,
    pub ingested_at: &'a str,
}

/// Insert a usage event. Silently ignores duplicates (idempotent).
pub fn insert_event(conn: &Connection, row: &UsageEventRow<'_>) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO usage_events (
            dedup_key, session_id, project_path, project_name, model,
            input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens,
            total_tokens, cost, timestamp, git_branch, ingested_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
         ON CONFLICT(dedup_key) DO NOTHING",
        params![
            row.dedup_key,
            row.session_id,
            row.project_path,
            row.project_name,
            row.model,
            row.input_tokens,
            row.output_tokens,
            row.cache_creation_tokens,
            row.cache_read_tokens,
            row.total_tokens,
            row.cost,
            row.timestamp,
            row.git_branch,
            row.ingested_at,
        ],
    )?;
    Ok(())
}

/// Insert / update ingest file progress.
pub fn upsert_ingest_file(
    conn: &Connection,
    path: &str,
    size_bytes: u64,
    mtime: i64,
    lines_ingested: u64,
    updated_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO ingest_files(path, size_bytes, mtime, lines_ingested, updated_at)
         VALUES(?1,?2,?3,?4,?5)
         ON CONFLICT(path) DO UPDATE SET
             size_bytes     = excluded.size_bytes,
             mtime          = excluded.mtime,
             lines_ingested = excluded.lines_ingested,
             updated_at     = excluded.updated_at",
        params![path, size_bytes, mtime, lines_ingested, updated_at],
    )?;
    Ok(())
}

/// Read ingest file progress for a given path. Returns `None` if not seen yet.
pub fn get_ingest_file(conn: &Connection, path: &str) -> SqlResult<Option<(u64, i64, u64)>> {
    // Returns (size_bytes, mtime, lines_ingested)
    let result = conn.query_row(
        "SELECT size_bytes, mtime, lines_ingested FROM ingest_files WHERE path = ?1",
        params![path],
        |row| {
            Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, u64>(2)?,
            ))
        },
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Read a generic meta value by key. Returns `None` if the key does not exist.
pub fn meta_get(conn: &Connection, key: &str) -> SqlResult<Option<String>> {
    let result = conn.query_row(
        "SELECT value FROM meta WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Write (upsert) a generic meta key/value pair.
pub fn meta_set(conn: &Connection, key: &str, value: &str) -> SqlResult<()> {
    set_meta(conn, key, value)
}

/// Update the `last_refresh_at` meta entry.
pub fn set_last_refresh(conn: &Connection, ts: &str) -> SqlResult<()> {
    set_meta(conn, "last_refresh_at", ts)
}

/// Count total events in the database.
pub fn count_events(conn: &Connection) -> SqlResult<u64> {
    conn.query_row("SELECT COUNT(*) FROM usage_events", [], |row| {
        row.get::<_, u64>(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        open_at(":memory:").expect("in-memory db failed")
    }

    #[test]
    fn test_schema_creation() {
        let conn = test_conn();
        // Verify tables exist by querying them.
        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM ingest_files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        let version: String = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(version, "5");
    }

    #[test]
    fn test_idempotent_insert_same_dedup_key() {
        let conn = test_conn();
        let row = UsageEventRow {
            dedup_key: "msg_abc:req_xyz",
            session_id: "sess1",
            project_path: "/home/user/project",
            project_name: "project",
            model: "claude-opus-4-8",
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 150,
            cost: 0.0015,
            timestamp: "2026-07-01T12:00:00Z",
            git_branch: Some("main"),
            ingested_at: "2026-07-01T12:01:00Z",
        };
        insert_event(&conn, &row).unwrap();
        insert_event(&conn, &row).unwrap(); // second insert should be ignored

        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "duplicate dedup_key must not create a second row");
    }

    #[test]
    fn test_insert_and_read_event() {
        let conn = test_conn();
        let row = UsageEventRow {
            dedup_key: "msg_001:req_001",
            session_id: "sess_a",
            project_path: "/users/x/projects/alpha",
            project_name: "projects/alpha",
            model: "claude-sonnet-4-6",
            input_tokens: 200,
            output_tokens: 30,
            cache_creation_tokens: 10,
            cache_read_tokens: 5,
            total_tokens: 245,
            cost: 0.001,
            timestamp: "2026-07-02T08:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-02T08:01:00Z",
        };
        insert_event(&conn, &row).unwrap();

        let (model, total): (String, u64) = conn
            .query_row(
                "SELECT model, total_tokens FROM usage_events WHERE dedup_key = 'msg_001:req_001'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();

        assert_eq!(model, "claude-sonnet-4-6");
        assert_eq!(total, 245);
    }

    #[test]
    fn test_ingest_file_upsert_and_get() {
        let conn = test_conn();
        let path = "/home/user/.claude/projects/proj/sess.jsonl";

        // First write
        upsert_ingest_file(&conn, path, 1024, 1_700_000_000, 10, "2026-07-01T00:00:00Z").unwrap();
        let (size, mtime, lines) = get_ingest_file(&conn, path).unwrap().unwrap();
        assert_eq!(size, 1024);
        assert_eq!(mtime, 1_700_000_000);
        assert_eq!(lines, 10);

        // Update (file grew)
        upsert_ingest_file(&conn, path, 2048, 1_700_001_000, 20, "2026-07-01T01:00:00Z").unwrap();
        let (size2, _, lines2) = get_ingest_file(&conn, path).unwrap().unwrap();
        assert_eq!(size2, 2048);
        assert_eq!(lines2, 20);
    }

    #[test]
    fn test_get_ingest_file_missing() {
        let conn = test_conn();
        let result = get_ingest_file(&conn, "/nonexistent/path").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_count_events() {
        let conn = test_conn();
        assert_eq!(count_events(&conn).unwrap(), 0);

        for i in 0..5u64 {
            let key = format!("msg_{i}:req_{i}");
            let row = UsageEventRow {
                dedup_key: &key,
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-haiku-4-5",
                input_tokens: i,
                output_tokens: i,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: i * 2,
                cost: 0.0,
                timestamp: "2026-07-01T00:00:00Z",
                git_branch: None,
                ingested_at: "2026-07-01T00:00:01Z",
            };
            insert_event(&conn, &row).unwrap();
        }
        assert_eq!(count_events(&conn).unwrap(), 5);
    }

    // -----------------------------------------------------------------------
    // Migration v2 — backfill project_name tests
    // -----------------------------------------------------------------------

    /// Simulate a row inserted at schema v1 (old rule: last 2 segments) whose
    /// project_path is a Conductor workspace path. After running apply_v2 the
    /// project_name must be the repo component only; project_path must be
    /// unchanged. Running the migration a second time must not change anything.
    #[test]
    fn test_v2_backfill_corrects_conductor_project_names() {
        // Open a fresh in-memory DB (goes through the full migration to v2).
        let conn = test_conn();

        // Insert a row mimicking old v1 behaviour: project_name is the last
        // two segments ("tub2/chengdu-v4") but project_path is the real cwd.
        let conductor_path = "/Users/x/conductor/workspaces/tub2/chengdu-v4";
        let row = UsageEventRow {
            dedup_key: "msg_backfill:req_backfill",
            session_id: "sess_bf",
            project_path: conductor_path,
            project_name: "tub2/chengdu-v4", // old (wrong) value
            model: "claude-sonnet-4-6",
            input_tokens: 100,
            output_tokens: 20,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 120,
            cost: 0.0,
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        // Run apply_v2 directly to simulate the migration on the seeded row.
        apply_v2(&conn).unwrap();

        let (project_name, project_path): (String, String) = conn
            .query_row(
                "SELECT project_name, project_path FROM usage_events \
                 WHERE dedup_key = 'msg_backfill:req_backfill'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();

        assert_eq!(
            project_name, "tub2",
            "conductor project_name must be corrected to the repo"
        );
        assert_eq!(
            project_path, conductor_path,
            "project_path (raw cwd) must be unchanged"
        );

        // Run apply_v2 a second time — must be idempotent (no error, no change).
        apply_v2(&conn).unwrap();

        let project_name_after: String = conn
            .query_row(
                "SELECT project_name FROM usage_events \
                 WHERE dedup_key = 'msg_backfill:req_backfill'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            project_name_after, "tub2",
            "second run of apply_v2 must not change project_name"
        );
    }

    /// Non-Conductor rows must keep correct names after backfill.
    #[test]
    fn test_v2_backfill_preserves_non_conductor_names() {
        let conn = test_conn();

        let row = UsageEventRow {
            dedup_key: "msg_nc:req_nc",
            session_id: "sess_nc",
            project_path: "/Users/x/dev/inventures/tub2",
            project_name: "inventures/tub2",
            model: "claude-haiku-4-5",
            input_tokens: 50,
            output_tokens: 10,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 60,
            cost: 0.0,
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        apply_v2(&conn).unwrap();

        let project_name: String = conn
            .query_row(
                "SELECT project_name FROM usage_events WHERE dedup_key = 'msg_nc:req_nc'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        // Non-Conductor fallback: last 2 segments → inventures/tub2
        assert_eq!(project_name, "inventures/tub2");
    }

    /// v3 re-runs the backfill so a stale "worktrees/agent-XXXX" name (produced
    /// by an older rule) is corrected to "unknown".
    #[test]
    fn test_v3_backfill_corrects_worktree_names() {
        let conn = test_conn();

        let worktree_path = "/Users/x/dev/tub2/worktrees/agent-1234";
        let row = UsageEventRow {
            dedup_key: "msg_wt:req_wt",
            session_id: "sess_wt",
            project_path: worktree_path,
            project_name: "worktrees/agent-1234", // old (wrong) value
            model: "claude-sonnet-4-6",
            input_tokens: 100,
            output_tokens: 20,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 120,
            cost: 0.0,
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        apply_v3(&conn).unwrap();

        let project_name: String = conn
            .query_row(
                "SELECT project_name FROM usage_events WHERE dedup_key = 'msg_wt:req_wt'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            project_name, "unknown",
            "worktree project_name must be corrected to \"unknown\""
        );
    }
}
