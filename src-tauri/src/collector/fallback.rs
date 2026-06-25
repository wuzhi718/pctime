use crate::models::{RawWindow, Rect};

pub fn collect_windows() -> Result<Vec<RawWindow>, String> {
    Ok(vec![RawWindow {
        app_name: "PCTime Preview".to_string(),
        title: "Visible-window tracking is implemented for Windows first".to_string(),
        process_path: String::new(),
        pid: 0,
        rect: Rect {
            left: 0,
            top: 0,
            right: 1280,
            bottom: 720,
        },
        focused: true,
    }])
}

pub fn idle_seconds() -> Result<u64, String> {
    Ok(0)
}
