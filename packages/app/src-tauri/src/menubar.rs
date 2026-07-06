//! Menu-bar text badge: renders live usage percentage next to the tray icon.
//!
//! macOS does not let an app control the ordering/priority of its status item
//! relative to other apps. What we CAN do is put the usage percentage into the
//! item itself (via `TrayIcon::set_title`) so it is glanceable without a click.

use rusqlite::Connection;
use tauri::{AppHandle, Manager, State};

use crate::db;
use crate::limits::{LimitsSnapshot, LimitsStatus};
use crate::usage::AppState;

/// Meta key under which the badge mode is persisted.
pub const BADGE_MODE_KEY: &str = "menubar_badge_mode";

/// What the menu-bar text badge shows.
///
/// Persisted in the `meta` table under [`BADGE_MODE_KEY`]. Value contract:
/// `off` (icon only, default), `session` (5h session %), `week` (weekly %),
/// `max` (the larger of session and weekly %).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeMode {
    Off,
    Session,
    Week,
    Max,
}

impl BadgeMode {
    /// Parse a stored/incoming value. Returns `None` for unrecognised input.
    pub fn parse(s: &str) -> Option<BadgeMode> {
        match s {
            "off" => Some(BadgeMode::Off),
            "session" => Some(BadgeMode::Session),
            "week" => Some(BadgeMode::Week),
            "max" => Some(BadgeMode::Max),
            _ => None,
        }
    }

    /// Stable string form for persistence and the frontend contract.
    pub fn as_str(self) -> &'static str {
        match self {
            BadgeMode::Off => "off",
            BadgeMode::Session => "session",
            BadgeMode::Week => "week",
            BadgeMode::Max => "max",
        }
    }
}

/// Handle to the tray icon, stored as managed state so the badge can be updated
/// after startup without recreating the icon (which would drop its position).
pub struct TrayHandle(pub std::sync::Mutex<Option<tauri::tray::TrayIcon>>);

/// Compute the badge text for a mode + snapshot.
///
/// - `Off` → `None` (icon only).
/// - Otherwise → `Some("N%")` when the snapshot is `Ok` and the relevant window
///   exists; `Some("–")` when the snapshot is missing/unavailable or the window
///   is absent. Never returns a stale value — that is the caller's concern
///   (it always passes the latest known snapshot).
pub fn format_badge(mode: BadgeMode, snapshot: Option<&LimitsSnapshot>) -> Option<String> {
    if mode == BadgeMode::Off {
        return None;
    }
    Some(badge_text(mode, snapshot))
}

fn badge_text(mode: BadgeMode, snapshot: Option<&LimitsSnapshot>) -> String {
    let Some(snap) = snapshot else {
        return "–".to_owned();
    };
    if !matches!(snap.status, LimitsStatus::Ok) {
        return "–".to_owned();
    }
    let session = snap.session.as_ref().map(|w| w.utilization);
    let weekly = snap.weekly.as_ref().map(|w| w.utilization);
    let pct = match mode {
        BadgeMode::Session => session,
        BadgeMode::Week => weekly,
        BadgeMode::Max => match (session, weekly) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        },
        BadgeMode::Off => None,
    };
    match pct {
        Some(p) => format!("{}%", p.round() as i64),
        None => "–".to_owned(),
    }
}

fn read_mode(conn: &Connection) -> BadgeMode {
    db::meta_get(conn, BADGE_MODE_KEY)
        .ok()
        .flatten()
        .and_then(|s| BadgeMode::parse(&s))
        .unwrap_or(BadgeMode::Off)
}

/// Re-apply the tray title from the current mode + last known snapshot.
///
/// Locks `conn` and `last_limits` sequentially (never together) to respect the
/// AppState lock discipline.
pub fn apply_badge(app: &AppHandle) {
    let mode = {
        let state = app.state::<AppState>();
        let guard = match state.conn.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let m = read_mode(&guard);
        drop(guard);
        m
    };

    let title = {
        let state = app.state::<AppState>();
        let snapshot = state.last_limits.lock().ok().and_then(|g| g.clone());
        format_badge(mode, snapshot.as_ref())
    };

    let tray_state = app.state::<TrayHandle>();
    let guard = tray_state.0.lock();
    if let Ok(guard) = guard {
        if let Some(tray) = guard.as_ref() {
            let _ = tray.set_title(title);
        }
    }
}

