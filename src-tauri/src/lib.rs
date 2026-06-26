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
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::WindowEvent;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

const TRAY_SHOW: &str = "tray_show";
const TRAY_SETTINGS: &str = "tray_settings";
const TRAY_FEEDBACK: &str = "tray_feedback";
const TRAY_PROJECT: &str = "tray_project";
const TRAY_QUIT: &str = "tray_quit";
const PROJECT_URL: &str = "https://github.com/wuzhi718/pctime";
const FEEDBACK_URL: &str = "https://github.com/wuzhi718/pctime/issues";

pub struct AppState {
    pub db_path: PathBuf,
    pub settings_path: PathBuf,
    pub storage_location: String,
    pub close_to_tray: Arc<AtomicBool>,
    pub monitoring: Arc<AtomicBool>,
    pub sample_interval_ms: Arc<AtomicU64>,
}

fn start_monitor(
    db_path: PathBuf,
    monitoring: Arc<AtomicBool>,
    sample_interval_ms: Arc<AtomicU64>,
) {
    thread::spawn(move || loop {
        let interval_ms =
            sampler::normalize_sample_interval_ms(sample_interval_ms.load(Ordering::Relaxed));

        if monitoring.load(Ordering::Relaxed) {
            let _ = sampler::capture_current(&db_path, interval_ms);
        }

        thread::sleep(Duration::from_millis(interval_ms));
    });
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW => show_main_window(app),
            TRAY_SETTINGS => {
                show_main_window(app);
                let _ = app.emit("pctime://open-settings", ());
            }
            TRAY_FEEDBACK => {
                let _ = app.opener().open_url(FEEDBACK_URL, None::<&str>);
            }
            TRAY_PROJECT => {
                let _ = app.opener().open_url(PROJECT_URL, None::<&str>);
            }
            TRAY_QUIT => app.exit(0),
            _ => {}
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();

                if state.close_to_tray.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let fallback_dir = app.path().app_data_dir()?;
            let settings_path = fallback_dir.join("settings.json");
            let install_data_dir = env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.join("pctime-data")))
                .unwrap_or_else(|| fallback_dir.clone());
            let (db_path, storage_location) =
                storage::init_database(&install_data_dir, &fallback_dir)?;
            let _ = storage::refresh_unclassified_categories(&db_path);
            let close_to_tray = Arc::new(AtomicBool::new(commands::load_close_to_tray(
                &settings_path,
            )));
            let monitoring = Arc::new(AtomicBool::new(true));
            let sample_interval_ms = Arc::new(AtomicU64::new(commands::load_sample_interval_ms(
                &settings_path,
            )));

            start_monitor(
                db_path.clone(),
                Arc::clone(&monitoring),
                Arc::clone(&sample_interval_ms),
            );

            if let Some(icon) = app.default_window_icon().cloned() {
                let show = MenuItem::with_id(app, TRAY_SHOW, "显示主界面", true, None::<&str>)?;
                let settings = MenuItem::with_id(app, TRAY_SETTINGS, "设置", true, None::<&str>)?;
                let feedback =
                    MenuItem::with_id(app, TRAY_FEEDBACK, "意见反馈", true, None::<&str>)?;
                let project = MenuItem::with_id(app, TRAY_PROJECT, "项目主页", true, None::<&str>)?;
                let separator = PredefinedMenuItem::separator(app)?;
                let quit = MenuItem::with_id(app, TRAY_QUIT, "退出应用", true, None::<&str>)?;
                let tray_menu = Menu::with_items(
                    app,
                    &[&show, &settings, &feedback, &project, &separator, &quit],
                )?;

                let _ = TrayIconBuilder::new()
                    .icon(icon)
                    .menu(&tray_menu)
                    .tooltip("PCTime")
                    .show_menu_on_left_click(false)
                    .on_tray_icon_event(|tray, event| match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        }
                        | TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        } => {
                            show_main_window(tray.app_handle());
                        }
                        _ => {}
                    })
                    .build(app);
            }

            app.manage(AppState {
                db_path,
                settings_path,
                storage_location,
                close_to_tray,
                monitoring,
                sample_interval_ms,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::record_now,
            commands::list_live_windows,
            commands::set_monitoring,
            commands::get_app_version,
            commands::get_close_to_tray,
            commands::set_close_to_tray,
            commands::get_sample_interval_ms,
            commands::set_sample_interval_ms,
            commands::get_startup_enabled,
            commands::set_startup_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
