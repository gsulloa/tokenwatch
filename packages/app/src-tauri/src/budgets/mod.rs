//! Per-project-group budget tracking.
//!
//! Implements:
//! - CRUD commands for `project_groups` and `project_group_members`.
//! - `query_group_budgets`: pure read, no network I/O.
//! - `evaluate_group_alerts`: called from the limits poll under a single
//!   `conn` lock to avoid races.
//!
//! Design references: D3, D4, D5, D6, D7, D8, D9, D12, D17, D18.

use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{db, limits::current_session_window, usage::AppState};

// ---------------------------------------------------------------------------
// Public types — JSON contract with the frontend
// ---------------------------------------------------------------------------

/// A single defined project group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub budget_basis: Option<String>,
    pub budget_value: Option<f64>,
}

/// A group together with its current member project names.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupWithMembers {
    pub group: Group,
    pub members: Vec<String>,
}

/// One row in `GroupBudgetsSnapshot.rows`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupBudgetRow {
    /// `None` for the synthetic "otros" bucket.
    pub group_id: Option<i64>,
    pub name: String,
    pub budget_basis: Option<String>,
    pub budget_value: Option<f64>,
    /// Sum of `cost` from `usage_events` in the window for this group's projects.
    pub window_cost_usd: f64,
    /// `window_cost_usd / total_window_cost * 100`; 0 when total == 0.
    /// Pure local ratio — NOT multiplied by `session.utilization`.
    pub local_cost_share_pct: f64,
    /// Session-weighted estimate: `local_cost_share_pct × session.utilization / 100`.
    /// Available ONLY when `origin = "session"` AND the snapshot has a session utilization.
    /// `None` in rolling mode (no active session).
    /// Summed across all groups ≈ `session.utilization`. Labeled "est." in the UI.
    pub session_weighted_pct: Option<f64>,
    /// `share` → `session_weighted_pct` (session, est.); `usd` → `window_cost_usd`; `None` → `null`.
    /// In rolling mode, `share` basis yields `None` (no session → no weighted estimate → no alert).
    pub measured_value: Option<f64>,
}

/// Returned by `query_group_budgets`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupBudgetsSnapshot {
    pub rows: Vec<GroupBudgetRow>,
    /// ISO timestamp (…Z millis) of the window's lower bound.
    pub window_start: String,
    /// `"session"` when anchored to `session.resets_at`; `"rolling"` otherwise.
    pub origin: String,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_budget(basis: &Option<String>, value: &Option<f64>) -> Result<(), String> {
    match basis.as_deref() {
        None => {
            // No cap — value must also be None (we just ignore it silently, backend enforces).
            Ok(())
        }
        Some("share") => {
            let v = value.ok_or_else(|| "budget_value required for basis 'share'".to_owned())?;
            if v <= 0.0 || v > 100.0 {
                return Err(format!(
                    "budget_value for 'share' must be in (0, 100]; got {v}"
                ));
            }
            Ok(())
        }
        Some("usd") => {
            let v = value.ok_or_else(|| "budget_value required for basis 'usd'".to_owned())?;
            if v <= 0.0 {
                return Err(format!("budget_value for 'usd' must be > 0; got {v}"));
            }
            Ok(())
        }
        Some(other) => Err(format!(
            "budget_basis must be 'share', 'usd', or null; got '{other}'"
        )),
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("group name must not be empty".to_owned());
    }
    Ok(())
}

/// Map a rusqlite UNIQUE constraint violation to a readable message.
fn map_unique_err(e: rusqlite::Error, name: &str) -> String {
    if let rusqlite::Error::SqliteFailure(ref fe, _) = e {
        if fe.extended_code == 2067 || fe.code == rusqlite::ErrorCode::ConstraintViolation {
            return format!("A group named '{name}' already exists");
        }
    }
    e.to_string()
}

// ---------------------------------------------------------------------------
// CRUD commands
// ---------------------------------------------------------------------------

/// List all groups together with their member project names.
#[tauri::command]
pub fn list_groups(state: State<'_, AppState>) -> Result<Vec<GroupWithMembers>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    list_groups_inner(&conn)
}

fn list_groups_inner(conn: &Connection) -> Result<Vec<GroupWithMembers>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT g.id, g.name, g.budget_basis, g.budget_value,
                    m.project_name
             FROM project_groups g
             LEFT JOIN project_group_members m ON m.group_id = g.id
             ORDER BY g.id, m.project_name",
        )
        .map_err(|e| e.to_string())?;

    // (group_id, name, basis, value, project_name?)
    type GroupRow = (i64, String, Option<String>, Option<f64>, Option<String>);
    let rows: Vec<GroupRow> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e: rusqlite::Error| e.to_string())?;

    let mut groups: Vec<GroupWithMembers> = Vec::new();
    for (id, name, basis, value, member) in rows {
        if let Some(last) = groups.last_mut() {
            if last.group.id == id {
                if let Some(m) = member {
                    last.members.push(m);
                }
                continue;
            }
        }
        groups.push(GroupWithMembers {
            group: Group {
                id,
                name,
                budget_basis: basis,
                budget_value: value,
            },
            members: member.into_iter().collect(),
        });
    }

    Ok(groups)
}

