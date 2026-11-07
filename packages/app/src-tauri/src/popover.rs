//! Popover (tray panel) timing state and macOS diagnostics.
//!
//! The timing logic is pure and platform-independent so it can be unit tested;
//! the NSWindow introspection helper is macOS-only.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A `resign key` that arrives this soon after showing the popover is treated
/// as spurious: it is ignored and the panel is made key again.
pub const SHOW_GRACE: Duration = Duration::from_millis(300);

/// If the panel was hidden by a `resign key` this recently, a tray click only
/// dismisses (the mouse-down already hid it) instead of re-opening it.
pub const RESIGN_CLICK_DEBOUNCE: Duration = Duration::from_millis(250);

/// Timestamps used to disambiguate popover show/hide races.
#[derive(Default)]
pub struct PopoverTiming {
    pub shown_at: Mutex<Option<Instant>>,
    pub hidden_by_resign_at: Mutex<Option<Instant>>,
}

/// True when `since` is set and strictly less than `window` before `now`.
pub fn within(since: Option<Instant>, now: Instant, window: Duration) -> bool {
    match since {
        Some(t) => now.saturating_duration_since(t) < window,
        None => false,
    }
}

/// Log the NSWindow / NSApplication state relevant to the popover not showing.
/// Must be called on the main thread; silently returns otherwise.
#[cfg(target_os = "macos")]
pub fn log_panel_state(win: &tauri::WebviewWindow, stage: &str) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWindow};

    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!(target: "tokenwatch::popover", stage, "log_panel_state off main thread");
        return;
    };
    let Ok(ptr) = win.ns_window() else {
        return;
    };
    if ptr.is_null() {
        return;
    }
    // SAFETY: `ns_window()` returns a valid NSWindow pointer owned by tao; we
    // only borrow it for the duration of this call, on the main thread.
    let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
    let app_active = NSApplication::sharedApplication(mtm).isActive();
    tracing::info!(
        target: "tokenwatch::popover",
        stage,
        is_visible = ns.isVisible(),
        is_key = ns.isKeyWindow(),
        on_active_space = ns.isOnActiveSpace(),
        style_mask = ns.styleMask().0,
        level = ns.level(),
        hides_on_deactivate = ns.hidesOnDeactivate(),
        app_active,
        "popover panel state"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_none_is_false() {
        assert!(!within(None, Instant::now(), SHOW_GRACE));
    }

    #[test]
    fn within_inside_window_is_true() {
        let t = Instant::now();
        assert!(within(Some(t), t + Duration::from_millis(100), SHOW_GRACE));
        assert!(within(Some(t), t, SHOW_GRACE));
    }

    #[test]
    fn within_at_boundary_and_outside_is_false() {
        let t = Instant::now();
        assert!(!within(Some(t), t + SHOW_GRACE, SHOW_GRACE));
        assert!(!within(Some(t), t + Duration::from_millis(500), SHOW_GRACE));
    }
}
