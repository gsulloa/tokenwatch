//! Claude usage-limits capability.
//!
//! Reads the OAuth token from the macOS Keychain, queries the Anthropic
//! `/api/oauth/usage` endpoint, parses the response into a [`LimitsSnapshot`],
//! and manages threshold-based macOS notifications per window.
//!
//! Exposes three Tauri commands to the frontend:
//! - `query_limits`       — immediate fetch + snapshot
//! - `get_alerts_muted`   — read the mute flag
//! - `set_alerts_muted`   — write the mute flag

use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::{db, usage::AppState};

// ---------------------------------------------------------------------------
// Public event name
// ---------------------------------------------------------------------------

pub const LIMITS_UPDATED_EVENT: &str = "limits-updated";

// ---------------------------------------------------------------------------
// Poll interval
// ---------------------------------------------------------------------------

pub const LIMITS_POLL_SECS: u64 = 300;

// ---------------------------------------------------------------------------
// Threshold constants
// ---------------------------------------------------------------------------

const THRESHOLDS: [u8; 3] = [50, 70, 80];

// ---------------------------------------------------------------------------
// Public types — JSON contract with the frontend
// ---------------------------------------------------------------------------

/// A single utilization window (session, weekly, or weekly-scoped).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    /// Human-readable label, set for `weekly_scoped` windows (e.g. "Fable").
    pub label: Option<String>,
    /// Utilization as a percentage (0–100).
    pub utilization: f64,
    /// ISO 8601 timestamp when the window resets.
    pub resets_at: String,
}

/// Why limits are unavailable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LimitsStatus {
    Ok,
    Unavailable { reason: String },
}

/// Full snapshot returned by `query_limits` and emitted via `limits-updated`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsSnapshot {
    pub session: Option<Window>,
    pub weekly: Option<Window>,
    pub weekly_by_model: Vec<Window>,
    pub fetched_at: String,
    pub status: LimitsStatus,
}

impl LimitsSnapshot {
    fn unavailable(reason: &str) -> Self {
        LimitsSnapshot {
            session: None,
            weekly: None,
            weekly_by_model: vec![],
            fetched_at: Utc::now().to_rfc3339(),
            status: LimitsStatus::Unavailable {
                reason: reason.to_owned(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Internal API response shapes (tolerant parsing)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct ApiResponse {
    #[serde(default)]
    limits: Vec<ApiLimit>,
    five_hour: Option<ApiTopLevel>,
    seven_day: Option<ApiTopLevel>,
    seven_day_opus: Option<ApiTopLevel>,
    seven_day_sonnet: Option<ApiTopLevel>,
}

#[derive(Debug, Deserialize)]
struct ApiTopLevel {
    utilization: f64,
    resets_at: String,
}

#[derive(Debug, Deserialize)]
struct ApiLimit {
    kind: Option<String>,
    percent: Option<f64>,
    resets_at: Option<String>,
    scope: Option<ApiScope>,
}

#[derive(Debug, Deserialize)]
struct ApiScope {
    model: Option<ApiModel>,
}

#[derive(Debug, Deserialize)]
struct ApiModel {
    display_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Keychain reading
// ---------------------------------------------------------------------------

/// Reads the Claude Code OAuth token from the macOS Keychain.
///
/// Returns `Ok(token_string)` on success, or `Err(reason_str)` where the
/// reason maps to a [`LimitsStatus::Unavailable`] reason.
fn read_keychain_token() -> Result<String, String> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-w",
            "-s",
            "Claude Code-credentials",
        ])
        .output()
        .map_err(|e| {
            tracing::warn!("failed to run `security`: {e}");
            "not_signed_in".to_owned()
        })?;

    // `security` exits with code 44 when the item is not found; other non-zero
    // codes (e.g. 128) often mean permission denied.
    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        if code == 44 {
            return Err("not_signed_in".to_owned());
        }
        // Any other failure (permission denied, etc.)
        return Err("keychain_denied".to_owned());
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if raw.is_empty() {
        return Err("not_signed_in".to_owned());
    }

    // Parse the JSON blob stored in the Keychain item.
    let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        tracing::warn!("keychain JSON parse error: {e}");
        "parse".to_owned()
    })?;

    let access_token = parsed
        .get("claudeAiOauth")
        .and_then(|o| o.get("accessToken"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "parse".to_owned())?
        .to_owned();

    // Check expiry (epoch ms).
    if let Some(expires_at) = parsed
        .get("claudeAiOauth")
        .and_then(|o| o.get("expiresAt"))
        .and_then(|v| v.as_i64())
    {
        let now_ms = Utc::now().timestamp_millis();
        if expires_at < now_ms {
            return Err("expired".to_owned());
        }
    }

    Ok(access_token)
}

// ---------------------------------------------------------------------------
// HTTP fetch
// ---------------------------------------------------------------------------

async fn fetch_usage(token: &str) -> Result<LimitsSnapshot, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("limits network error: {e}");
            "network".to_owned()
        })?;

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        tracing::warn!("limits endpoint returned 429");
        return Err("http".to_owned());
    }
    if !status.is_success() {
        tracing::warn!("limits endpoint returned HTTP {status}");
        return Err("http".to_owned());
    }

    let api: ApiResponse = resp.json().await.map_err(|e| {
        tracing::warn!("limits parse error: {e}");
        "parse".to_owned()
    })?;

    parse_api_response(api)
}