/// Create a new group. Returns the created group.
#[tauri::command]
pub fn create_group(
    state: State<'_, AppState>,
    name: String,
    budget_basis: Option<String>,
    budget_value: Option<f64>,
) -> Result<Group, String> {
    validate_name(&name)?;
    validate_budget(&budget_basis, &budget_value)?;

    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let created_at = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO project_groups (name, budget_basis, budget_value, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![name, budget_basis, budget_value, created_at],
    )
    .map_err(|e| map_unique_err(e, &name))?;

    let id = conn.last_insert_rowid();
    Ok(Group {
        id,
        name,
        budget_basis,
        budget_value,
    })
}

/// Update an existing group's name and/or budget.
#[tauri::command]
pub fn update_group(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    budget_basis: Option<String>,
    budget_value: Option<f64>,
) -> Result<(), String> {
    validate_name(&name)?;
    validate_budget(&budget_basis, &budget_value)?;

    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let rows_affected = conn
        .execute(
            "UPDATE project_groups
             SET name = ?1, budget_basis = ?2, budget_value = ?3
             WHERE id = ?4",
            params![name, budget_basis, budget_value, id],
        )
        .map_err(|e| map_unique_err(e, &name))?;

    if rows_affected == 0 {
        return Err(format!("No group found with id {id}"));
    }
    Ok(())
}

