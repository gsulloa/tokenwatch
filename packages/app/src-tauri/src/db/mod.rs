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

    if version < 6 {
        // recompute_costs opens its own transaction, so do NOT wrap here.
        apply_v6(conn)?;
        set_meta(conn, "schema_version", "6")?;
    }

    if version < 7 {
        // recompute_costs opens its own transaction, so do NOT wrap here.
        apply_v7(conn)?;
        set_meta(conn, "schema_version", "7")?;
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

/// Migration v6 — repair pre-fix zero-cost events (e.g. `claude-sonnet-5`) by
/// re-deriving `cost` for every row from its already-stored `model` and token
/// counts, using the current price table (`pricing::cost`). PR #34 fixed
/// family-based price matching for Sonnet 5 (and future major versions), but
/// cost is persisted at ingest time and incremental ingestion never re-parses
/// already-ingested JSONL lines, so events ingested before the fix kept
/// `cost = 0`. Aditiva: no toca `input_tokens`/`output_tokens`/`total_tokens`/
/// `project_name`/`dedup_key`.
fn apply_v6(conn: &Connection) -> SqlResult<()> {
    recompute_costs(conn)
}

/// Migration v7 — repair events persisted at `cost = 0` because their model
/// family (Claude Fable 5 / Claude Mythos 5) was absent from the price table.
/// Reuses the same idempotent recompute as v6.
fn apply_v7(conn: &Connection) -> SqlResult<()> {
    recompute_costs(conn)
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

/// Recompute `cost` for every row from its stored `model` + four token
/// columns, updating in place within a single transaction (atomic).
/// Deterministic and idempotent: every row's cost becomes
/// `pricing::cost(model, usage)`, so re-running on already-correct data
/// leaves it unchanged. Unknown models keep `cost = 0` — this backfill only
/// applies the current price table, it never invents prices.
fn recompute_costs(conn: &Connection) -> SqlResult<()> {
    // Collect all (dedup_key, model, tokens…) first, then update in a single
    // transaction so the backfill is atomic.
    let mut stmt = conn.prepare(
        "SELECT dedup_key, model, input_tokens, output_tokens, \
                cache_creation_tokens, cache_read_tokens \
         FROM usage_events",
    )?;
    let rows: Vec<(String, String, u64, u64, u64, u64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u64>(2)?,
                row.get::<_, u64>(3)?,
                row.get::<_, u64>(4)?,
                row.get::<_, u64>(5)?,
            ))
        })?
        .collect::<SqlResult<_>>()?;

    let tx = conn.unchecked_transaction()?;
    for (dedup_key, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens) in
        rows
    {
        let usage = crate::pricing::Usage {
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
        };
        let new_cost = crate::pricing::cost(&model, &usage);
        tx.execute(
            "UPDATE usage_events SET cost = ?1 WHERE dedup_key = ?2",
            params![new_cost, dedup_key],
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
        assert_eq!(version, "7");
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

    // -----------------------------------------------------------------------
    // Migration v6 — recompute cost tests
    // -----------------------------------------------------------------------

    /// A pre-fix Sonnet 5 row (stored with `cost = 0`) must be repaired to a
    /// non-zero cost matching `pricing::cost` after `apply_v6`.
    #[test]
    fn test_v6_recompute_fixes_zero_cost_sonnet_5() {
        let conn = test_conn();

        let row = UsageEventRow {
            dedup_key: "msg_s5:req_s5",
            session_id: "sess_s5",
            project_path: "/Users/x/dev/tub2",
            project_name: "tub2",
            model: "claude-sonnet-5",
            input_tokens: 1_000,
            output_tokens: 100,
            cache_creation_tokens: 50,
            cache_read_tokens: 20,
            total_tokens: 1_170,
            cost: 0.0, // pre-fix: stored as zero
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        apply_v6(&conn).unwrap();

        let cost: f64 = conn
            .query_row(
                "SELECT cost FROM usage_events WHERE dedup_key = 'msg_s5:req_s5'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        let expected = crate::pricing::cost(
            "claude-sonnet-5",
            &crate::pricing::Usage {
                input_tokens: 1_000,
                output_tokens: 100,
                cache_creation_tokens: 50,
                cache_read_tokens: 20,
            },
        );
        assert!(
            (cost - expected).abs() < 1e-9,
            "got {cost}, want {expected}"
        );
        assert!(cost > 0.0, "sonnet 5 cost must be repaired to non-zero");
    }

    /// Running `apply_v6` a second time must not change an already-recomputed
    /// cost — the recompute is a pure, deterministic function of stored data.
    #[test]
    fn test_v6_recompute_idempotent() {
        let conn = test_conn();

        let row = UsageEventRow {
            dedup_key: "msg_idem:req_idem",
            session_id: "sess_idem",
            project_path: "/Users/x/dev/tub2",
            project_name: "tub2",
            model: "claude-sonnet-5",
            input_tokens: 1_000,
            output_tokens: 100,
            cache_creation_tokens: 50,
            cache_read_tokens: 20,
            total_tokens: 1_170,
            cost: 0.0,
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        apply_v6(&conn).unwrap();
        let cost_after_first: f64 = conn
            .query_row(
                "SELECT cost FROM usage_events WHERE dedup_key = 'msg_idem:req_idem'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        apply_v6(&conn).unwrap();
        let cost_after_second: f64 = conn
            .query_row(
                "SELECT cost FROM usage_events WHERE dedup_key = 'msg_idem:req_idem'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(
            cost_after_first, cost_after_second,
            "second run of apply_v6 must not change cost"
        );
    }

    /// `apply_v6` must only touch `cost` — token counts, `total_tokens`,
    /// `project_name`, and `dedup_key` must be unchanged.
    #[test]
    fn test_v6_recompute_preserves_other_columns() {
        let conn = test_conn();

        let row = UsageEventRow {
            dedup_key: "msg_preserve:req_preserve",
            session_id: "sess_preserve",
            project_path: "/Users/x/dev/tub2",
            project_name: "tub2",
            model: "claude-sonnet-5",
            input_tokens: 1_000,
            output_tokens: 100,
            cache_creation_tokens: 50,
            cache_read_tokens: 20,
            total_tokens: 1_170,
            cost: 0.0,
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        apply_v6(&conn).unwrap();

        let (
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            total_tokens,
            project_name,
            dedup_key,
        ): (u64, u64, u64, u64, u64, String, String) = conn
            .query_row(
                "SELECT input_tokens, output_tokens, cache_creation_tokens, \
                        cache_read_tokens, total_tokens, project_name, dedup_key \
                 FROM usage_events WHERE dedup_key = 'msg_preserve:req_preserve'",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(input_tokens, 1_000);
        assert_eq!(output_tokens, 100);
        assert_eq!(cache_creation_tokens, 50);
        assert_eq!(cache_read_tokens, 20);
        assert_eq!(total_tokens, 1_170);
        assert_eq!(project_name, "tub2");
        assert_eq!(dedup_key, "msg_preserve:req_preserve");
    }

    /// An unknown model must keep `cost = 0` after `apply_v6` — the backfill
    /// only applies the current price table, it never invents prices.
    #[test]
    fn test_v6_recompute_unknown_model_stays_zero() {
        let conn = test_conn();

        let row = UsageEventRow {
            dedup_key: "msg_unknown:req_unknown",
            session_id: "sess_unknown",
            project_path: "/Users/x/dev/tub2",
            project_name: "tub2",
            model: "gpt-4o",
            input_tokens: 1_000,
            output_tokens: 100,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 1_100,
            cost: 0.0,
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        apply_v6(&conn).unwrap();

        let cost: f64 = conn
            .query_row(
                "SELECT cost FROM usage_events WHERE dedup_key = 'msg_unknown:req_unknown'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(cost, 0.0, "unknown model must keep cost at 0");
    }

    // -----------------------------------------------------------------------
    // Migration v7 — recompute cost tests (Fable/Mythos backfill)
    // -----------------------------------------------------------------------

    /// A pre-fix Fable 5 row (stored with `cost = 0`) must be repaired to a
    /// non-zero cost matching `pricing::cost` after `apply_v7`.
    #[test]
    fn test_v7_recompute_fixes_zero_cost_fable() {
        let conn = test_conn();

        let row = UsageEventRow {
            dedup_key: "msg_f5:req_f5",
            session_id: "sess_f5",
            project_path: "/Users/x/dev/tub2",
            project_name: "tub2",
            model: "claude-fable-5",
            input_tokens: 1_000,
            output_tokens: 100,
            cache_creation_tokens: 50,
            cache_read_tokens: 20,
            total_tokens: 1_170,
            cost: 0.0, // pre-fix: stored as zero
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        apply_v7(&conn).unwrap();

        let cost: f64 = conn
            .query_row(
                "SELECT cost FROM usage_events WHERE dedup_key = 'msg_f5:req_f5'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        let expected = crate::pricing::cost(
            "claude-fable-5",
            &crate::pricing::Usage {
                input_tokens: 1_000,
                output_tokens: 100,
                cache_creation_tokens: 50,
                cache_read_tokens: 20,
            },
        );
        assert!(
            (cost - expected).abs() < 1e-9,
            "got {cost}, want {expected}"
        );
        assert!(cost > 0.0, "fable 5 cost must be repaired to non-zero");
    }

    /// Running `apply_v7` a second time must not change an already-recomputed
    /// cost — the recompute is a pure, deterministic function of stored data.
    #[test]
    fn test_v7_recompute_idempotent() {
        let conn = test_conn();

        let row = UsageEventRow {
            dedup_key: "msg_f5_idem:req_f5_idem",
            session_id: "sess_f5_idem",
            project_path: "/Users/x/dev/tub2",
            project_name: "tub2",
            model: "claude-fable-5",
            input_tokens: 1_000,
            output_tokens: 100,
            cache_creation_tokens: 50,
            cache_read_tokens: 20,
            total_tokens: 1_170,
            cost: 0.0,
            timestamp: "2026-07-01T00:00:00Z",
            git_branch: None,
            ingested_at: "2026-07-01T00:00:01Z",
        };
        insert_event(&conn, &row).unwrap();

        apply_v7(&conn).unwrap();
        let cost_after_first: f64 = conn
            .query_row(
                "SELECT cost FROM usage_events WHERE dedup_key = 'msg_f5_idem:req_f5_idem'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        apply_v7(&conn).unwrap();
        let cost_after_second: f64 = conn
            .query_row(
                "SELECT cost FROM usage_events WHERE dedup_key = 'msg_f5_idem:req_f5_idem'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(
            cost_after_first, cost_after_second,
            "second run of apply_v7 must not change cost"
        );
    }
}
