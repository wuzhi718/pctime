use std::path::Path;

use chrono::Local;

use crate::collector;
use crate::models::{CaptureStats, LiveWindow, WindowSample};
use crate::storage;
use crate::visibility;

pub const IDLE_THRESHOLD_SECONDS: u64 = 300;

pub fn capture_current(db_path: &Path, duration_ms: u64) -> Result<CaptureStats, String> {
    let captured_at = Local::now().timestamp_millis();
    let idle_seconds = collector::idle_seconds().unwrap_or(0);
    let idle = idle_seconds >= IDLE_THRESHOLD_SECONDS;
    let samples = if idle { Vec::new() } else { current_samples()? };

    storage::insert_samples(
        db_path,
        captured_at,
        duration_ms,
        idle,
        idle_seconds,
        &samples,
    )?;

    Ok(CaptureStats {
        captured_at,
        windows_recorded: samples.len(),
        idle,
        idle_seconds,
    })
}

pub fn live_windows() -> Result<Vec<LiveWindow>, String> {
    Ok(current_samples()?
        .into_iter()
        .map(|sample| LiveWindow {
            app_name: sample.app_name,
            window_title: sample.window_title,
            category: sample.category,
            visible_area: sample.visible_area,
            visible_share: sample.visible_share,
            focused: sample.focused,
        })
        .collect())
}

fn current_samples() -> Result<Vec<WindowSample>, String> {
    let raw = collector::collect_windows()?;
    Ok(visibility::compute_visible_windows(raw))
}
