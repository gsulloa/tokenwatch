pub mod budgets;
pub mod config;
pub mod db;
pub mod ingest;
pub mod limits;
pub mod menubar;
pub mod popover;
pub mod pricing;
pub mod usage;

use std::sync::Mutex;

use tauri::{Emitter, Manager};
use tauri_plugin_updater::Builder as UpdaterBuilder;
use tauri_plugin_window_state::StateFlags;

use usage::AppState;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(
            // Never persist the popover (a tray NSPanel whose size/visibility must
            // not be restored). For the dashboard, never restore FULLSCREEN or
            // VISIBLE: a stale fullscreen/visible `main` would create a Space or
            // show a window at launch, contradicting "hidden at startup".
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["popover"])
                .with_state_flags(StateFlags::all() - StateFlags::FULLSCREEN - StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_process::init())
        .plugin(UpdaterBuilder::new().build())
        .plugin(tauri_plugin_notification::init());

    // Register the NSPanel plugin on macOS only.
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .setup(|app| {
            // macOS menu-bar mode: no Dock icon.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Timing state used to disambiguate popover show/hide races.
            app.manage(popover::PopoverTiming::default());

            // Open the database and apply migrations.
            let conn = db::open_default().expect("failed to open TokenWatch database");

            // Register the state with Tauri (managed state for commands).
            app.manage(AppState {
                conn: Mutex::new(conn),
                last_limits: std::sync::Mutex::new(None),
            });

            // Spawn the background polling task: ingest at startup + every 30s.
            usage::spawn_polling_task(app.handle().clone());

            // Spawn the limits polling task: fetch limits at startup + every 5 min.
            limits::spawn_limits_polling_task(app.handle().clone());

            // Build the system tray icon with a context menu.
            build_tray(app)?;

            // ----------------------------------------------------------------
            // macOS: Convert the popover WebviewWindow into a non-activating
            // NSPanel so it can appear over fullscreen Spaces without switching
            // Spaces or activating our app.
            // ----------------------------------------------------------------
            #[cfg(target_os = "macos")]
            {
                // tauri-nspanel re-exports the (deprecated) `cocoa` crate through
                // its public API — `NSWindowCollectionBehavior` and the
                // `panel_delegate!` macro. Consuming them is unavoidable until the
                // plugin migrates to objc2, so we scope-allow the deprecation here.
                // (The `panel_delegate!` macro also expands to legacy `objc` macros
                // that reference a `cargo-clippy` cfg; that `unexpected_cfgs` noise is
                // silenced via check-cfg in Cargo.toml's [lints].)
                #![allow(deprecated)]
                use tauri_nspanel::{
                    cocoa::appkit::NSWindowCollectionBehavior, panel_delegate, WebviewWindowExt,
                };

                let popover_win = app
                    .get_webview_window("popover")
                    .expect("popover window must exist");

                // Keep the existing style-mask bits and OR in NonactivatingPanel
                // (replacing the whole mask can drop bits macOS needs). Done on
                // the NSWindow before it is converted to a panel; setup runs on
                // the main thread.
                if let Ok(ptr) = popover_win.ns_window() {
                    if !ptr.is_null() {
                        // SAFETY: valid NSWindow pointer owned by tao, used
                        // synchronously on the main thread.
                        let ns: &objc2_app_kit::NSWindow =
                            unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
                        ns.setStyleMask(
                            ns.styleMask() | objc2_app_kit::NSWindowStyleMask::NonactivatingPanel,
                        );
                        // Float above the menu bar itself.
                        ns.setLevel(objc2_app_kit::NSPopUpMenuWindowLevel);
                    }
                }

                let panel = popover_win.to_panel().unwrap();

                // Collection behavior:
                //   CanJoinAllSpaces    — always on the active Space
                //   FullScreenAuxiliary — draws over fullscreen app's Space
                //   Stationary          — does not move between Spaces itself
                panel.set_collection_behaviour(
                    NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary,
                );

                // NSPanel defaults to hidesOnDeactivate = YES; an Accessory app
                // that was never activated is permanently "inactive", so AppKit
                // would hide the panel immediately.
                panel.set_hides_on_deactivate(false);

                // Auto-hide when the panel loses key status (focus lost), except
                // for a spurious resign right after showing.
                let delegate = panel_delegate!(PopoverPanelDelegate {
                    window_did_resign_key
                });

                let app_handle = app.handle().clone();
                delegate.set_listener(Box::new(move |delegate_name: String| {
                    if delegate_name != "window_did_resign_key" {
                        return;
                    }
                    let now = std::time::Instant::now();
                    let timing = app_handle.state::<popover::PopoverTiming>();
                    let shown_at = *timing.shown_at.lock().unwrap();
                    let since_show_ms =
                        shown_at.map(|t| now.saturating_duration_since(t).as_millis());
                    tracing::info!(
                        target: "tokenwatch::popover",
                        ?since_show_ms,
                        "window_did_resign_key"
                    );
                    if popover::within(shown_at, now, popover::SHOW_GRACE) {
                        tracing::info!(target: "tokenwatch::popover", "resign ignored");
                        let handle = app_handle.clone();
                        let _ = app_handle.run_on_main_thread(move || {
                            if let Ok(p) =
                                tauri_nspanel::ManagerExt::get_webview_panel(&handle, "popover")
                            {
                                p.make_key_window();
                            }
                        });
                    } else if let Ok(p) =
                        tauri_nspanel::ManagerExt::get_webview_panel(&app_handle, "popover")
                    {
                        p.order_out(None);
                        *timing.hidden_by_resign_at.lock().unwrap() = Some(now);
                    }
                }));

                panel.set_delegate(delegate);

                popover::log_panel_state(&popover_win, "after_setup");
            }

            // ----------------------------------------------------------------
            // Non-macOS: use a plain WebviewWindow with a focus-lost handler.
            // ----------------------------------------------------------------
            #[cfg(not(target_os = "macos"))]
            {
                let popover_win = app
                    .get_webview_window("popover")
                    .expect("popover window must exist");

                let popover_win_clone = popover_win.clone();
                popover_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let _ = popover_win_clone.hide();
                    }
                });
            }

            // Closing the dashboard should hide it (keep the window alive so it
            // can be reopened) and drop back to Accessory so no Dock icon lingers.
            let dashboard_win = app
                .get_webview_window("main")
                .expect("main window must exist");
            let dashboard_app = app.handle().clone();
            dashboard_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(win) = dashboard_app.get_webview_window("main") {
                        let _ = win.hide();
                    }
                    #[cfg(target_os = "macos")]
                    let _ = dashboard_app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            usage::refresh_usage,
            usage::query_series,
            usage::usage_meta,
            usage::query_today_by_project,
            limits::query_limits,
            limits::get_alerts_muted,
            limits::set_alerts_muted,
            menubar::get_menubar_badge_mode,
            menubar::set_menubar_badge_mode,
            budgets::list_groups,
            budgets::create_group,
            budgets::update_group,
            budgets::delete_group,
            budgets::assign_project,
            budgets::unassign_project,
            budgets::list_project_names,
            budgets::query_group_budgets,
            open_dashboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenWatch");
}

