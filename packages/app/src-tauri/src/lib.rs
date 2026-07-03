pub mod config;
pub mod db;
pub mod ingest;
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
        // TODO(menubar): TokenWatch is a macOS menu-bar app.
        //   - set macOS activation policy to Accessory:
        //       #[cfg(target_os = "macos")]
        //       app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        //   - build a TrayIcon (tauri::tray::TrayIconBuilder) with a menu and
        //     toggle a borderless popover window on click.
        //   - hide the main window on launch / on blur.
        .setup(|app| {
            // Open the database and apply migrations.
            let conn = db::open_default().expect("failed to open TokenWatch database");

            // Register the state with Tauri (managed state for commands).
            app.manage(AppState {
                conn: Mutex::new(conn),
            });

            // Spawn the background polling task: ingest at startup + every 30s.
            // The task fetches the managed state from the app handle.
            usage::spawn_polling_task(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            usage::refresh_usage,
            usage::query_series,
            usage::usage_meta,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenWatch");
}
