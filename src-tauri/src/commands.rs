use std::sync::atomic::Ordering;

use tauri::State;

use crate::models::{CaptureStats, Dashboard, LiveWindow, RangeQuery};
use crate::{sampler, storage, AppState};

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
        range,
        sampler::live_windows().unwrap_or_default(),
    )
}

#[tauri::command]
pub fn record_now(state: State<'_, AppState>) -> Result<CaptureStats, String> {
    sampler::capture_current(&state.db_path, 1_000)
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
pub fn get_startup_enabled() -> Result<bool, String> {
    startup_enabled()
}

#[tauri::command]
pub fn set_startup_enabled(enabled: bool) -> Result<bool, String> {
    set_startup(enabled)?;
    startup_enabled()
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
    let key = hkcu.open_subkey(RUN_KEY).map_err(|error| error.to_string())?;
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
