use crate::models::RawWindow;

const IGNORED_APPS: &[&str] = &["nvidia overlay", "nvidia share"];

pub fn should_ignore_raw_window(window: &RawWindow) -> bool {
    is_ignored_app(&window.app_name)
}

fn is_ignored_app(app_name: &str) -> bool {
    let normalized = app_name.trim().trim_end_matches(".exe").to_lowercase();

    IGNORED_APPS.iter().any(|app| normalized == *app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Rect;

    #[test]
    fn ignores_nvidia_overlay_processes() {
        assert!(is_ignored_app("NVIDIA Overlay.exe"));
        assert!(is_ignored_app("NVIDIA Share.exe"));
        assert!(!is_ignored_app("NVIDIA App.exe"));
    }

    #[test]
    fn ignores_raw_overlay_windows() {
        let window = RawWindow {
            app_name: "NVIDIA Overlay.exe".to_string(),
            title: "NVIDIA Overlay".to_string(),
            process_path: String::new(),
            pid: 1,
            rect: Rect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            },
            focused: false,
        };

        assert!(should_ignore_raw_window(&window));
    }
}
