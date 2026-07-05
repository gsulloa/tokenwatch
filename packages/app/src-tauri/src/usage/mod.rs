//! Tauri commands and background polling for usage data.
//!
//! Exposes three commands to the frontend:
//! - `refresh_usage`  — trigger an immediate full ingest.
//! - `query_series`   — aggregate series data for the chart.
//! - `usage_meta`     — metadata (last refresh, event count, date range).
//!
//! A background task (on Tauri's async runtime) runs ingest at startup and every
//! [`POLL_INTERVAL_SECS`] seconds, emitting `usage-updated` to the frontend
//! when new data arrives.

use std::sync::Mutex;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{db, ingest};

/// Seconds between background ingest polls.
pub const POLL_INTERVAL_SECS: u64 = 30;

/// Tauri event name emitted when new usage data is available.
pub const USAGE_UPDATED_EVENT: &str = "usage-updated";

// ---------------------------------------------------------------------------
// Public types (JSON contract with the frontend)
// ---------------------------------------------------------------------------

/// Temporal grouping bucket.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Bucket {
    Hour,
    Day,
    Week,
    Month,
}

/// Y-axis metric.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Metric {
    Tokens,
    Cost,
}

/// How to group series.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SeriesBy {
    Model,
    Project,
    ModelProject,
}

/// Parameters for `query_series`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesQuery {
    pub bucket: Bucket,
    pub metric: Metric,
    pub series_by: SeriesBy,
    /// Inclusive lower bound (ISO 8601 UTC datetime, e.g. `"2026-07-04T00:00:00Z"`).
    pub since: Option<String>,
    /// Inclusive upper bound (ISO 8601 UTC datetime, e.g. `"2026-07-04T23:59:59Z"`).
    pub until: Option<String>,
}

/// A single named data series aligned to the bucket labels.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub name: String,
    pub points: Vec<f64>,
}

/// Response from `query_series`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesResponse {
    /// Ordered bucket labels (e.g. `["2026-06-01", "2026-06-02", ...]`).
    pub buckets: Vec<String>,
    /// One series per group, each with `points.len() == buckets.len()`.
    pub series: Vec<Series>,
    pub metric: Metric,
    pub bucket: Bucket,
}

/// Returned by `refresh_usage`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSummary {
    pub events_inserted: u64,
    pub files_processed: u64,
    pub files_skipped: u64,
    pub refreshed_at: String,
}

/// Returned by `usage_meta`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMeta {
    pub last_refresh_at: Option<String>,
    pub event_count: u64,
    pub earliest_date: Option<String>,
    pub latest_date: Option<String>,
}

/// A single project row returned by `query_today_by_project`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUsageRow {
    pub project: String,
    pub tokens: i64,
    pub pct: f64,
}

/// Returned by `query_today_by_project`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayByProject {
    pub rows: Vec<ProjectUsageRow>,
    pub total_tokens: i64,
}

// ---------------------------------------------------------------------------
// Shared app state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub conn: Mutex<Connection>,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Force an immediate full ingest and return a summary.
#[tauri::command]
pub async fn refresh_usage(state: State<'_, AppState>) -> Result<RefreshSummary, String> {
    let summary = {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        let s = ingest::ingest(&conn).map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        db::set_last_refresh(&conn, &now).map_err(|e| e.to_string())?;
        (s, now)
    };

    Ok(RefreshSummary {
        events_inserted: summary.0.events_inserted,
        files_processed: summary.0.files_processed,
        files_skipped: summary.0.files_skipped,
        refreshed_at: summary.1,
    })
}

/// Query aggregated time-series data for the usage chart.
#[tauri::command]
pub fn query_series(
    state: State<'_, AppState>,
    params: SeriesQuery,
) -> Result<SeriesResponse, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    query_series_inner(&conn, &params).map_err(|e| e.to_string())
}