fn parse_api_response(api: ApiResponse) -> Result<LimitsSnapshot, String> {
    let mut session: Option<Window> = None;
    let mut weekly: Option<Window> = None;
    let mut weekly_by_model: Vec<Window> = Vec::new();

    if !api.limits.is_empty() {
        for limit in &api.limits {
            let kind = match &limit.kind {
                Some(k) => k.as_str(),
                None => continue,
            };
            let utilization = match limit.percent {
                Some(p) => p,
                None => continue,
            };
            let resets_at = match &limit.resets_at {
                Some(r) => r.clone(),
                None => continue,
            };

            match kind {
                "session" => {
                    session = Some(Window {
                        label: None,
                        utilization,
                        resets_at,
                    });
                }
                "weekly_all" => {
                    weekly = Some(Window {
                        label: None,
                        utilization,
                        resets_at,
                    });
                }
                "weekly_scoped" => {
                    let label = limit
                        .scope
                        .as_ref()
                        .and_then(|s| s.model.as_ref())
                        .and_then(|m| m.display_name.clone());
                    weekly_by_model.push(Window {
                        label,
                        utilization,
                        resets_at,
                    });
                }
                _ => {}
            }
        }
    } else {
        // Fallback: use top-level fields.
        if let Some(fh) = api.five_hour {
            session = Some(Window {
                label: None,
                utilization: fh.utilization,
                resets_at: fh.resets_at,
            });
        }
        if let Some(sd) = api.seven_day {
            weekly = Some(Window {
                label: None,
                utilization: sd.utilization,
                resets_at: sd.resets_at,
            });
        }
        if let Some(opus) = api.seven_day_opus {
            weekly_by_model.push(Window {
                label: Some("Opus".to_owned()),
                utilization: opus.utilization,
                resets_at: opus.resets_at,
            });
        }
        if let Some(sonnet) = api.seven_day_sonnet {
            weekly_by_model.push(Window {
                label: Some("Sonnet".to_owned()),
                utilization: sonnet.utilization,
                resets_at: sonnet.resets_at,
            });
        }
    }

    Ok(LimitsSnapshot {
        session,
        weekly,
        weekly_by_model,
        fetched_at: Utc::now().to_rfc3339(),
        status: LimitsStatus::Ok,
    })
}

// ---------------------------------------------------------------------------
// Core snapshot logic (shared by command + poll task)
// ---------------------------------------------------------------------------

pub async fn fetch_snapshot() -> LimitsSnapshot {
    let token = match read_keychain_token() {
        Ok(t) => t,
        Err(reason) => return LimitsSnapshot::unavailable(&reason),
    };

    match fetch_usage(&token).await {
        Ok(snapshot) => snapshot,
        Err(reason) => LimitsSnapshot::unavailable(&reason),
    }
}

// ---------------------------------------------------------------------------
// Threshold alert state
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct ThresholdState {
    last_resets_at: String,
    highest_fired: u8,
}

pub(crate) struct AlertToFire {
    label: String,
    threshold: u8,
}

