//! Tauri shell: wires the OS-agnostic core and the current platform to the
//! web UI. All business logic lives in `trufi-core`; this crate only owns
//! state and exposes commands.

mod commands;
mod state;

use state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let rules_dir = app.path().app_config_dir().ok().map(|d| d.join("rules"));
            app.manage(AppState::new(rules_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system_info,
            commands::snapshot,
            commands::health,
            commands::rules,
            commands::scan,
            commands::clean,
            commands::startup_items,
            commands::set_startup_enabled,
            commands::installed_apps,
            commands::uninstall_app,
            commands::relaunch_elevated,
            commands::reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Trufi Cleaner");
}
