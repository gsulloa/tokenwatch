pub mod config;
pub mod db;
pub mod ingest;
pub mod limits;
pub mod pricing;
pub mod usage;

use std::sync::Mutex;

use tauri::Manager;
use tauri_plugin_updater::Builder as UpdaterBuilder;

use usage::AppState;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(UpdaterBuilder::new().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // macOS menu-bar mode: no Dock icon.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Open the database and apply migrations.
            let conn = db::open_default().expect("failed to open TokenWatch database");

            // Register the state with Tauri (managed state for commands).
            app.manage(AppState {
                conn: Mutex::new(conn),
            });

            // Spawn the background polling task: ingest at startup + every 30s.
            usage::spawn_polling_task(app.handle().clone());

            // Spawn the limits polling task: fetch limits at startup + every 5 min.
            limits::spawn_limits_polling_task(app.handle().clone());

            // Build the system tray icon with a context menu.
            build_tray(app)?;

            // Subscribe to window focus-lost → hide the popover (not main).
            let popover_win = app
                .get_webview_window("popover")
                .expect("popover window must exist");
            let popover_win_clone = popover_win.clone();
            popover_win.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = popover_win_clone.hide();
                }
            });

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

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("no default window icon")?;

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "dashboard" => {
                if let Some(win) = app.get_webview_window("main") {
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
                if let Some(win) = app.get_webview_window("popover") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = win.hide();
                    } else {
                        // Position the popover near the tray click point.
                        position_popover_near_click(&win, position);
                        let _ = win.show();
                        let _ = win.set_focus();
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

// ---------------------------------------------------------------------------
// Frontend-invokable commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn open_dashboard(app: tauri::AppHandle) -> Result<(), String> {
    // An Accessory (menu-bar) app cannot bring its own windows to the front.
    // Switch to Regular so the dashboard can be shown, focused and appear in
    // the Dock / app switcher; we revert to Accessory when it is closed.
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }
    // Hide the popover when opening the dashboard.
    if let Some(pop) = app.get_webview_window("popover") {
        let _ = pop.hide();
    }
    Ok(())
}
