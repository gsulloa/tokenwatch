pub mod budgets;
pub mod config;
pub mod db;
pub mod ingest;
pub mod limits;
pub mod pricing;
pub mod usage;

use std::sync::Mutex;

use tauri::{Emitter, Manager};
use tauri_plugin_updater::Builder as UpdaterBuilder;

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
        .plugin(tauri_plugin_window_state::Builder::default().build())
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

                let panel = popover_win.to_panel().unwrap();

                // Non-activating panel: clicking shows the panel without
                // activating our app, so no Space-switch occurs.
                #[allow(non_upper_case_globals)]
                const NSWindowStyleMaskNonActivatingPanel: i32 = 1 << 7;
                panel.set_style_mask(NSWindowStyleMaskNonActivatingPanel);

                // Collection behavior:
                //   CanJoinAllSpaces    — always on the active Space
                //   FullScreenAuxiliary — draws over fullscreen app's Space
                //   Stationary          — does not move between Spaces itself
                panel.set_collection_behaviour(
                    NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary,
                );

                // Raise to NSPopUpMenuWindowLevel (101) so it floats above
                // the menu bar itself.
                #[allow(non_upper_case_globals)]
                const NSPopUpMenuWindowLevel: i32 = 101;
                panel.set_level(NSPopUpMenuWindowLevel);

                // Auto-hide when the panel loses key status (focus lost).
                let delegate = panel_delegate!(PopoverPanelDelegate {
                    window_did_resign_key
                });

                let app_handle = app.handle().clone();
                delegate.set_listener(Box::new(move |delegate_name: String| {
                    if delegate_name == "window_did_resign_key" {
                        if let Ok(p) =
                            tauri_nspanel::ManagerExt::get_webview_panel(&app_handle, "popover")
                        {
                            p.order_out(None);
                        }
                    }
                }));

                panel.set_delegate(delegate);
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

    let _tray = TrayIconBuilder::new()
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
                        if panel.is_visible() {
                            panel.order_out(None);
                        } else {
                            // Use the WebviewWindow handle only for positioning;
                            // the panel's own show() makes it key without
                            // activating the app.
                            if let Some(win) = app.get_webview_window("popover") {
                                position_popover_near_click(&win, position);
                            }
                            panel.show();
                            // Becoming key as a panel can leave the webview
                            // scrolled to the bottom; tell the frontend to pin
                            // its scroll position back to the top.
                            let _ = app.emit("popover-shown", ());
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
