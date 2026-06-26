use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;

use tauri::State;

use crate::models::{CaptureStats, Dashboard, LiveWindow, RangeQuery};
use crate::{sampler, storage, AppState};

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct UserSettings {
    close_to_tray: Option<bool>,
    sample_interval_ms: Option<u64>,
}

#[tauri::command]
pub fn get_dashboard(
    state: State<'_, AppState>,
    range: Option<RangeQuery>,
) -> Result<Dashboard, String> {
    storage::dashboard(
        &state.db_path,
        &state.storage_location,
        state.monitoring.load(Ordering::Relaxed),
        sampler::IDLE_THRESHOLD_SECONDS,
        state.sample_interval_ms.load(Ordering::Relaxed),
        range,
        sampler::live_windows().unwrap_or_default(),
    )
}

#[tauri::command]
pub fn record_now(state: State<'_, AppState>) -> Result<CaptureStats, String> {
    sampler::capture_current(
        &state.db_path,
        sampler::normalize_sample_interval_ms(state.sample_interval_ms.load(Ordering::Relaxed)),
    )
}

#[tauri::command]
pub fn list_live_windows() -> Result<Vec<LiveWindow>, String> {
    sampler::live_windows()
}

#[tauri::command]
pub fn set_monitoring(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    state.monitoring.store(enabled, Ordering::Relaxed);
    Ok(enabled)
}

#[tauri::command]
pub fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[tauri::command]
pub fn get_close_to_tray(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.close_to_tray.load(Ordering::Relaxed))
}

#[tauri::command]
pub fn set_close_to_tray(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    state.close_to_tray.store(enabled, Ordering::Relaxed);
    save_close_to_tray(&state.settings_path, enabled)?;
    Ok(enabled)
}

#[tauri::command]
pub fn get_sample_interval_ms(state: State<'_, AppState>) -> Result<u64, String> {
    Ok(sampler::normalize_sample_interval_ms(
        state.sample_interval_ms.load(Ordering::Relaxed),
    ))
}

#[tauri::command]
pub fn set_sample_interval_ms(state: State<'_, AppState>, interval_ms: u64) -> Result<u64, String> {
    let normalized = sampler::normalize_sample_interval_ms(interval_ms);
    state
        .sample_interval_ms
        .store(normalized, Ordering::Relaxed);
    save_sample_interval_ms(&state.settings_path, normalized)?;
    Ok(normalized)
}

#[tauri::command]
pub fn get_startup_enabled() -> Result<bool, String> {
    startup_enabled()
}

#[tauri::command]
pub fn set_startup_enabled(enabled: bool) -> Result<bool, String> {
    set_startup(enabled)?;
    startup_enabled()
}

pub fn load_close_to_tray(settings_path: &Path) -> bool {
    load_settings(settings_path)
        .and_then(|settings| settings.close_to_tray)
        .unwrap_or(true)
}

pub fn load_sample_interval_ms(settings_path: &Path) -> u64 {
    load_settings(settings_path)
        .and_then(|settings| settings.sample_interval_ms)
        .map(sampler::normalize_sample_interval_ms)
        .unwrap_or(sampler::DEFAULT_SAMPLE_INTERVAL_MS)
}

fn save_close_to_tray(settings_path: &Path, enabled: bool) -> Result<(), String> {
    let mut settings = load_settings(settings_path).unwrap_or_default();
    settings.close_to_tray = Some(enabled);
    save_settings(settings_path, &settings)
}

fn save_sample_interval_ms(settings_path: &Path, interval_ms: u64) -> Result<(), String> {
    let mut settings = load_settings(settings_path).unwrap_or_default();
    settings.sample_interval_ms = Some(sampler::normalize_sample_interval_ms(interval_ms));
    save_settings(settings_path, &settings)
}

fn load_settings(settings_path: &Path) -> Option<UserSettings> {
    fs::read_to_string(settings_path)
        .ok()
        .and_then(|contents| serde_json::from_str::<UserSettings>(&contents).ok())
}

fn save_settings(settings_path: &Path, settings: &UserSettings) -> Result<(), String> {
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let contents = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(settings_path, contents).map_err(|error| error.to_string())
}

#[cfg(windows)]
const STARTUP_VALUE_NAME: &str = "PCTime";

#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(windows)]
fn startup_enabled() -> Result<bool, String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(RUN_KEY)
        .map_err(|error| error.to_string())?;
    let command = key
        .get_value::<String, _>(STARTUP_VALUE_NAME)
        .unwrap_or_default();
    let expected = current_exe_command()?;

    Ok(normalize_command(&command) == normalize_command(&expected))
}

#[cfg(windows)]
fn set_startup(enabled: bool) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(RUN_KEY)
        .map_err(|error| error.to_string())?;

    if enabled {
        key.set_value(STARTUP_VALUE_NAME, &current_exe_command()?)
            .map_err(|error| error.to_string())
    } else {
        match key.delete_value(STARTUP_VALUE_NAME) {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(windows)]
fn current_exe_command() -> Result<String, String> {
    std::env::current_exe()
        .map(|path| format!("\"{}\"", path.display()))
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
fn normalize_command(command: &str) -> String {
    command.trim().trim_matches('"').to_ascii_lowercase()
}

#[cfg(not(windows))]
fn startup_enabled() -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(windows))]
fn set_startup(_enabled: bool) -> Result<(), String> {
    Err("Start at login is only implemented on Windows for now".to_string())
}
