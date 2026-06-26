use std::path::Path;

use chrono::Local;

use crate::collector;
use crate::models::{CaptureStats, LiveWindow, WindowSample};
use crate::storage;
use crate::visibility;

pub const IDLE_THRESHOLD_SECONDS: u64 = 300;
pub const DEFAULT_SAMPLE_INTERVAL_MS: u64 = 5_000;
pub const MIN_SAMPLE_INTERVAL_MS: u64 = 1_000;
pub const MAX_SAMPLE_INTERVAL_MS: u64 = 60_000;

pub fn normalize_sample_interval_ms(interval_ms: u64) -> u64 {
    interval_ms.clamp(MIN_SAMPLE_INTERVAL_MS, MAX_SAMPLE_INTERVAL_MS)
}

pub fn capture_current(
    db_path: &Path,
    duration_ms: u64,
    always_active_pattern: &str,
) -> Result<CaptureStats, String> {
    let captured_at = Local::now().timestamp_millis();
    let idle_seconds = collector::idle_seconds().unwrap_or(0);
    let input_idle = idle_seconds >= IDLE_THRESHOLD_SECONDS;
    let samples = if input_idle && always_active_pattern.trim().is_empty() {
        Vec::new()
    } else {
        current_samples()?
    };
    let idle = input_idle && !matches_always_active(&samples, always_active_pattern);
    let samples = if idle { Vec::new() } else { samples };

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

fn matches_always_active(samples: &[WindowSample], pattern: &str) -> bool {
    let tokens = pattern
        .split('|')
        .map(|token| token.trim().to_lowercase())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        return false;
    }

    samples.iter().any(|sample| {
        let haystack = format!(
            "{}\n{}\n{}\n{}",
            sample.app_name, sample.window_title, sample.process_path, sample.category
        )
        .to_lowercase();
        tokens.iter().any(|token| haystack.contains(token))
    })
}
