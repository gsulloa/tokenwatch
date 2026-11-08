//! macOS status-item click handling.
//!
//! tray-icon overlays a `TaoTrayTarget` view on the status bar button and
//! attaches the menu to the `NSStatusItem`. On recent macOS releases the
//! overlay never receives clicks and AppKit shows the attached menu instead,
//! so the popover never opens. Here we hide the overlay, drive the button's
//! own target/action, and attach the menu only transiently for right-click.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSEventMask, NSEventModifierFlags, NSEventType, NSScreen};
use tauri::{AppHandle, Manager};

static APP: OnceLock<AppHandle> = OnceLock::new();
static MENU_OPEN: AtomicBool = AtomicBool::new(false);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "TokenWatchTrayTarget"]
    #[ivars = ()]
    struct TrayTarget;

    impl TrayTarget {
        #[unsafe(method(trayClicked:))]
        fn tray_clicked(&self, _sender: Option<&AnyObject>) {
            on_tray_clicked(MainThreadMarker::from(self));
        }
    }
);

impl TrayTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

/// Hide tray-icon's `TaoTrayTarget` overlay subviews so they can never
/// intercept clicks.
fn hide_overlay_in(item: &objc2_app_kit::NSStatusItem, mtm: MainThreadMarker) {
    let Some(button) = item.button(mtm) else {
        return;
    };
    for view in button.subviews().iter() {
        if view.class().name().to_bytes() == b"TaoTrayTarget" && !view.isHidden() {
            view.setHidden(true);
        }
    }
}

/// Re-hide the overlay (call after anything that may re-show it, e.g. set_title).
pub fn hide_overlay(tray: &tauri::tray::TrayIcon) {
    let _ = tray.with_inner_tray_icon(|inner| {
        if let (Some(item), Some(mtm)) = (inner.ns_status_item(), MainThreadMarker::new()) {
            hide_overlay_in(&item, mtm);
        }
    });
}

/// Take over the status button's click handling. Must run on the main thread.
pub fn install(app: &AppHandle, tray: &tauri::tray::TrayIcon) {
    let _ = APP.set(app.clone());
    let _ = tray.with_inner_tray_icon(|inner| {
        let (Some(item), Some(mtm)) = (inner.ns_status_item(), MainThreadMarker::new()) else {
            tracing::warn!(target: "tokenwatch::popover", "no status item to install on");
            return;
        };
        let Some(button) = item.button(mtm) else {
            return;
        };
        hide_overlay_in(&item, mtm);

        let target = TrayTarget::new(mtm);
        let obj: &AnyObject = &target;
        unsafe {
            button.setTarget(Some(obj));
            button.setAction(Some(sel!(trayClicked:)));
        }
        button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
        // NSControl's target is weak; keep the target alive for the process lifetime.
        std::mem::forget(target);
        tracing::info!(target: "tokenwatch::popover", "tray button action installed");
    });
}

fn on_tray_clicked(mtm: MainThreadMarker) {
    let Some(app) = APP.get() else {
        return;
    };
    if MENU_OPEN.load(Ordering::SeqCst) {
        return;
    }
    let event = NSApplication::sharedApplication(mtm).currentEvent();
    let (right, ctrl) = match &event {
        Some(e) => (
            e.r#type() == NSEventType::RightMouseUp,
            e.modifierFlags().contains(NSEventModifierFlags::Control),
        ),
        None => (false, false),
    };
    if right || ctrl {
        tracing::info!(target: "tokenwatch::popover", button = "right", "tray click");
        show_menu(app);
    } else {
        tracing::info!(target: "tokenwatch::popover", button = "left", "tray click");
        crate::toggle_popover(app, None);
    }
}

/// Pop the context menu: attach it to the status item only for the duration
/// of the (modal) click, then detach so left clicks reach our action again.
fn show_menu(app: &AppHandle) {
    crate::hide_popover(app);
    let Some(tray) = app.tray_by_id(crate::TRAY_ID) else {
        return;
    };
    let menu = app.state::<crate::TrayMenu>().0.clone();
    MENU_OPEN.store(true, Ordering::SeqCst);
    let _ = tray.set_menu(Some(menu));
    let _ = tray.with_inner_tray_icon(|inner| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let button = inner.ns_status_item().and_then(|i| i.button(mtm));
        // Without an action the button shows the attached menu on click.
        if let Some(b) = &button {
            unsafe { b.setAction(None) };
        }
        inner.show_menu();
        if let Some(b) = &button {
            unsafe { b.setAction(Some(sel!(trayClicked:))) };
            b.highlight(false);
        }
    });
    let _ = tray.set_menu(None::<tauri::menu::Menu<tauri::Wry>>);
    MENU_OPEN.store(false, Ordering::SeqCst);
}

/// Logical (top-left origin) position for a popover of `width` points, just
/// below the status item and clamped to its screen's visible frame.
pub fn popover_origin(app: &AppHandle, width: f64) -> Option<(f64, f64)> {
    let tray = app.tray_by_id(crate::TRAY_ID)?;
    tray.with_inner_tray_icon(move |inner| {
        let mtm = MainThreadMarker::new()?;
        let window = inner.ns_status_item()?.button(mtm)?.window()?;
        let f = window.frame();
        let primary_h = NSScreen::screens(mtm).firstObject()?.frame().size.height;
        let mut x = f.origin.x + f.size.width / 2.0 - width / 2.0;
        let y = primary_h - f.origin.y;
        if let Some(screen) = window.screen() {
            let vf = screen.visibleFrame();
            let (min, max) = (vf.origin.x + 8.0, vf.origin.x + vf.size.width - width - 8.0);
            x = x.min(max).max(min);
        }
        Some((x, y))
    })
    .ok()
    .flatten()
}
