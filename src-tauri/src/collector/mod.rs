#[cfg(not(target_os = "windows"))]
mod fallback;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(target_os = "windows"))]
pub use fallback::{collect_windows, idle_seconds};
#[cfg(target_os = "windows")]
pub use windows::{collect_windows, idle_seconds};