/// Return metadata about the stored usage data.
#[tauri::command]
pub fn usage_meta(state: State<'_, AppState>) -> Result<UsageMeta, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let last_refresh_at = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'last_refresh_at'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok();

    let event_count = db::count_events(&conn).map_err(|e| e.to_string())?;

    let (earliest_date, latest_date) = if event_count > 0 {
        let earliest: String = conn
            .query_row("SELECT MIN(timestamp) FROM usage_events", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let latest: String = conn
            .query_row("SELECT MAX(timestamp) FROM usage_events", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        (Some(earliest), Some(latest))
    } else {
        (None, None)
    };

    Ok(UsageMeta {
        last_refresh_at,
        event_count,
        earliest_date,
        latest_date,
    })
}

/// Query token consumption for today grouped by project (local time).
#[tauri::command]
pub fn query_today_by_project(state: State<'_, AppState>) -> Result<TodayByProject, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    // Compute start-of-today in local time, then convert to UTC RFC3339.
    let today_local = chrono::Local::now().date_naive();
    let today_start_local = today_local
        .and_hms_opt(0, 0, 0)
        .expect("midnight always valid");
    let today_start_utc: chrono::DateTime<chrono::Utc> =
        chrono::TimeZone::from_local_datetime(&chrono::Local, &today_start_local)
            .single()
            .ok_or_else(|| "ambiguous local midnight".to_owned())?
            .with_timezone(&chrono::Utc);
    let since = today_start_utc.to_rfc3339();

    let mut stmt = conn
        .prepare(
            "SELECT project_name, CAST(SUM(total_tokens) AS INTEGER)
             FROM usage_events
             WHERE timestamp >= ?1
             GROUP BY project_name
             ORDER BY 2 DESC",
        )
        .map_err(|e| e.to_string())?;

    let raw: Vec<(String, i64)> = stmt
        .query_map([&since], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let total_tokens: i64 = raw.iter().map(|(_, t)| t).sum();

    let rows = raw
        .into_iter()
        .map(|(project, tokens)| {
            let pct = if total_tokens > 0 {
                tokens as f64 / total_tokens as f64 * 100.0
            } else {
                0.0
            };
            ProjectUsageRow {
                project,
                tokens,
                pct,
            }
        })
        .collect();

    Ok(TodayByProject { rows, total_tokens })
}

// ---------------------------------------------------------------------------
// Internal query logic
// ---------------------------------------------------------------------------

/// Build the complete ordered list of bucket labels covering the effective range,
/// using a recursive CTE in SQLite. Anchors are in local wall-clock time so that
/// `strftime(fmt, anchor)` (no 'localtime' modifier) produces labels identical to
/// `strftime(fmt, timestamp, 'localtime')` used by the data query.
fn all_buckets_in_range(
    conn: &Connection,
    params: &SeriesQuery,
    strftime_fmt: &str,
    where_clause: &str,
) -> anyhow::Result<Vec<String>> {
    // The floor and step expressions differ by granularity.
    // Floor ensures the CTE starts at the bucket boundary that contains lo_local.
    let (floor_expr, step_expr, hi_floor_expr) = match params.bucket {
        Bucket::Hour => (
            // Floor to start of hour
            "strftime('%Y-%m-%d %H:00:00', lo_local)".to_owned(),
            "+1 hour".to_owned(),
            // Floor hi to start of its hour bucket
            "strftime('%Y-%m-%d %H:00:00', hi_local)".to_owned(),
        ),
        Bucket::Day => (
            // Floor to start of day
            "date(lo_local) || ' 00:00:00'".to_owned(),
            "+1 day".to_owned(),
            "date(hi_local) || ' 00:00:00'".to_owned(),
        ),
        Bucket::Week => (
            // Floor to Monday of the week containing lo_local.
            // SQLite's 'weekday 1' modifier: if current day IS Monday, it stays;
            // otherwise advances to next Monday. So we subtract 6 days first to
            // ensure we always land on or before current day's Monday.
            "date(lo_local, '-6 days', 'weekday 1') || ' 00:00:00'".to_owned(),
            "+7 days".to_owned(),
            "date(hi_local, '-6 days', 'weekday 1') || ' 00:00:00'".to_owned(),
        ),
        Bucket::Month => (
            // Floor to start of month
            "date(lo_local, 'start of month') || ' 00:00:00'".to_owned(),
            "+1 month".to_owned(),
            "date(hi_local, 'start of month') || ' 00:00:00'".to_owned(),
        ),
    };

    // Build lo_local / hi_local sub-selects depending on whether since/until are provided.
    let (lo_select, hi_select) = match (&params.since, &params.until) {
        (Some(since), Some(until)) => (
            format!("datetime('{since}', 'localtime')"),
            format!("datetime('{until}', 'localtime')"),
        ),
        (Some(since), None) => (
            format!("datetime('{since}', 'localtime')"),
            format!(
                "(SELECT datetime(MAX(timestamp), 'localtime') FROM usage_events {where_clause})"
            ),
        ),
        (None, Some(until)) => (
            format!(
                "(SELECT datetime(MIN(timestamp), 'localtime') FROM usage_events {where_clause})"
            ),
            format!("datetime('{until}', 'localtime')"),
        ),
        (None, None) => (
            format!(
                "(SELECT datetime(MIN(timestamp), 'localtime') FROM usage_events {where_clause})"
            ),
            format!(
                "(SELECT datetime(MAX(timestamp), 'localtime') FROM usage_events {where_clause})"
            ),
        ),
    };

    // Recursive CTE: generate anchors from floor(lo) to floor(hi) inclusive,
    // stepping by the bucket interval. A hard limit of 5000 prevents runaway loops.
    let cte_sql = format!(
        "WITH RECURSIVE bounds AS (
            SELECT {lo_select} AS lo_local,
                   {hi_select} AS hi_local
        ),
        buckets(anchor, hi_floor, depth) AS (
            SELECT {floor_expr},
                   {hi_floor_expr},
                   0
            FROM bounds
            UNION ALL
            SELECT datetime(anchor, '{step_expr}'),
                   hi_floor,
                   depth + 1
            FROM buckets
            WHERE anchor < hi_floor
              AND depth < 5000
        )
        SELECT DISTINCT strftime('{strftime_fmt}', anchor) AS label
        FROM buckets
        ORDER BY label",
        lo_select = lo_select,
        hi_select = hi_select,
        floor_expr = floor_expr,
        hi_floor_expr = hi_floor_expr,
        step_expr = step_expr,
        strftime_fmt = strftime_fmt,
    );

    let mut stmt = conn.prepare(&cte_sql)?;
    let labels: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(labels)
}

fn query_series_inner(conn: &Connection, params: &SeriesQuery) -> anyhow::Result<SeriesResponse> {
    // Build the strftime format string and label formatter for each bucket type.
    let (strftime_fmt, _label_hint) = match params.bucket {
        Bucket::Hour => ("%Y-%m-%d %H:00", "hour"),
        Bucket::Day => ("%Y-%m-%d", "day"),
        Bucket::Week => ("%Y-W%W", "week"),
        Bucket::Month => ("%Y-%m", "month"),
    };

    // Build the GROUP BY / SELECT expression for the series name.
    let series_col = match params.series_by {
        SeriesBy::Model => "model".to_owned(),
        SeriesBy::Project => "project_name".to_owned(),
        SeriesBy::ModelProject => "model || ' \u{00b7} ' || project_name".to_owned(),
    };

    // Build the metric aggregate expression.
    let metric_expr = match params.metric {
        Metric::Tokens => "SUM(total_tokens)",
        Metric::Cost => "SUM(cost)",
    };

    // Build optional date filters.
    // `since` and `until` are full ISO 8601 UTC datetimes computed by the frontend (D2).
    let mut conditions = Vec::new();
    if let Some(since) = &params.since {
        conditions.push(format!("timestamp >= '{since}'"));
    }
    if let Some(until) = &params.until {
        conditions.push(format!("timestamp <= '{until}'"));
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Query raw (bucket_label, series_name, value) rows.
    // The 'localtime' modifier converts UTC timestamps to the host's local time zone
    // before bucketing, so grouping reflects the user's local day/hour (D1).
    let sql = format!(
        "SELECT strftime('{strftime_fmt}', timestamp, 'localtime') AS bucket_label,
                {series_col} AS series_name,
                {metric_expr} AS value
         FROM usage_events
         {where_clause}
         GROUP BY bucket_label, series_name
         ORDER BY bucket_label, series_name"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<(String, String, f64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        return Ok(SeriesResponse {
            buckets: vec![],
            series: vec![],
            metric: params.metric,
            bucket: params.bucket,
        });
    }

    // Build the full ordered bucket list (including empty buckets) using the
    // recursive CTE helper. This replaces the old row-derived dedup approach.
    let buckets = all_buckets_in_range(conn, params, strftime_fmt, &where_clause)?;

    let mut seen_series = std::collections::HashSet::new();
    let series_names: Vec<String> = rows
        .iter()
        .map(|r| r.1.clone())
        .filter(|s| seen_series.insert(s.clone()))
        .collect();

    // Build a lookup map (bucket, series) → value.
    let mut value_map: std::collections::HashMap<(String, String), f64> =
        std::collections::HashMap::new();
    for (bucket_label, series_name, value) in &rows {
        value_map.insert((bucket_label.clone(), series_name.clone()), *value);
    }

    // Assemble series with 0-filled gaps (now against the complete bucket list).
    let series: Vec<Series> = series_names
        .into_iter()
        .map(|name| {
            let points: Vec<f64> = buckets
                .iter()
                .map(|b| {
                    value_map
                        .get(&(b.clone(), name.clone()))
                        .copied()
                        .unwrap_or(0.0)
                })
                .collect();
            Series { name, points }
        })
        .collect();

    Ok(SeriesResponse {
        buckets,
        series,
        metric: params.metric,
        bucket: params.bucket,
    })
}

// ---------------------------------------------------------------------------
// Background polling task
// ---------------------------------------------------------------------------

/// Spawn the background polling task. Call once from `.setup()`.
pub fn spawn_polling_task(app_handle: AppHandle) {
    // Use Tauri's managed async runtime: `.setup()` does not run inside a Tokio
    // reactor context, so `tokio::spawn` would panic ("there is no reactor
    // running"). `tauri::async_runtime::spawn` schedules onto Tauri's runtime.
    tauri::async_runtime::spawn(async move {
        // Run once immediately at startup.
        run_refresh_and_emit(&app_handle).await;

        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS));
        interval.tick().await; // consume the first immediate tick
        loop {
            interval.tick().await;
            run_refresh_and_emit(&app_handle).await;
        }
    });
}