/// Return the current badge mode as its string value.
#[tauri::command]
pub fn get_menubar_badge_mode(state: State<'_, AppState>) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    Ok(read_mode(&conn).as_str().to_owned())
}

/// Persist a new badge mode and re-apply the tray title immediately.
#[tauri::command]
pub fn set_menubar_badge_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: String,
) -> Result<(), String> {
    let parsed = BadgeMode::parse(&mode).ok_or_else(|| format!("invalid badge mode: {mode}"))?;
    {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        db::meta_set(&conn, BADGE_MODE_KEY, parsed.as_str()).map_err(|e| e.to_string())?;
    }
    apply_badge(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Window;

    fn snap_ok(session: Option<f64>, weekly: Option<f64>) -> LimitsSnapshot {
        LimitsSnapshot {
            session: session.map(|u| Window {
                label: None,
                utilization: u,
                resets_at: "2026-07-06T12:00:00Z".to_owned(),
            }),
            weekly: weekly.map(|u| Window {
                label: None,
                utilization: u,
                resets_at: "2026-07-12T00:00:00Z".to_owned(),
            }),
            weekly_by_model: vec![],
            fetched_at: "2026-07-06T10:00:00Z".to_owned(),
            status: LimitsStatus::Ok,
        }
    }

    #[test]
    fn off_is_none_regardless_of_snapshot() {
        assert_eq!(format_badge(BadgeMode::Off, None), None);
        assert_eq!(
            format_badge(BadgeMode::Off, Some(&snap_ok(Some(45.0), Some(56.0)))),
            None
        );
    }

    #[test]
    fn session_mode_shows_session_percent() {
        let s = snap_ok(Some(45.0), Some(56.0));
        assert_eq!(
            format_badge(BadgeMode::Session, Some(&s)),
            Some("45%".to_owned())
        );
    }

    #[test]
    fn week_mode_shows_weekly_percent() {
        let s = snap_ok(Some(45.0), Some(56.0));
        assert_eq!(
            format_badge(BadgeMode::Week, Some(&s)),
            Some("56%".to_owned())
        );
    }

    #[test]
    fn max_mode_shows_larger_percent() {
        let s = snap_ok(Some(20.0), Some(56.0));
        assert_eq!(
            format_badge(BadgeMode::Max, Some(&s)),
            Some("56%".to_owned())
        );
    }

    #[test]
    fn max_mode_falls_back_to_present_window() {
        let s = snap_ok(Some(20.0), None);
        assert_eq!(
            format_badge(BadgeMode::Max, Some(&s)),
            Some("20%".to_owned())
        );
    }

    #[test]
    fn rounds_to_nearest_integer() {
        let s = snap_ok(Some(44.6), None);
        assert_eq!(
            format_badge(BadgeMode::Session, Some(&s)),
            Some("45%".to_owned())
        );
    }

    #[test]
    fn none_snapshot_shows_dash() {
        assert_eq!(format_badge(BadgeMode::Session, None), Some("–".to_owned()));
    }

    #[test]
    fn unavailable_snapshot_shows_dash() {
        let s = LimitsSnapshot {
            session: None,
            weekly: None,
            weekly_by_model: vec![],
            fetched_at: "2026-07-06T10:00:00Z".to_owned(),
            status: LimitsStatus::Unavailable {
                reason: "expired".to_owned(),
            },
        };
        assert_eq!(
            format_badge(BadgeMode::Week, Some(&s)),
            Some("–".to_owned())
        );
    }

    #[test]
    fn missing_window_in_ok_snapshot_shows_dash() {
        let s = snap_ok(None, Some(56.0));
        assert_eq!(
            format_badge(BadgeMode::Session, Some(&s)),
            Some("–".to_owned())
        );
    }

    #[test]
    fn parse_roundtrip() {
        for m in [
            BadgeMode::Off,
            BadgeMode::Session,
            BadgeMode::Week,
            BadgeMode::Max,
        ] {
            assert_eq!(BadgeMode::parse(m.as_str()), Some(m));
        }
        assert_eq!(BadgeMode::parse("bogus"), None);
    }
}