/// Delete a group (members fall back to "otros" via ON DELETE CASCADE).
/// Also removes the `budget_alert:<id>` meta key in the same transaction (D17).
#[tauri::command]
pub fn delete_group(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM project_groups WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    // Clean up alert state so a recreated group (same id via AUTOINCREMENT is
    // prevented, but as an extra guard) starts with fired=false.
    let meta_key = format!("budget_alert:{id}");
    tx.execute("DELETE FROM meta WHERE key = ?1", params![meta_key])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

/// Assign a project to a group (upsert — replaces any existing membership).
#[tauri::command]
pub fn assign_project(
    state: State<'_, AppState>,
    project_name: String,
    group_id: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO project_group_members (project_name, group_id)
         VALUES (?1, ?2)
         ON CONFLICT(project_name) DO UPDATE SET group_id = excluded.group_id",
        params![project_name, group_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove a project from any group (it returns to "otros").
#[tauri::command]
pub fn unassign_project(state: State<'_, AppState>, project_name: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM project_group_members WHERE project_name = ?1",
        params![project_name],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// List all distinct `project_name` values known from usage events.
#[tauri::command]
pub fn list_project_names(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT DISTINCT project_name FROM usage_events ORDER BY project_name")
        .map_err(|e| e.to_string())?;
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e: rusqlite::Error| e.to_string())?;
    Ok(names)
}

// ---------------------------------------------------------------------------
// Budget computation (D8)
// ---------------------------------------------------------------------------

/// Compute group budgets from the current window. Pure read — no network I/O.
///
/// `conn` must already be locked by the caller when called from the poll path
/// (D17). The `state` argument is used only to read `last_limits` (different
/// lock), which is cloned and dropped before this function touches `conn`.
pub fn compute_group_budgets(conn: &Connection, state: &AppState) -> GroupBudgetsSnapshot {
    // --- 1. Determine window (D2 / D12) ---
    // utilization: Some(f64) in session mode, None in rolling (used for session_weighted_pct).
    let (origin, window_start, session_resets_at, session_utilization) =
        match current_session_window(state) {
            Some((resets_at, ws, util)) => ("session".to_owned(), ws, Some(resets_at), Some(util)),
            None => {
                let ws =
                    (Utc::now() - Duration::hours(5)).to_rfc3339_opts(SecondsFormat::Millis, true);
                ("rolling".to_owned(), ws, None, None)
            }
        };
    let _ = session_resets_at; // used by evaluate_group_alerts, not here

    // --- 2. Load all defined groups (materialise with 0 first — D17) ---
    let groups: Vec<Group> = {
        let mut stmt = match conn
            .prepare("SELECT id, name, budget_basis, budget_value FROM project_groups ORDER BY id")
        {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("compute_group_budgets: {e}");
                return GroupBudgetsSnapshot {
                    rows: vec![],
                    window_start,
                    origin,
                };
            }
        };
        stmt.query_map([], |row| {
            Ok(Group {
                id: row.get(0)?,
                name: row.get(1)?,
                budget_basis: row.get(2)?,
                budget_value: row.get(3)?,
            })
        })
        .and_then(|mapped| mapped.collect::<rusqlite::Result<Vec<_>>>())
        .unwrap_or_default()
    };

    // --- 3. Load memberships into a HashMap (one SELECT — D17) ---
    let membership: std::collections::HashMap<String, i64> = {
        let mut stmt =
            match conn.prepare("SELECT project_name, group_id FROM project_group_members") {
                Ok(s) => s,
                Err(_) => {
                    return GroupBudgetsSnapshot {
                        rows: vec![],
                        window_start,
                        origin,
                    };
                }
            };
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .and_then(|mapped| mapped.collect::<rusqlite::Result<Vec<_>>>())
        .unwrap_or_default()
        .into_iter()
        .collect()
    };

    // --- 4. Query cost per project in the window (D12: bare column compare) ---
    let project_costs: Vec<(String, f64)> = {
        let mut stmt = match conn.prepare(
            "SELECT project_name, SUM(cost) FROM usage_events WHERE timestamp >= ?1 GROUP BY project_name",
        ) {
            Ok(s) => s,
            Err(_) => {
                return GroupBudgetsSnapshot {
                    rows: vec![],
                    window_start,
                    origin,
                };
            }
        };
        stmt.query_map(params![window_start], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .and_then(|mapped| mapped.collect::<rusqlite::Result<Vec<_>>>())
        .unwrap_or_default()
    };

    // --- 5. Accumulate costs into groups + "otros" ---
    // group_id → accumulated cost
    let mut group_cost: std::collections::HashMap<i64, f64> =
        groups.iter().map(|g| (g.id, 0.0)).collect();
    let mut otros_cost: f64 = 0.0;

    for (project, cost) in &project_costs {
        if let Some(&gid) = membership.get(project) {
            *group_cost.entry(gid).or_insert(0.0) += cost;
        } else {
            otros_cost += cost;
        }
    }

    let total: f64 = project_costs.iter().map(|(_, c)| c).sum();

    // --- 6. Build rows ---
    let share_pct = |cost: f64| -> f64 {
        if total > 0.0 {
            cost / total * 100.0
        } else {
            0.0
        }
    };

    // session_weighted_pct = local_cost_share_pct × session.utilization / 100
    // Only available in session mode. In rolling mode → None.
    let weighted_pct =
        |pct: f64| -> Option<f64> { session_utilization.map(|util| pct * util / 100.0) };

    let mut rows: Vec<GroupBudgetRow> = groups
        .iter()
        .map(|g| {
            let cost = group_cost.get(&g.id).copied().unwrap_or(0.0);
            let pct = share_pct(cost);
            let sw_pct = weighted_pct(pct);
            // share cap now measures the session-weighted estimate (None in rolling).
            let measured = match g.budget_basis.as_deref() {
                Some("share") => sw_pct,
                Some("usd") => Some(cost),
                _ => None,
            };
            GroupBudgetRow {
                group_id: Some(g.id),
                name: g.name.clone(),
                budget_basis: g.budget_basis.clone(),
                budget_value: g.budget_value,
                window_cost_usd: cost,
                local_cost_share_pct: pct,
                session_weighted_pct: sw_pct,
                measured_value: measured,
            }
        })
        .collect();

    // Sort defined groups by cost desc.
    rows.sort_by(|a, b| {
        b.window_cost_usd
            .partial_cmp(&a.window_cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Append "otros" last (D5).
    if otros_cost > 0.0 || !groups.is_empty() {
        // Always include "otros" row so the frontend can show unassigned projects.
        // But only include if there is actually unassigned activity, OR if there
        // are no groups at all (to avoid an empty list with just a zero "otros").
        if otros_cost > 0.0 {
            let otros_pct = share_pct(otros_cost);
            rows.push(GroupBudgetRow {
                group_id: None,
                name: "otros".to_owned(),
                budget_basis: None,
                budget_value: None,
                window_cost_usd: otros_cost,
                local_cost_share_pct: otros_pct,
                session_weighted_pct: weighted_pct(otros_pct),
                measured_value: None,
            });
        }
    }

    GroupBudgetsSnapshot {
        rows,
        window_start,
        origin,
    }
}

/// Query group budgets (Tauri command). No network I/O.
#[tauri::command]
pub fn query_group_budgets(state: State<'_, AppState>) -> Result<GroupBudgetsSnapshot, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    Ok(compute_group_budgets(&conn, &state))
}

// ---------------------------------------------------------------------------
// Group alerts (D9)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct GroupAlertState {
    last_resets_at: String,
    fired: bool,
}

/// Evaluate group budget alerts and return notification texts.
///
/// Must be called with `conn` already locked (mirror of `evaluate_alerts`).
/// Only runs evaluation when `origin == "session"` (D9).
pub fn evaluate_group_alerts(
    conn: &Connection,
    snapshot: &GroupBudgetsSnapshot,
    session_resets_at: Option<&str>,
) -> Vec<String> {
    // Rolling window → no alerts (D9).
    if snapshot.origin != "session" {
        return vec![];
    }
    let resets_at = match session_resets_at {
        Some(r) => r,
        None => return vec![],
    };

    let mut notifications = Vec::new();

    for row in &snapshot.rows {
        // Skip "otros" and groups without a cap.
        let gid = match row.group_id {
            Some(id) => id,
            None => continue,
        };
        let basis = match row.budget_basis.as_deref() {
            Some(b) => b,
            None => continue,
        };
        let cap = match row.budget_value {
            Some(v) => v,
            None => continue,
        };
        let measured = match row.measured_value {
            Some(v) => v,
            None => continue,
        };

        let meta_key = format!("budget_alert:{gid}");
        let mut alert_state: GroupAlertState = db::meta_get(conn, &meta_key)
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or(GroupAlertState {
                last_resets_at: String::new(),
                fired: false,
            });

        // Reset on new session window.
        if alert_state.last_resets_at != resets_at {
            alert_state.last_resets_at = resets_at.to_owned();
            alert_state.fired = false;
        }

        if measured >= cap && !alert_state.fired {
            let text = match basis {
                "share" => format!(
                    "Grupo «{}»: {:.1}% de tu sesión (est., tope {:.1}%)",
                    row.name, measured, cap
                ),
                "usd" => format!(
                    "Grupo «{}»: ${:.2} en la sesión (tope ${:.2})",
                    row.name, measured, cap
                ),
                _ => continue,
            };
            notifications.push(text);
            alert_state.fired = true;
        }

        // Persist updated state.
        if let Ok(json) = serde_json::to_string(&alert_state) {
            let _ = db::meta_set(conn, &meta_key, &json);
        }
    }

    notifications
}

/// Called from `run_limits_and_emit` under one `conn` lock (D17).
///
/// Returns notification texts; the caller decides whether to emit them based on
/// `alerts_muted`.
pub fn run_group_alerts(
    conn: &Connection,
    state: &AppState,
    session_resets_at: Option<&str>,
) -> Vec<String> {
    let snapshot = compute_group_budgets(conn, state);
    evaluate_group_alerts(conn, &snapshot, session_resets_at)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, UsageEventRow};
    use crate::limits::{LimitsSnapshot, LimitsStatus, Window};

    fn test_conn() -> Connection {
        db::open_at(":memory:").expect("in-memory db")
    }

    fn make_state_with_session(resets_at: &str) -> AppState {
        let conn = test_conn();
        let snapshot = LimitsSnapshot {
            session: Some(Window {
                label: None,
                utilization: 42.0,
                resets_at: resets_at.to_owned(),
            }),
            weekly: None,
            weekly_by_model: vec![],
            fetched_at: "2026-07-05T10:00:00Z".to_owned(),
            status: LimitsStatus::Ok,
        };
        AppState {
            conn: std::sync::Mutex::new(conn),
            last_limits: std::sync::Mutex::new(Some(snapshot)),
        }
    }

    fn make_state_no_session() -> AppState {
        let conn = test_conn();
        AppState {
            conn: std::sync::Mutex::new(conn),
            last_limits: std::sync::Mutex::new(None),
        }
    }

    fn insert_event(conn: &Connection, key: &str, project: &str, cost: f64, ts: &str) {
        db::insert_event(
            conn,
            &UsageEventRow {
                dedup_key: key,
                session_id: "s",
                project_path: "/p",
                project_name: project,
                model: "claude-sonnet-4-6",
                input_tokens: 100,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 110,
                cost,
                timestamp: ts,
                git_branch: None,
                ingested_at: ts,
            },
        )
        .unwrap();
    }

    // -----------------------------------------------------------------------
    // Schema v5 migration
    // -----------------------------------------------------------------------

    #[test]
    fn test_v5_tables_exist_after_migration() {
        let conn = test_conn();
        // Both tables must exist after migration.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_groups", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_group_members", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
        let version: String = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(version, "6");
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_budget_share_range() {
        assert!(validate_budget(&Some("share".into()), &Some(30.0)).is_ok());
        assert!(validate_budget(&Some("share".into()), &Some(100.0)).is_ok());
        assert!(validate_budget(&Some("share".into()), &Some(0.0)).is_err());
        assert!(validate_budget(&Some("share".into()), &Some(100.01)).is_err());
        assert!(validate_budget(&Some("share".into()), &Some(-1.0)).is_err());
    }

    #[test]
    fn test_validate_budget_usd_range() {
        assert!(validate_budget(&Some("usd".into()), &Some(0.01)).is_ok());
        assert!(validate_budget(&Some("usd".into()), &Some(0.0)).is_err());
        assert!(validate_budget(&Some("usd".into()), &Some(-1.0)).is_err());
    }

    #[test]
    fn test_validate_budget_none() {
        assert!(validate_budget(&None, &None).is_ok());
    }

    #[test]
    fn test_validate_budget_invalid_basis() {
        assert!(validate_budget(&Some("percent".into()), &Some(50.0)).is_err());
    }

    // -----------------------------------------------------------------------
    // ON DELETE CASCADE
    // -----------------------------------------------------------------------

    #[test]
    fn test_on_delete_cascade_removes_members() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G1', NULL, NULL, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let gid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO project_group_members (project_name, group_id) VALUES ('proj-a', ?1)",
            params![gid],
        )
        .unwrap();
        conn.execute("DELETE FROM project_groups WHERE id = ?1", params![gid])
            .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_group_members", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "members must be cascade-deleted with group");
    }

    // -----------------------------------------------------------------------
    // 1-project-1-group uniqueness
    // -----------------------------------------------------------------------

    #[test]
    fn test_assign_project_upsert() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G1', NULL, NULL, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let gid1 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G2', NULL, NULL, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let gid2 = conn.last_insert_rowid();

        // Assign to G1.
        conn.execute(
            "INSERT INTO project_group_members (project_name, group_id) VALUES ('proj-x', ?1)",
            params![gid1],
        )
        .unwrap();
        // Re-assign to G2 (upsert).
        conn.execute(
            "INSERT INTO project_group_members (project_name, group_id) VALUES ('proj-x', ?1)
             ON CONFLICT(project_name) DO UPDATE SET group_id = excluded.group_id",
            params![gid2],
        )
        .unwrap();

        let stored_gid: i64 = conn
            .query_row(
                "SELECT group_id FROM project_group_members WHERE project_name = 'proj-x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored_gid, gid2, "project must be in G2 after reassignment");
    }

    // -----------------------------------------------------------------------
    // compute_group_budgets — local share
    // -----------------------------------------------------------------------

    #[test]
    fn test_compute_total_zero() {
        // Empty window → all pct == 0, no rows.
        let conn = test_conn();
        let state = AppState {
            conn: std::sync::Mutex::new(test_conn()),
            last_limits: std::sync::Mutex::new(None),
        };
        // Insert a group with no events in the window.
        conn.execute(
            "INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G', NULL, NULL, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let snap = compute_group_budgets(&conn, &state);
        for row in &snap.rows {
            assert_eq!(row.local_cost_share_pct, 0.0, "no events → pct must be 0");
        }
    }

    #[test]
    fn test_compute_local_share_pct() {
        // G1=$5, G2=$12, unassigned=$3 → total=$20
        // Use a session-anchored window: resets_at = 2026-07-05T15:00:00Z
        // → window_start = 2026-07-05T10:00:00.000Z
        // Events placed at 2026-07-05T12:00:00.000Z (inside window).
        let resets_at = "2026-07-05T15:00:00Z";
        let conn = test_conn();
        // Groups.
        conn.execute("INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('ClienteA', 'share', 30.0, '2026-01-01T00:00:00Z')", []).unwrap();
        let g1 = conn.last_insert_rowid();
        conn.execute("INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('Experimentos', 'usd', 15.0, '2026-01-01T00:00:00Z')", []).unwrap();
        let g2 = conn.last_insert_rowid();
        // Members.
        conn.execute(
            "INSERT INTO project_group_members VALUES ('proj-a', ?1)",
            params![g1],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO project_group_members VALUES ('proj-b', ?1)",
            params![g2],
        )
        .unwrap();

        // Events — inside the session window.
        insert_event(&conn, "e1", "proj-a", 5.0, "2026-07-05T12:00:00.000Z");
        insert_event(&conn, "e2", "proj-b", 12.0, "2026-07-05T12:01:00.000Z");
        insert_event(&conn, "e3", "proj-c", 3.0, "2026-07-05T12:02:00.000Z"); // unassigned

        let state = make_state_with_session(resets_at);
        let snap = compute_group_budgets(&conn, &state);

        let g1_row = snap.rows.iter().find(|r| r.name == "ClienteA").unwrap();
        let g2_row = snap.rows.iter().find(|r| r.name == "Experimentos").unwrap();
        let otros_row = snap.rows.iter().find(|r| r.name == "otros").unwrap();

        assert!((g1_row.window_cost_usd - 5.0).abs() < 1e-9);
        assert!((g2_row.window_cost_usd - 12.0).abs() < 1e-9);
        assert!((otros_row.window_cost_usd - 3.0).abs() < 1e-9);

        assert!(
            (g1_row.local_cost_share_pct - 25.0).abs() < 1e-6,
            "ClienteA should be 25%"
        );
        assert!(
            (g2_row.local_cost_share_pct - 60.0).abs() < 1e-6,
            "Experimentos should be 60%"
        );
        assert!(
            (otros_row.local_cost_share_pct - 15.0).abs() < 1e-6,
            "otros should be 15%"
        );

        // session_weighted_pct: local_cost_share_pct × session.utilization (42%) / 100
        // ClienteA: 25% × 42% / 100 = 10.5%
        assert!(
            (g1_row.session_weighted_pct.unwrap() - 10.5).abs() < 1e-6,
            "ClienteA session_weighted_pct should be 10.5%"
        );
        // Experimentos: 60% × 42% / 100 = 25.2%
        assert!(
            (g2_row.session_weighted_pct.unwrap() - 25.2).abs() < 1e-6,
            "Experimentos session_weighted_pct should be 25.2%"
        );
        // otros: 15% × 42% / 100 = 6.3%
        assert!(
            (otros_row.session_weighted_pct.unwrap() - 6.3).abs() < 1e-6,
            "otros session_weighted_pct should be 6.3%"
        );

        // measured_value: share → session_weighted_pct (est.), usd → cost.
        assert!((g1_row.measured_value.unwrap() - 10.5).abs() < 1e-6);
        assert!((g2_row.measured_value.unwrap() - 12.0).abs() < 1e-9);
        assert!(otros_row.measured_value.is_none());

        // local_cost_share_pct must remain pure (no utilization multiplication).
        assert!(
            (g1_row.local_cost_share_pct - 25.0).abs() < 1e-6,
            "local_cost_share_pct must not be multiplied by utilization"
        );
        assert!(
            (g2_row.local_cost_share_pct - 60.0).abs() < 1e-6,
            "local_cost_share_pct must not be multiplied by utilization"
        );
    }

    #[test]
    fn test_group_defined_no_activity_appears_at_zero() {
        let conn = test_conn();
        conn.execute("INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('Idle', NULL, NULL, '2026-01-01T00:00:00Z')", []).unwrap();
        let state = AppState {
            conn: std::sync::Mutex::new(test_conn()),
            last_limits: std::sync::Mutex::new(None),
        };
        let snap = compute_group_budgets(&conn, &state);
        let row = snap.rows.iter().find(|r| r.name == "Idle").unwrap();
        assert_eq!(row.window_cost_usd, 0.0);
        assert_eq!(row.local_cost_share_pct, 0.0);
    }

    // -----------------------------------------------------------------------
    // Session vs rolling window
    // -----------------------------------------------------------------------

    #[test]
    fn test_session_window_excludes_old_events() {
        // resets_at = 2026-07-05T15:00:00Z → window_start = 2026-07-05T10:00:00.000Z
        let resets_at = "2026-07-05T15:00:00Z";
        let conn = test_conn();
        // Event inside window.
        insert_event(&conn, "in", "p", 5.0, "2026-07-05T12:00:00.000Z");
        // Event outside window (before window_start).
        insert_event(&conn, "out", "p", 99.0, "2026-07-05T09:59:59.999Z");

        let state = make_state_with_session(resets_at);
        let snap = compute_group_budgets(&conn, &state);
        // Only the "in" event should appear → total = 5.
        let total: f64 = snap.rows.iter().map(|r| r.window_cost_usd).sum();
        assert!(
            (total - 5.0).abs() < 1e-9,
            "only in-window event should be counted; total={total}"
        );
        assert_eq!(snap.origin, "session");
    }

    #[test]
    fn test_rolling_fallback_no_session() {
        let conn = test_conn();
        let state = make_state_no_session();
        let snap = compute_group_budgets(&conn, &state);
        assert_eq!(snap.origin, "rolling");
    }

    #[test]
    fn test_no_session_utilization_multiplication_in_local_share() {
        // Verify that local_cost_share_pct is a pure ratio (NOT multiplied by utilization).
        // session.utilization = 42.0; the ONLY place utilization enters is session_weighted_pct.
        let resets_at = "2026-07-05T15:00:00Z";
        let conn = test_conn();
        conn.execute("INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G', 'share', 50.0, '2026-01-01T00:00:00Z')", []).unwrap();
        let gid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO project_group_members VALUES ('p', ?1)",
            params![gid],
        )
        .unwrap();
        insert_event(&conn, "e1", "p", 10.0, "2026-07-05T12:00:00.000Z");

        let state = make_state_with_session(resets_at); // utilization = 42.0
        let snap = compute_group_budgets(&conn, &state);
        let row = snap.rows.iter().find(|r| r.name == "G").unwrap();

        // local_cost_share_pct must be 100% (only group, all cost) — NOT multiplied by 0.42.
        assert!(
            (row.local_cost_share_pct - 100.0).abs() < 1e-6,
            "local_cost_share_pct must be pure ratio (100%), got {}",
            row.local_cost_share_pct
        );

        // session_weighted_pct = 100% × 42% / 100 = 42%.
        assert!(
            (row.session_weighted_pct.unwrap() - 42.0).abs() < 1e-6,
            "session_weighted_pct should be 42.0%, got {:?}",
            row.session_weighted_pct
        );

        // measured_value for share = session_weighted_pct = 42.0% (not 100%).
        assert!(
            (row.measured_value.unwrap() - 42.0).abs() < 1e-6,
            "measured_value(share) must be session_weighted_pct (42.0%), got {:?}",
            row.measured_value
        );
    }

    #[test]
    fn test_share_cap_87pct_local_34pct_session_under_30pct_cap() {
        // Regression: local share 87% × session utilization 34% ⇒ weighted ≈ 29.58%.
        // A 30% share cap must NOT fire (29.58 < 30).
        // This is the exact user scenario that prompted D20.
        let resets_at = "2026-07-05T15:00:00Z";
        let conn = test_conn();
        conn.execute(
            "INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('MiGrupo', 'share', 30.0, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let gid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO project_group_members VALUES ('proj-main', ?1)",
            params![gid],
        )
        .unwrap();

        // Group cost = 87, total = 100 → local share = 87%; other project = 13.
        insert_event(&conn, "e1", "proj-main", 87.0, "2026-07-05T12:00:00.000Z");
        insert_event(&conn, "e2", "proj-other", 13.0, "2026-07-05T12:01:00.000Z");

        // Build state with session.utilization = 34.0.
        let snapshot = crate::limits::LimitsSnapshot {
            session: Some(crate::limits::Window {
                label: None,
                utilization: 34.0,
                resets_at: resets_at.to_owned(),
            }),
            weekly: None,
            weekly_by_model: vec![],
            fetched_at: "2026-07-05T10:00:00Z".to_owned(),
            status: crate::limits::LimitsStatus::Ok,
        };
        let state_conn = test_conn();
        let state = AppState {
            conn: std::sync::Mutex::new(state_conn),
            last_limits: std::sync::Mutex::new(Some(snapshot)),
        };

        let snap = compute_group_budgets(&conn, &state);
        let row = snap.rows.iter().find(|r| r.name == "MiGrupo").unwrap();

        // local_cost_share_pct = 87%
        assert!(
            (row.local_cost_share_pct - 87.0).abs() < 1e-6,
            "local_cost_share_pct should be 87.0%, got {}",
            row.local_cost_share_pct
        );

        // session_weighted_pct = 87% × 34% / 100 = 29.58%
        let sw = row.session_weighted_pct.unwrap();
        assert!(
            (sw - 29.58).abs() < 0.01,
            "session_weighted_pct should be ≈29.58%, got {sw}"
        );

        // measured_value(share) = 29.58% < 30% cap → alert must NOT fire.
        let mv = row.measured_value.unwrap();
        assert!(
            (mv - 29.58).abs() < 0.01,
            "measured_value(share) should be ≈29.58%, got {mv}"
        );
        assert!(
            mv < 30.0,
            "29.58% must be under 30% cap — alert must not fire"
        );

        // Verify alert does not fire.
        let notes = evaluate_group_alerts(&conn, &snap, Some(resets_at));
        assert!(
            notes.is_empty(),
            "87% local × 34% session = 29.58% must NOT fire a 30% cap alert; got: {notes:?}"
        );
    }

    #[test]
    fn test_rolling_origin_has_no_session_weighted_pct() {
        // In rolling mode: session_weighted_pct must be None for all rows,
        // and measured_value for share must also be None (no share-cap alerts in rolling).
        // Use a current timestamp so the event falls inside the rolling window (now−5h).
        let conn = test_conn();
        conn.execute(
            "INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G', 'share', 30.0, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let gid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO project_group_members VALUES ('p', ?1)",
            params![gid],
        )
        .unwrap();
        // Use now() so the event is always inside the rolling window (now − 5h).
        let now_ts = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        insert_event(&conn, "e1", "p", 10.0, &now_ts);

        let state = make_state_no_session();
        let snap = compute_group_budgets(&conn, &state);
        assert_eq!(snap.origin, "rolling");

        let row = snap.rows.iter().find(|r| r.name == "G").unwrap();
        assert!(
            row.session_weighted_pct.is_none(),
            "rolling mode must not produce session_weighted_pct"
        );
        assert!(
            row.measured_value.is_none(),
            "rolling mode: measured_value(share) must be None (no share-cap alerts)"
        );
        // local_cost_share_pct is still valid and 100% (only activity).
        assert!(
            (row.local_cost_share_pct - 100.0).abs() < 1e-6,
            "local_cost_share_pct must be correct even in rolling mode"
        );
    }

    // -----------------------------------------------------------------------
    // Window boundary tests (D12)
    // -----------------------------------------------------------------------

    #[test]
    fn test_window_boundary_exact_inclusion() {
        // resets_at = 2026-07-05T15:00:00Z → window_start = 2026-07-05T10:00:00.000Z
        let resets_at = "2026-07-05T15:00:00Z";
        let conn = test_conn();
        // Exactly at window_start — must be included (>=).
        insert_event(&conn, "exact", "p", 1.0, "2026-07-05T10:00:00.000Z");
        // 1ms before — must be excluded.
        // "2026-07-05T09:59:59.999Z" < "2026-07-05T10:00:00.000Z" lexicographically.
        insert_event(&conn, "before_1ms", "p", 99.0, "2026-07-05T09:59:59.999Z");
        // 1s before — must be excluded.
        insert_event(&conn, "before_1s", "p", 99.0, "2026-07-05T09:59:59.000Z");

        let state = make_state_with_session(resets_at);
        let snap = compute_group_budgets(&conn, &state);
        let total: f64 = snap.rows.iter().map(|r| r.window_cost_usd).sum();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "only the exact-boundary event should be included; total={total}"
        );
    }

    #[test]
    fn test_resets_at_plus00_00_parses() {
        // resets_at in +00:00 form must parse and produce a correct window.
        let resets_at = "2026-07-05T15:00:00+00:00";
        let conn = test_conn();
        insert_event(&conn, "in", "p", 7.0, "2026-07-05T12:00:00.000Z");
        insert_event(&conn, "out", "p", 99.0, "2026-07-05T09:59:59.999Z");

        let state = make_state_with_session(resets_at);
        let snap = compute_group_budgets(&conn, &state);
        assert_eq!(snap.origin, "session", "+00:00 must parse as session");
        let total: f64 = snap.rows.iter().map(|r| r.window_cost_usd).sum();
        assert!(
            (total - 7.0).abs() < 1e-9,
            "+00:00 resets_at: only in-window event; total={total}"
        );
    }

    #[test]
    fn test_invalid_resets_at_falls_back_to_rolling() {
        // An invalid resets_at must produce rolling origin.
        let state = make_state_with_session("NOT_A_DATE");
        let conn = test_conn();
        let snap = compute_group_budgets(&conn, &state);
        // current_session_window returns None on parse failure → rolling.
        assert_eq!(snap.origin, "rolling");
    }

    // -----------------------------------------------------------------------
    // Alert state machine
    // -----------------------------------------------------------------------

    fn make_snapshot_with_group(
        group_id: i64,
        name: &str,
        basis: &str,
        cap: f64,
        measured: f64,
    ) -> GroupBudgetsSnapshot {
        // For share: measured is the session_weighted_pct; local_cost_share_pct is set
        // to a plausible raw value (measured / 0.5 assuming 50% util) for test purposes.
        // For usd: measured is the cost.
        let local_pct = if basis == "share" {
            measured / 0.5
        } else {
            0.0
        };
        let sw_pct = if basis == "share" {
            Some(measured)
        } else {
            None
        };
        GroupBudgetsSnapshot {
            rows: vec![GroupBudgetRow {
                group_id: Some(group_id),
                name: name.to_owned(),
                budget_basis: Some(basis.to_owned()),
                budget_value: Some(cap),
                window_cost_usd: if basis == "usd" { measured } else { 0.0 },
                local_cost_share_pct: local_pct,
                session_weighted_pct: sw_pct,
                measured_value: Some(measured),
            }],
            window_start: "2026-07-05T10:00:00.000Z".to_owned(),
            origin: "session".to_owned(),
        }
    }

    #[test]
    fn test_alert_share_cross() {
        let conn = test_conn();
        let snap = make_snapshot_with_group(1, "ClienteA", "share", 30.0, 31.0);
        let notes = evaluate_group_alerts(&conn, &snap, Some("2026-07-05T15:00:00Z"));
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].contains("31.0%"),
            "notification must include measured value"
        );
        assert!(notes[0].contains("30.0%"), "notification must include cap");
        assert!(notes[0].contains("ClienteA"));
        // Notification must say "de tu sesión (est.," to reflect the weighted metric.
        assert!(
            notes[0].contains("de tu sesión (est.,"),
            "share notification must mention session estimate; got: {}",
            notes[0]
        );
    }

    #[test]
    fn test_alert_usd_cross() {
        let conn = test_conn();
        let snap = make_snapshot_with_group(2, "ClienteB", "usd", 2.0, 2.10);
        let notes = evaluate_group_alerts(&conn, &snap, Some("2026-07-05T15:00:00Z"));
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].contains("$2.10"),
            "notification must include measured cost"
        );
        assert!(notes[0].contains("$2.00"), "notification must include cap");
    }

    #[test]
    fn test_alert_no_re_fire_same_window() {
        let conn = test_conn();
        let snap = make_snapshot_with_group(3, "G", "share", 30.0, 31.0);
        let resets_at = "2026-07-05T15:00:00Z";
        let first = evaluate_group_alerts(&conn, &snap, Some(resets_at));
        let second = evaluate_group_alerts(&conn, &snap, Some(resets_at));
        assert_eq!(first.len(), 1, "first cross must fire");
        assert_eq!(second.len(), 0, "must not re-fire in same window");
    }

    #[test]
    fn test_alert_resets_on_new_resets_at() {
        let conn = test_conn();
        let snap = make_snapshot_with_group(4, "G", "share", 30.0, 31.0);
        evaluate_group_alerts(&conn, &snap, Some("2026-07-05T15:00:00Z"));
        let snap2 = make_snapshot_with_group(4, "G", "share", 30.0, 31.0);
        let after = evaluate_group_alerts(&conn, &snap2, Some("2026-07-05T20:00:00Z"));
        assert_eq!(after.len(), 1, "new resets_at must reset fired state");
    }

    #[test]
    fn test_alert_no_alerts_in_rolling() {
        let conn = test_conn();
        let mut snap = make_snapshot_with_group(5, "G", "share", 30.0, 31.0);
        snap.origin = "rolling".to_owned();
        let notes = evaluate_group_alerts(&conn, &snap, Some("2026-07-05T15:00:00Z"));
        assert!(notes.is_empty(), "rolling mode must not fire alerts");
    }

    #[test]
    fn test_alert_no_cap_group_never_alerts() {
        let conn = test_conn();
        let snap = GroupBudgetsSnapshot {
            rows: vec![GroupBudgetRow {
                group_id: Some(6),
                name: "Interno".to_owned(),
                budget_basis: None,
                budget_value: None,
                window_cost_usd: 100.0,
                local_cost_share_pct: 100.0,
                session_weighted_pct: Some(42.0),
                measured_value: None,
            }],
            window_start: "2026-07-05T10:00:00.000Z".to_owned(),
            origin: "session".to_owned(),
        };
        let notes = evaluate_group_alerts(&conn, &snap, Some("2026-07-05T15:00:00Z"));
        assert!(notes.is_empty(), "no-cap group must not alert");
    }

    #[test]
    fn test_alert_otros_never_alerts() {
        let conn = test_conn();
        let snap = GroupBudgetsSnapshot {
            rows: vec![GroupBudgetRow {
                group_id: None, // "otros"
                name: "otros".to_owned(),
                budget_basis: Some("share".to_owned()),
                budget_value: Some(10.0),
                window_cost_usd: 50.0,
                local_cost_share_pct: 100.0,
                session_weighted_pct: Some(42.0),
                measured_value: Some(42.0),
            }],
            window_start: "2026-07-05T10:00:00.000Z".to_owned(),
            origin: "session".to_owned(),
        };
        let notes = evaluate_group_alerts(&conn, &snap, Some("2026-07-05T15:00:00Z"));
        assert!(notes.is_empty(), "otros bucket must never alert");
    }

    #[test]
    fn test_delete_group_clears_alert_state() {
        let conn = test_conn();
        // Insert group and alert state manually.
        conn.execute(
            "INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G', 'share', 30.0, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let gid = conn.last_insert_rowid();
        let meta_key = format!("budget_alert:{gid}");
        db::meta_set(&conn, &meta_key, r#"{"last_resets_at":"x","fired":true}"#).unwrap();

        // Delete the group in a transaction (mimics delete_group command).
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute("DELETE FROM project_groups WHERE id = ?1", params![gid])
            .unwrap();
        tx.execute("DELETE FROM meta WHERE key = ?1", params![meta_key])
            .unwrap();
        tx.commit().unwrap();

        let val = db::meta_get(&conn, &meta_key).unwrap();
        assert!(val.is_none(), "alert meta must be deleted with the group");

        // A new group should start with fired=false (no stale state).
        conn.execute(
            "INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G2', 'share', 30.0, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let new_gid = conn.last_insert_rowid();
        let new_key = format!("budget_alert:{new_gid}");
        let state = db::meta_get(&conn, &new_key).unwrap();
        assert!(state.is_none(), "new group must have no stale alert state");
    }

    #[test]
    fn test_unpriced_model_contributes_zero_cost() {
        // A model with cost=0 (unpriced) must not trip a usd cap.
        let conn = test_conn();
        conn.execute("INSERT INTO project_groups (name, budget_basis, budget_value, created_at) VALUES ('G', 'usd', 1.0, '2026-01-01T00:00:00Z')", []).unwrap();
        let gid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO project_group_members VALUES ('proj', ?1)",
            params![gid],
        )
        .unwrap();

        // Insert event with cost=0 (unpriced model).
        db::insert_event(
            &conn,
            &UsageEventRow {
                dedup_key: "unpriced",
                session_id: "s",
                project_path: "/p",
                project_name: "proj",
                model: "unknown-model-xyz",
                input_tokens: 1_000_000,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 1_000_000,
                cost: 0.0,
                timestamp: "2026-07-05T12:00:00.000Z",
                git_branch: None,
                ingested_at: "2026-07-05T12:00:00.000Z",
            },
        )
        .unwrap();

        let state = make_state_with_session("2026-07-05T15:00:00Z");
        let snap = compute_group_budgets(&conn, &state);
        let row = snap.rows.iter().find(|r| r.group_id == Some(gid)).unwrap();
        assert_eq!(
            row.window_cost_usd, 0.0,
            "unpriced model contributes 0 cost"
        );

        let notes = evaluate_group_alerts(&conn, &snap, Some("2026-07-05T15:00:00Z"));
        assert!(notes.is_empty(), "zero cost must not trip usd cap");
    }
}