async fn run_refresh_and_emit(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let result = {
        let conn = match state.conn.lock() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to lock db connection: {e}");
                return;
            }
        };
        let summary = ingest::ingest(&conn);
        let now = chrono::Utc::now().to_rfc3339();
        if let Ok(ref s) = summary {
            let _ = db::set_last_refresh(&conn, &now);
            s.events_inserted
        } else {
            0
        }
    };

    if result > 0 {
        if let Err(e) = app_handle.emit(USAGE_UPDATED_EVENT, result) {
            tracing::warn!("failed to emit {USAGE_UPDATED_EVENT}: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, UsageEventRow};

    fn test_conn() -> Connection {
        db::open_at(":memory:").expect("in-memory db")
    }

    fn seed_events(conn: &Connection) {
        let events = vec![
            // Day 1 — opus
            UsageEventRow {
                dedup_key: "m1:r1",
                session_id: "s",
                project_path: "/p/backend/madrid",
                project_name: "backend/madrid",
                model: "claude-opus-4-8",
                input_tokens: 1000,
                output_tokens: 100,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 1100,
                cost: 0.005,
                timestamp: "2026-06-01T10:00:00Z",
                git_branch: None,
                ingested_at: "2026-06-01T10:01:00Z",
            },
            // Day 1 — sonnet
            UsageEventRow {
                dedup_key: "m2:r2",
                session_id: "s",
                project_path: "/p/backend/madrid",
                project_name: "backend/madrid",
                model: "claude-sonnet-4-6",
                input_tokens: 500,
                output_tokens: 50,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 550,
                cost: 0.002,
                timestamp: "2026-06-01T11:00:00Z",
                git_branch: None,
                ingested_at: "2026-06-01T10:01:00Z",
            },
            // Day 2 — opus, different project
            UsageEventRow {
                dedup_key: "m3:r3",
                session_id: "s",
                project_path: "/p/frontend/app",
                project_name: "frontend/app",
                model: "claude-opus-4-8",
                input_tokens: 2000,
                output_tokens: 200,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 2200,
                cost: 0.015,
                timestamp: "2026-06-02T09:00:00Z",
                git_branch: None,
                ingested_at: "2026-06-02T09:01:00Z",
            },
            // Day 5 — sonnet (gap on days 3 and 4)
            UsageEventRow {
                dedup_key: "m4:r4",
                session_id: "s",
                project_path: "/p/backend/madrid",
                project_name: "backend/madrid",
                model: "claude-sonnet-4-6",
                input_tokens: 800,
                output_tokens: 80,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 880,
                cost: 0.003,
                timestamp: "2026-06-05T14:00:00Z",
                git_branch: None,
                ingested_at: "2026-06-05T14:01:00Z",
            },
        ];

        for row in &events {
            db::insert_event(conn, row).unwrap();
        }
    }

    #[test]
    fn test_query_series_by_model_day() {
        let conn = test_conn();
        seed_events(&conn);

        let params = SeriesQuery {
            bucket: Bucket::Day,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        // With full-range filling, bounds are data min/max (06-01..06-05)
        // → all 5 days must appear: 2026-06-01 through 2026-06-05.
        assert_eq!(
            resp.buckets,
            vec![
                "2026-06-01",
                "2026-06-02",
                "2026-06-03",
                "2026-06-04",
                "2026-06-05",
            ]
        );
        assert_eq!(resp.series.len(), 2); // opus and sonnet

        let opus = resp
            .series
            .iter()
            .find(|s| s.name == "claude-opus-4-8")
            .unwrap();
        // Day 1: 1100, Day 2: 2200, Day 3: 0, Day 4: 0, Day 5: 0
        assert_eq!(opus.points.len(), 5);
        assert!((opus.points[0] - 1100.0).abs() < 1.0);
        assert!((opus.points[1] - 2200.0).abs() < 1.0);
        assert!((opus.points[2] - 0.0).abs() < 1.0, "day 3 must be 0");
        assert!((opus.points[3] - 0.0).abs() < 1.0, "day 4 must be 0");
        assert!((opus.points[4] - 0.0).abs() < 1.0, "day 5 must be 0");

        let sonnet = resp
            .series
            .iter()
            .find(|s| s.name == "claude-sonnet-4-6")
            .unwrap();
        // Day 1: 550, Day 2: 0, Day 3: 0, Day 4: 0, Day 5: 880
        assert!((sonnet.points[0] - 550.0).abs() < 1.0);
        assert!((sonnet.points[1] - 0.0).abs() < 1.0, "day 2 must be 0");
        assert!((sonnet.points[2] - 0.0).abs() < 1.0, "day 3 must be 0");
        assert!((sonnet.points[3] - 0.0).abs() < 1.0, "day 4 must be 0");
        assert!((sonnet.points[4] - 880.0).abs() < 1.0);
    }

    #[test]
    fn test_query_series_by_project() {
        let conn = test_conn();
        seed_events(&conn);

        let params = SeriesQuery {
            bucket: Bucket::Day,
            metric: Metric::Cost,
            series_by: SeriesBy::Project,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        assert!(
            resp.series.iter().any(|s| s.name == "backend/madrid"),
            "expected backend/madrid series"
        );
        assert!(
            resp.series.iter().any(|s| s.name == "frontend/app"),
            "expected frontend/app series"
        );
    }

    #[test]
    fn test_query_series_by_model_project() {
        let conn = test_conn();
        seed_events(&conn);

        let params = SeriesQuery {
            bucket: Bucket::Day,
            metric: Metric::Tokens,
            series_by: SeriesBy::ModelProject,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        // Series names should be like "claude-opus-4-8 · backend/madrid"
        assert!(
            resp.series
                .iter()
                .any(|s| s.name.contains("claude-opus-4-8") && s.name.contains("backend/madrid")),
            "expected combined model·project series; got: {:?}",
            resp.series.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_query_series_week_bucket() {
        let conn = test_conn();
        seed_events(&conn);

        let params = SeriesQuery {
            bucket: Bucket::Week,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();
        // Weeks: 2026-W22 (Jun 1), 2026-W23 (Jun 2 and 5 — same ISO week group may differ)
        // Just assert we get at least 1 bucket and each series aligns.
        assert!(!resp.buckets.is_empty());
        for series in &resp.series {
            assert_eq!(
                series.points.len(),
                resp.buckets.len(),
                "series '{}' has mismatched points",
                series.name
            );
        }
    }

    #[test]
    fn test_query_series_month_bucket() {
        let conn = test_conn();
        seed_events(&conn);

        let params = SeriesQuery {
            bucket: Bucket::Month,
            metric: Metric::Cost,
            series_by: SeriesBy::Model,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();
        // All events are in June 2026 → expect 1 month bucket
        assert_eq!(resp.buckets, vec!["2026-06"]);
        for series in &resp.series {
            assert_eq!(series.points.len(), 1);
        }
    }

    #[test]
    fn test_query_series_empty_db() {
        let conn = test_conn();

        let params = SeriesQuery {
            bucket: Bucket::Day,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();
        assert!(resp.buckets.is_empty());
        assert!(resp.series.is_empty());
    }

    #[test]
    fn test_query_series_since_until_filter() {
        let conn = test_conn();
        seed_events(&conn);

        // since/until are now full ISO 8601 UTC datetimes (D2).
        let params = SeriesQuery {
            bucket: Bucket::Day,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: Some("2026-06-01T00:00:00Z".to_owned()),
            until: Some("2026-06-02T23:59:59Z".to_owned()),
        };

        let resp = query_series_inner(&conn, &params).unwrap();
        // Day 5 is outside the range
        assert!(
            !resp.buckets.contains(&"2026-06-05".to_owned()),
            "day 5 should be excluded by until filter"
        );
    }

    #[test]
    fn test_empty_bucket_filling() {
        // Explicitly verify that a series with a gap returns 0 for that bucket.
        let conn = test_conn();

        // Insert two events for opus on day 1 and day 3 only (gap on day 2).
        let rows = vec![
            UsageEventRow {
                dedup_key: "gap1:r1",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 100,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 110,
                cost: 0.001,
                timestamp: "2026-07-01T10:00:00Z",
                git_branch: None,
                ingested_at: "2026-07-01T10:01:00Z",
            },
            UsageEventRow {
                dedup_key: "gap2:r2",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 200,
                output_tokens: 20,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 220,
                cost: 0.002,
                timestamp: "2026-07-03T10:00:00Z",
                git_branch: None,
                ingested_at: "2026-07-03T10:01:00Z",
            },
        ];
        for r in &rows {
            db::insert_event(&conn, r).unwrap();
        }

        let params = SeriesQuery {
            bucket: Bucket::Day,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();
        // With full-range filling, all 3 days (2026-07-01, 2026-07-02, 2026-07-03)
        // must appear; day 2 is a zero-filled bucket.
        assert_eq!(resp.buckets, vec!["2026-07-01", "2026-07-02", "2026-07-03"]);
        assert_eq!(resp.buckets.len(), 3);
        let opus = &resp.series[0];
        assert_eq!(opus.points.len(), 3);
        assert!((opus.points[0] - 110.0).abs() < 1.0);
        assert!((opus.points[1] - 0.0).abs() < 1.0, "day 2 must be 0");
        assert!((opus.points[2] - 220.0).abs() < 1.0);
    }

    // -------------------------------------------------------------------------
    // Task 1.4(a): Bucket::Hour returns per-hour bucket labels
    // -------------------------------------------------------------------------
    #[test]
    fn test_query_series_hour_bucket_labels() {
        let conn = test_conn();

        // Insert two events in the same UTC day but different hours.
        let rows = vec![
            UsageEventRow {
                dedup_key: "h1:r1",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 100,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 110,
                cost: 0.001,
                timestamp: "2026-07-04T10:00:00Z",
                git_branch: None,
                ingested_at: "2026-07-04T10:01:00Z",
            },
            UsageEventRow {
                dedup_key: "h2:r2",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 200,
                output_tokens: 20,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 220,
                cost: 0.002,
                timestamp: "2026-07-04T11:00:00Z",
                git_branch: None,
                ingested_at: "2026-07-04T11:01:00Z",
            },
        ];
        for r in &rows {
            db::insert_event(&conn, r).unwrap();
        }

        let params = SeriesQuery {
            bucket: Bucket::Hour,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        // Each event falls in a different UTC hour, so even after 'localtime'
        // conversion they must produce exactly 2 distinct hour buckets.
        assert_eq!(resp.buckets.len(), 2, "expected 2 hour buckets");

        // Bucket labels must match the 'YYYY-MM-DD HH:00' format.
        for label in &resp.buckets {
            assert!(
                label.len() == 16 && label.contains(':'),
                "bucket label '{label}' does not match expected hour format 'YYYY-MM-DD HH:00'"
            );
        }

        // The two bucket labels should agree with what SQLite's strftime+localtime
        // returns for those exact timestamps — derive expected labels the same way.
        let expected_labels: Vec<String> = conn
            .prepare(
                "SELECT strftime('%Y-%m-%d %H:00', '2026-07-04T10:00:00Z', 'localtime')
                 UNION ALL
                 SELECT strftime('%Y-%m-%d %H:00', '2026-07-04T11:00:00Z', 'localtime')
                 ORDER BY 1",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(
            resp.buckets, expected_labels,
            "hour bucket labels must match SQLite strftime+localtime output"
        );

        // All tokens are under one model, so the single series must have 2 points.
        assert_eq!(resp.series.len(), 1);
        let opus = &resp.series[0];
        assert_eq!(opus.points.len(), 2);
        let total: f64 = opus.points.iter().sum();
        assert!((total - 330.0).abs() < 1.0, "total tokens should be 330");
    }

    // -------------------------------------------------------------------------
    // Task 1.4(b): datetime since/until filters include only events in range
    // (last-24h style — insert one event "now" and one 25h ago, filter since
    //  now-24h, assert only the recent event is counted)
    // -------------------------------------------------------------------------
    #[test]
    fn test_query_series_datetime_filter_last_24h() {
        let conn = test_conn();

        // Use fixed timestamps so the test is deterministic.
        // "recent"  : 2026-07-04T12:00:00Z  — inside  the last-24h window
        // "old"     : 2026-07-03T11:00:00Z  — 25 hours before the "now" anchor
        // Filter    : since=2026-07-03T12:00:00Z  until=2026-07-04T12:00:00Z
        let recent_ts = "2026-07-04T12:00:00Z";
        let old_ts = "2026-07-03T11:00:00Z";

        let rows = vec![
            UsageEventRow {
                dedup_key: "dt1:r1",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 100,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 110,
                cost: 0.001,
                timestamp: recent_ts,
                git_branch: None,
                ingested_at: "2026-07-04T12:01:00Z",
            },
            UsageEventRow {
                dedup_key: "dt2:r2",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 999,
                output_tokens: 99,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 1098,
                cost: 0.009,
                timestamp: old_ts,
                git_branch: None,
                ingested_at: "2026-07-03T11:01:00Z",
            },
        ];
        for r in &rows {
            db::insert_event(&conn, r).unwrap();
        }

        // Query the last 24h with full datetime precision (D2 contract).
        let params = SeriesQuery {
            bucket: Bucket::Hour,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: Some("2026-07-03T12:00:00Z".to_owned()),
            until: Some("2026-07-04T12:00:00Z".to_owned()),
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        // Only the recent event (110 tokens) should be included.
        let total_tokens: f64 = resp.series.iter().flat_map(|s| s.points.iter()).sum();
        assert!(
            (total_tokens - 110.0).abs() < 1.0,
            "expected only the recent event (110 tokens) to be included; got {total_tokens}"
        );
    }

    // -------------------------------------------------------------------------
    // Task 3.2: Hour bucket with since/until spanning 24h, events in only 2 hours.
    // All 24 hourly buckets must appear; hours without events must be 0.
    // -------------------------------------------------------------------------
    #[test]
    fn test_hour_full_24h_range_all_buckets_present() {
        let conn = test_conn();

        // Two events, both within the 24h window but in specific hours only.
        let rows = vec![
            UsageEventRow {
                dedup_key: "h24:r1",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 100,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 110,
                cost: 0.001,
                // 2026-07-04T02:00:00Z — an event at 02:00 UTC
                timestamp: "2026-07-04T02:00:00Z",
                git_branch: None,
                ingested_at: "2026-07-04T02:01:00Z",
            },
            UsageEventRow {
                dedup_key: "h24:r2",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 200,
                output_tokens: 20,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 220,
                cost: 0.002,
                // 2026-07-04T20:00:00Z — an event at 20:00 UTC
                timestamp: "2026-07-04T20:00:00Z",
                git_branch: None,
                ingested_at: "2026-07-04T20:01:00Z",
            },
        ];
        for r in &rows {
            db::insert_event(&conn, r).unwrap();
        }

        // Query exactly 24 hours: 2026-07-04T00:00:00Z .. 2026-07-04T23:59:59Z
        let params = SeriesQuery {
            bucket: Bucket::Hour,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: Some("2026-07-04T00:00:00Z".to_owned()),
            until: Some("2026-07-04T23:59:59Z".to_owned()),
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        // Must have exactly 24 hourly buckets.
        assert_eq!(
            resp.buckets.len(),
            24,
            "expected 24 hourly buckets; got {}: {:?}",
            resp.buckets.len(),
            resp.buckets
        );

        // All bucket labels must have the 'YYYY-MM-DD HH:00' format.
        for label in &resp.buckets {
            assert_eq!(label.len(), 16, "bucket label '{label}' has wrong length");
        }

        // Total tokens across all buckets must equal only the 2 seeded events.
        let total: f64 = resp.series.iter().flat_map(|s| s.points.iter()).sum();
        assert!(
            (total - 330.0).abs() < 1.0,
            "expected total 330 tokens; got {total}"
        );

        // Verify that buckets without events are truly 0.
        assert_eq!(resp.series.len(), 1);
        let opus = &resp.series[0];
        assert_eq!(opus.points.len(), 24);
        let nonzero_count = opus.points.iter().filter(|&&v| v > 0.0).count();
        assert_eq!(
            nonzero_count, 2,
            "expected exactly 2 non-zero hour buckets; got {nonzero_count}"
        );
    }

    // -------------------------------------------------------------------------
    // Task 3.3: Final bucket inclusive — until lands inside last bucket.
    // The bucket containing 'until' must be present even if 'until' is not
    // at the exact bucket boundary.
    // -------------------------------------------------------------------------
    #[test]
    fn test_final_bucket_inclusive() {
        let conn = test_conn();

        // One event at 2026-07-10T10:30:00Z.
        let row = UsageEventRow {
            dedup_key: "inc:r1",
            session_id: "s",
            project_path: "/p",
            project_name: "p",
            model: "claude-opus-4-8",
            input_tokens: 100,
            output_tokens: 10,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 110,
            cost: 0.001,
            timestamp: "2026-07-10T10:30:00Z",
            git_branch: None,
            ingested_at: "2026-07-10T10:31:00Z",
        };
        db::insert_event(&conn, &row).unwrap();

        // Query with until = 2026-07-12T12:00:00Z (mid-day, inside the 2026-07-12 bucket).
        // The day bucket for 2026-07-12 must appear even though 'until' is midday.
        let params = SeriesQuery {
            bucket: Bucket::Day,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: Some("2026-07-10T00:00:00Z".to_owned()),
            until: Some("2026-07-12T12:00:00Z".to_owned()),
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        // Should have 3 days: 2026-07-10, 2026-07-11, 2026-07-12
        assert!(
            resp.buckets.contains(&"2026-07-12".to_owned()),
            "last bucket 2026-07-12 must be present; got: {:?}",
            resp.buckets
        );
        assert_eq!(
            resp.buckets.len(),
            3,
            "expected 3 day buckets; got {:?}",
            resp.buckets
        );
    }

    // -------------------------------------------------------------------------
    // Task 3.4a: Range crossing midnight (hour granularity) — labels match data query.
    // Task 3.4b: Week granularity crossing a year boundary — labels match data query.
    // -------------------------------------------------------------------------
    #[test]
    fn test_hour_range_crossing_midnight_label_match() {
        let conn = test_conn();

        // One event straddling midnight UTC (23:00 → local may be different date).
        let row = UsageEventRow {
            dedup_key: "mid:r1",
            session_id: "s",
            project_path: "/p",
            project_name: "p",
            model: "claude-opus-4-8",
            input_tokens: 100,
            output_tokens: 10,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 110,
            cost: 0.001,
            timestamp: "2026-07-04T23:30:00Z",
            git_branch: None,
            ingested_at: "2026-07-04T23:31:00Z",
        };
        db::insert_event(&conn, &row).unwrap();

        // Query spanning the midnight boundary: 22:00 .. 01:00 next day.
        let params = SeriesQuery {
            bucket: Bucket::Hour,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: Some("2026-07-04T22:00:00Z".to_owned()),
            until: Some("2026-07-05T01:00:00Z".to_owned()),
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        // Derive the expected label for the event using the same strftime+localtime
        // that the data query uses, to guarantee label-matching.
        let expected_event_label: String = conn
            .query_row(
                "SELECT strftime('%Y-%m-%d %H:00', '2026-07-04T23:30:00Z', 'localtime')",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert!(
            resp.buckets.contains(&expected_event_label),
            "event label '{expected_event_label}' must be in buckets: {:?}",
            resp.buckets
        );

        // Total tokens must equal only the 1 seeded event.
        let total: f64 = resp.series.iter().flat_map(|s| s.points.iter()).sum();
        assert!(
            (total - 110.0).abs() < 1.0,
            "expected 110 tokens; got {total}"
        );
    }

    #[test]
    fn test_week_range_crossing_year_boundary_label_match() {
        let conn = test_conn();

        // One event in week straddling the year boundary 2025→2026.
        // 2025-12-31 (Wednesday) and 2026-01-05 (Monday of the next week).
        let rows = vec![
            UsageEventRow {
                dedup_key: "yr:r1",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 100,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 110,
                cost: 0.001,
                timestamp: "2025-12-31T10:00:00Z",
                git_branch: None,
                ingested_at: "2025-12-31T10:01:00Z",
            },
            UsageEventRow {
                dedup_key: "yr:r2",
                session_id: "s",
                project_path: "/p",
                project_name: "p",
                model: "claude-opus-4-8",
                input_tokens: 200,
                output_tokens: 20,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 220,
                cost: 0.002,
                timestamp: "2026-01-05T10:00:00Z",
                git_branch: None,
                ingested_at: "2026-01-05T10:01:00Z",
            },
        ];
        for r in &rows {
            db::insert_event(&conn, r).unwrap();
        }

        let params = SeriesQuery {
            bucket: Bucket::Week,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: None,
            until: None,
        };

        let resp = query_series_inner(&conn, &params).unwrap();

        // Derive expected week labels the same way the data query does.
        let expected_labels: Vec<String> = conn
            .prepare(
                "SELECT DISTINCT strftime('%Y-W%W', ts, 'localtime') AS lbl
                 FROM (
                   SELECT '2025-12-31T10:00:00Z' AS ts
                   UNION ALL
                   SELECT '2026-01-05T10:00:00Z'
                 )
                 ORDER BY lbl",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        // Every expected label must appear in the response buckets.
        for lbl in &expected_labels {
            assert!(
                resp.buckets.contains(lbl),
                "expected week label '{lbl}' not found in buckets: {:?}",
                resp.buckets
            );
        }

        // All series points must align with buckets.
        for series in &resp.series {
            assert_eq!(series.points.len(), resp.buckets.len());
        }

        // Total tokens must equal sum of both events.
        let total: f64 = resp.series.iter().flat_map(|s| s.points.iter()).sum();
        assert!(
            (total - 330.0).abs() < 1.0,
            "expected 330 tokens; got {total}"
        );
    }
}
