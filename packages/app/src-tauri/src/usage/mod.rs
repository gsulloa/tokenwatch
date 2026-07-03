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
    /// Inclusive lower bound (ISO 8601 date string, e.g. `"2026-06-01"`).
    pub since: Option<String>,
    /// Inclusive upper bound (ISO 8601 date string, e.g. `"2026-07-03"`).
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

// ---------------------------------------------------------------------------
// Internal query logic
// ---------------------------------------------------------------------------

fn query_series_inner(conn: &Connection, params: &SeriesQuery) -> anyhow::Result<SeriesResponse> {
    // Build the strftime format string and label formatter for each bucket type.
    let (strftime_fmt, _label_hint) = match params.bucket {
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
    let mut conditions = Vec::new();
    if let Some(since) = &params.since {
        conditions.push(format!("timestamp >= '{since}T00:00:00Z'"));
    }
    if let Some(until) = &params.until {
        conditions.push(format!("timestamp <= '{until}T23:59:59Z'"));
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Query raw (bucket_label, series_name, value) rows.
    let sql = format!(
        "SELECT strftime('{strftime_fmt}', timestamp) AS bucket_label,
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

    // Collect ordered unique bucket labels and series names.
    let mut bucket_set: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
    bucket_set.dedup();
    // Ensure strict ordering (they come ordered from SQL already, but dedup by adjacent).
    let mut seen_buckets = std::collections::HashSet::new();
    let buckets: Vec<String> = bucket_set
        .into_iter()
        .filter(|b| seen_buckets.insert(b.clone()))
        .collect();

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

    // Assemble series with 0-filled gaps.
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

        // Expect 3 unique day buckets: 2026-06-01, 2026-06-02, 2026-06-05
        assert_eq!(resp.buckets, vec!["2026-06-01", "2026-06-02", "2026-06-05"]);
        assert_eq!(resp.series.len(), 2); // opus and sonnet

        let opus = resp
            .series
            .iter()
            .find(|s| s.name == "claude-opus-4-8")
            .unwrap();
        // Day 1: 1100, Day 2: 2200, Day 5: 0
        assert_eq!(opus.points.len(), 3);
        assert!((opus.points[0] - 1100.0).abs() < 1.0);
        assert!((opus.points[1] - 2200.0).abs() < 1.0);
        assert!((opus.points[2] - 0.0).abs() < 1.0, "gap bucket must be 0");

        let sonnet = resp
            .series
            .iter()
            .find(|s| s.name == "claude-sonnet-4-6")
            .unwrap();
        // Day 1: 550, Day 2: 0, Day 5: 880
        assert!((sonnet.points[0] - 550.0).abs() < 1.0);
        assert!((sonnet.points[1] - 0.0).abs() < 1.0, "gap bucket must be 0");
        assert!((sonnet.points[2] - 880.0).abs() < 1.0);
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

        let params = SeriesQuery {
            bucket: Bucket::Day,
            metric: Metric::Tokens,
            series_by: SeriesBy::Model,
            since: Some("2026-06-01".to_owned()),
            until: Some("2026-06-02".to_owned()),
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
        // Only 2 actual buckets (2026-07-01 and 2026-07-03) — day 2 never existed.
        // This confirms the filling only adds 0 for buckets that exist in the global list.
        assert_eq!(resp.buckets.len(), 2);
        let opus = &resp.series[0];
        assert!((opus.points[0] - 110.0).abs() < 1.0);
        assert!((opus.points[1] - 220.0).abs() < 1.0);
    }
}