// ---------------------------------------------------------------------------
// Tray icon setup
// ---------------------------------------------------------------------------

fn build_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let dashboard_item = MenuItemBuilder::with_id("dashboard", "Abrir dashboard").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Salir").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&dashboard_item, &quit_item])
        .build()?;

    // Menu-bar icon: a monochrome gauge glyph. Marked as a template image so
    // macOS renders it from the alpha mask — black on light menu bars, white on
    // dark ones — instead of reusing the full-color app icon.
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/menubar-template.png"))?;

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "dashboard" => {
                // Switch to Regular first so the dashboard can come to the front
                // and its icon shows in the Dock (Accessory apps have no Dock icon).
                #[cfg(target_os = "macos")]
                {
                    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
                    set_dock_icon(app);
                }
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                let app = tray.app_handle();

                // ----------------------------------------------------------
                // macOS: use the NSPanel API so the popover appears over
                // fullscreen Spaces without activating our app.
                // ----------------------------------------------------------
                #[cfg(target_os = "macos")]
                {
                    use tauri_nspanel::ManagerExt;

                    if let Ok(panel) = app.get_webview_panel("popover") {
                        let now = std::time::Instant::now();
                        let timing = app.state::<popover::PopoverTiming>();
                        let visible = panel.is_visible();
                        tracing::info!(
                            target: "tokenwatch::popover",
                            x = position.x,
                            y = position.y,
                            visible,
                            "tray click"
                        );
                        if visible {
                            panel.order_out(None);
                        } else if popover::within(
                            *timing.hidden_by_resign_at.lock().unwrap(),
                            now,
                            popover::RESIGN_CLICK_DEBOUNCE,
                        ) {
                            // The mouse-down already hid the panel via resign key;
                            // this click only dismisses it.
                            tracing::info!(target: "tokenwatch::popover", "dismiss click");
                            *timing.hidden_by_resign_at.lock().unwrap() = None;
                        } else {
                            // Use the WebviewWindow handle only for positioning.
                            if let Some(win) = app.get_webview_window("popover") {
                                position_popover_near_click(&win, position);
                                popover::log_panel_state(&win, "before_show");
                            }
                            let handle = app.clone();
                            let _ = app.run_on_main_thread(move || {
                                use tauri_nspanel::ManagerExt;
                                let Ok(panel) = handle.get_webview_panel("popover") else {
                                    return;
                                };
                                let timing = handle.state::<popover::PopoverTiming>();
                                *timing.shown_at.lock().unwrap() = Some(std::time::Instant::now());
                                panel.show();
                                if !panel.is_visible() {
                                    tracing::warn!(
                                        target: "tokenwatch::popover",
                                        "show() left panel invisible; forcing front"
                                    );
                                    panel.order_front_regardless();
                                    panel.make_key_window();
                                }
                                if let Some(win) = handle.get_webview_window("popover") {
                                    popover::log_panel_state(&win, "after_show");
                                }
                                // Becoming key as a panel can leave the webview
                                // scrolled to the bottom; tell the frontend to pin
                                // its scroll position back to the top.
                                let _ = handle.emit("popover-shown", ());

                                // Next run-loop turn.
                                let h2 = handle.clone();
                                let _ = handle.run_on_main_thread(move || {
                                    if let Some(win) = h2.get_webview_window("popover") {
                                        popover::log_panel_state(&win, "next_tick");
                                    }
                                });
                                // 500 ms later.
                                let h3 = handle.clone();
                                std::thread::spawn(move || {
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                    let h4 = h3.clone();
                                    let _ = h3.run_on_main_thread(move || {
                                        if let Some(win) = h4.get_webview_window("popover") {
                                            popover::log_panel_state(&win, "after_500ms");
                                        }
                                    });
                                });
                            });
                        }
                    }
                }

                // ----------------------------------------------------------
                // Non-macOS: plain WebviewWindow toggle.
                // ----------------------------------------------------------
                #[cfg(not(target_os = "macos"))]
                {
                    if let Some(win) = app.get_webview_window("popover") {
                        if win.is_visible().unwrap_or(false) {
                            let _ = win.hide();
                        } else {
                            position_popover_near_click(&win, position);
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                }
            }
        })
        .build(app)?;

    // Keep the tray handle in managed state so the menu-bar badge can be
    // updated after startup without recreating the icon (which would drop its
    // position). Apply the initial title from the last known snapshot (if any).
    app.manage(menubar::TrayHandle(std::sync::Mutex::new(Some(tray))));
    menubar::apply_badge(app.handle());

    Ok(())
}

