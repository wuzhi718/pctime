mod classifier;
mod collector;
mod commands;
mod models;
mod sampler;
mod storage;
mod visibility;

use std::env;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use tauri::Manager;

pub struct AppState {
    pub db_path: PathBuf,
    pub storage_location: String,
    pub monitoring: Arc<AtomicBool>,
}

fn start_monitor(db_path: PathBuf, monitoring: Arc<AtomicBool>) {
    thread::spawn(move || loop {
        if monitoring.load(Ordering::Relaxed) {
            let _ = sampler::capture_current(&db_path, 1_000);
        }

        thread::sleep(Duration::from_secs(1));
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let fallback_dir = app.path().app_data_dir()?;
            let install_data_dir = env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.join("pctime-data")))
                .unwrap_or_else(|| fallback_dir.clone());
            let (db_path, storage_location) =
                storage::init_database(&install_data_dir, &fallback_dir)?;
            let _ = storage::refresh_unclassified_categories(&db_path);
            let monitoring = Arc::new(AtomicBool::new(true));

            start_monitor(db_path.clone(), Arc::clone(&monitoring));

            app.manage(AppState {
                db_path,
                storage_location,
                monitoring,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::record_now,
            commands::list_live_windows,
            commands::set_monitoring,
            commands::get_startup_enabled,
            commands::set_startup_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
