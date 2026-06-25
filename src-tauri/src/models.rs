use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(self) -> i64 {
        (self.right - self.left).max(0) as i64
    }

    pub fn height(self) -> i64 {
        (self.bottom - self.top).max(0) as i64
    }

    pub fn area(self) -> i64 {
        self.width() * self.height()
    }

    pub fn intersection(self, other: Rect) -> Option<Rect> {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right.min(other.right);
        let bottom = self.bottom.min(other.bottom);

        (right > left && bottom > top).then_some(Rect {
            left,
            top,
            right,
            bottom,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RawWindow {
    pub app_name: String,
    pub title: String,
    pub process_path: String,
    pub pid: u32,
    pub rect: Rect,
    pub focused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowSample {
    pub app_name: String,
    pub window_title: String,
    pub process_path: String,
    pub pid: u32,
    pub rect: Rect,
    pub category: String,
    pub visible_area: i64,
    pub visible_share: f64,
    pub focused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveWindow {
    pub app_name: String,
    pub window_title: String,
    pub category: String,
    pub visible_area: i64,
    pub visible_share: f64,
    pub focused: bool,
}

#[derive(Debug, Serialize)]
pub struct CaptureStats {
    pub captured_at: i64,
    pub windows_recorded: usize,
    pub idle: bool,
    pub idle_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct MetricCard {
    pub label: String,
    pub value_seconds: i64,
    pub helper: String,
}

#[derive(Debug, Serialize)]
pub struct CategorySummary {
    pub category: String,
    pub seconds: i64,
    pub focus_seconds: i64,
    pub share: f64,
    pub sample_count: i64,
}

#[derive(Debug, Serialize)]
pub struct AppSummary {
    pub app_name: String,
    pub category: String,
    pub seconds: i64,
    pub focus_seconds: i64,
    pub share: f64,
    pub last_seen: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WindowSummary {
    pub app_name: String,
    pub window_title: String,
    pub category: String,
    pub seconds: i64,
    pub focus_seconds: i64,
    pub share: f64,
}

#[derive(Debug, Serialize)]
pub struct TimelinePoint {
    pub hour: String,
    pub active_seconds: i64,
    pub idle_seconds: i64,
    pub top_apps: Vec<TimelineApp>,
}

#[derive(Debug, Serialize)]
pub struct TimelineApp {
    pub app_name: String,
    pub category: String,
    pub seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RangeQuery {
    pub preset: String,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct RangeInfo {
    pub preset: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub label: String,
    pub bucket: String,
}

#[derive(Debug, Serialize)]
pub struct CaptureHealth {
    pub monitoring: bool,
    pub database_path: String,
    pub storage_location: String,
    pub database_size_bytes: u64,
    pub estimated_daily_bytes: u64,
    pub total_rows: i64,
    pub samples_today: i64,
    pub last_capture_at: Option<String>,
    pub idle_threshold_seconds: u64,
    pub sample_interval_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct Dashboard {
    pub generated_at: String,
    pub range: RangeInfo,
    pub active_seconds: i64,
    pub idle_seconds: i64,
    pub focus_seconds: i64,
    pub unclassified_seconds: i64,
    pub cards: Vec<MetricCard>,
    pub categories: Vec<CategorySummary>,
    pub apps: Vec<AppSummary>,
    pub windows: Vec<WindowSummary>,
    pub timeline: Vec<TimelinePoint>,
    pub live_windows: Vec<LiveWindow>,
    pub health: CaptureHealth,
}