/// Reposition the popover window so it appears just below the menu bar at the
/// x-position of the tray click.
fn position_popover_near_click(
    win: &tauri::WebviewWindow,
    click_pos: tauri::PhysicalPosition<f64>,
) {
    // Get window size so we can centre it horizontally on the click x.
    let (win_w, _) = win
        .outer_size()
        .map(|s| (s.width as f64, s.height as f64))
        .unwrap_or((360.0, 480.0));

    // Place the popover horizontally centred on the tray icon click, and just
    // below the macOS menu bar (approximately 22 px).
    let menu_bar_h: f64 = 22.0;
    let x = (click_pos.x - win_w / 2.0).max(0.0);
    let y = menu_bar_h;

    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Assign the app's Dock icon at runtime (macOS).
///
/// In dev builds Tauri sets the Dock icon only once, at startup. Because we boot
/// as an `Accessory` app (no Dock presence) and only switch to `Regular` when the
/// dashboard opens, that initial assignment is lost and macOS falls back to the
/// generic executable icon. Re-applying it here — on the main thread, right after
/// the policy switch — keeps the real icon in the Dock. No-op cost in release,
/// where the bundled `.app` already carries the icon.
#[cfg(target_os = "macos")]
fn set_dock_icon(app: &tauri::AppHandle) {
    const ICON_PNG: &[u8] = include_bytes!("../icons/icon.png");

    let _ = app.run_on_main_thread(|| {
        use objc2::{AnyThread, MainThreadMarker};
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::NSData;

        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let data = NSData::with_bytes(ICON_PNG);
        if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
            let ns_app = NSApplication::sharedApplication(mtm);
            unsafe { ns_app.setApplicationIconImage(Some(&image)) };
        }
    });
}

// ---------------------------------------------------------------------------
// Frontend-invokable commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn open_dashboard(app: tauri::AppHandle) -> Result<(), String> {
    // An Accessory (menu-bar) app cannot bring its own windows to the front.
    // Switch to Regular so the dashboard can be shown, focused and appear in
    // the Dock / app switcher; we revert to Accessory when it is closed.
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        set_dock_icon(&app);
    }

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }

    // Hide the popover when opening the dashboard.
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(panel) = app.get_webview_panel("popover") {
            panel.order_out(None);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(pop) = app.get_webview_window("popover") {
            let _ = pop.hide();
        }
    }

    Ok(())
}
