// No console window behind the app on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod settings;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use tauri::Manager;

pub struct AppState {
    pub settings: Mutex<settings::Settings>,
    /// The profile on screen, kept so an export doesn't have to send it back from the window.
    pub report: Mutex<Option<snapintel_core::Report>>,
    pub cancel: AtomicBool,
    pub downloading: AtomicBool,
    /// Files written by an export, which the window may then ask to show in the file manager.
    pub exports: Mutex<Vec<PathBuf>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let settings = settings::load(app.handle());
            commands::apply_theme(app.handle(), &settings.theme);
            app.manage(AppState {
                settings: Mutex::new(settings),
                report: Mutex::new(None),
                cancel: AtomicBool::new(false),
                downloading: AtomicBool::new(false),
                exports: Mutex::new(Vec::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::lookup,
            commands::get_settings,
            commands::save_settings,
            commands::pick_folder,
            commands::download,
            commands::cancel_download,
            commands::verify_downloads,
            commands::save_snapcode,
            commands::export_report,
            commands::transcript,
            commands::bitmoji_history,
            commands::self_check,
            commands::open_link,
            commands::show_in_folder,
            commands::account_folder,
            commands::startup_username,
        ])
        .run(tauri::generate_context!())
        .expect("SnapIntel could not start");
}