fn evaluate_window(
    conn: &Connection,
    window: &Window,
    meta_key: &str,
    human_label: &str,
    alerts: &mut Vec<AlertToFire>,
) {
    // Load persisted state.
    let mut state: ThresholdState = match db::meta_get(conn, meta_key) {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or(ThresholdState {
            last_resets_at: String::new(),
            highest_fired: 0,
        }),
        _ => ThresholdState {
            last_resets_at: String::new(),
            highest_fired: 0,
        },
    };

    // Reset if window has rolled over.
    if state.last_resets_at != window.resets_at {
        state.last_resets_at = window.resets_at.clone();
        state.highest_fired = 0;
    }

    // Find the highest threshold this utilization meets or exceeds.
    let mut crossed: Option<u8> = None;
    for &t in &THRESHOLDS {
        if window.utilization >= f64::from(t) {
            crossed = Some(t);
        }
    }

    if let Some(t) = crossed {
        if t > state.highest_fired {
            alerts.push(AlertToFire {
                label: human_label.to_owned(),
                threshold: t,
            });
            state.highest_fired = t;
        }
    }

    // Persist updated state.
    if let Ok(json) = serde_json::to_string(&state) {
        let _ = db::meta_set(conn, meta_key, &json);
    }
}

pub(crate) fn evaluate_alerts(conn: &Connection, snapshot: &LimitsSnapshot) -> Vec<AlertToFire> {
    let mut alerts: Vec<AlertToFire> = Vec::new();

    if let Some(w) = &snapshot.session {
        evaluate_window(conn, w, "alert_session", "Sesión 5h", &mut alerts);
    }
    if let Some(w) = &snapshot.weekly {
        evaluate_window(conn, w, "alert_weekly", "Semana", &mut alerts);
    }
    for w in &snapshot.weekly_by_model {
        let model = w.label.as_deref().unwrap_or("unknown");
        let key = format!("alert_weekly_scoped:{model}");
        let human = format!("Semana {model}");
        evaluate_window(conn, w, &key, &human, &mut alerts);
    }

    alerts
}

fn send_notifications(app_handle: &AppHandle, alerts: &[AlertToFire]) {
    for alert in alerts {
        let body = format!("{}: {} % usado", alert.label, alert.threshold);
        if let Err(e) = app_handle
            .notification()
            .builder()
            .title("TokenWatch")
            .body(&body)
            .show()
        {
            tracing::warn!("failed to send notification: {e}");
        }
    }
}

fn alerts_muted(conn: &Connection) -> bool {
    db::meta_get(conn, "alerts_muted")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Background limits-polling task
// ---------------------------------------------------------------------------

pub fn spawn_limits_polling_task(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Run once immediately at startup.
        run_limits_and_emit(&app_handle).await;

        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(LIMITS_POLL_SECS));
        interval.tick().await; // consume the first immediate tick
        loop {
            interval.tick().await;
            run_limits_and_emit(&app_handle).await;
        }
    });
}

async fn run_limits_and_emit(app_handle: &AppHandle) {
    let snapshot = fetch_snapshot().await;

    // Evaluate alerts if snapshot is Ok.
    if matches!(snapshot.status, LimitsStatus::Ok) {
        let state = app_handle.state::<AppState>();
        let alert_results: Option<(bool, Vec<AlertToFire>)> = state.conn.lock().ok().map(|conn| {
            let muted = alerts_muted(&conn);
            let alerts = evaluate_alerts(&conn, &snapshot);
            (muted, alerts)
        });
        if let Some((muted, alerts)) = alert_results {
            if !muted && !alerts.is_empty() {
                send_notifications(app_handle, &alerts);
            }
        }
    }

    if let Err(e) = app_handle.emit(LIMITS_UPDATED_EVENT, &snapshot) {
        tracing::warn!("failed to emit {LIMITS_UPDATED_EVENT}: {e}");
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Immediately fetch current usage limits and return a snapshot.
#[tauri::command]
pub async fn query_limits(_state: State<'_, AppState>) -> Result<LimitsSnapshot, String> {
    Ok(fetch_snapshot().await)
}

/// Return whether alert notifications are currently muted.
#[tauri::command]
pub fn get_alerts_muted(state: State<'_, AppState>) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    Ok(alerts_muted(&conn))
}

/// Set whether alert notifications are muted (persisted across restarts).
#[tauri::command]
pub fn set_alerts_muted(state: State<'_, AppState>, muted: bool) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::meta_set(&conn, "alerts_muted", if muted { "true" } else { "false" })
        .map_err(|e| e.to_string())
}
