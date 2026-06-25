use std::mem::size_of;
use std::path::Path;

use crate::models::{RawWindow, Rect};
use ::windows::core::PWSTR;
use ::windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use ::windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use ::windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use ::windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetTopWindow, GetWindow, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, GW_HWNDNEXT,
};

pub fn collect_windows() -> Result<Vec<RawWindow>, String> {
    let mut windows = Vec::new();

    unsafe {
        let foreground = GetForegroundWindow();
        let mut hwnd = GetTopWindow(None).ok();

        while let Some(current) = hwnd {
            if current.0.is_null() {
                break;
            }

            if should_collect(current) {
                if let Some(window) = collect_window(current, current == foreground) {
                    windows.push(window);
                }
            }

            hwnd = GetWindow(current, GW_HWNDNEXT).ok();
        }
    }

    Ok(windows)
}

pub fn idle_seconds() -> Result<u64, String> {
    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };

        if !GetLastInputInfo(&mut info).as_bool() {
            return Err("GetLastInputInfo failed".to_string());
        }

        let now = ::windows::Win32::System::SystemInformation::GetTickCount64();
        Ok(now.saturating_sub(info.dwTime as u64) / 1_000)
    }
}

unsafe fn should_collect(hwnd: HWND) -> bool {
    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return false;
    }

    if is_cloaked(hwnd) {
        return false;
    }

    rect_for_window(hwnd).is_some_and(|rect| rect.area() > 0)
}

unsafe fn collect_window(hwnd: HWND, focused: bool) -> Option<RawWindow> {
    let title = window_title(hwnd);
    let rect = rect_for_window(hwnd)?;

    let mut pid = 0_u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    let process_path = process_path(pid).unwrap_or_default();
    let app_name = app_name_from_path(&process_path).unwrap_or_else(|| {
        if title.is_empty() {
            "Unknown".to_string()
        } else {
            title.clone()
        }
    });

    if app_name == "Unknown" && title.is_empty() {
        return None;
    }

    Some(RawWindow {
        app_name,
        title,
        process_path,
        pid,
        rect,
        focused,
    })
}

unsafe fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0_u32;
    DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut _ as *mut _,
        size_of::<u32>() as u32,
    )
    .is_ok()
        && cloaked != 0
}

unsafe fn rect_for_window(hwnd: HWND) -> Option<Rect> {
    let mut native = RECT::default();
    let dwm_result = DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut native as *mut _ as *mut _,
        size_of::<RECT>() as u32,
    );

    if dwm_result.is_err() && GetWindowRect(hwnd, &mut native).is_err() {
        return None;
    }

    Some(Rect {
        left: native.left,
        top: native.top,
        right: native.right,
        bottom: native.bottom,
    })
}

unsafe fn window_title(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return String::new();
    }

    let mut buffer = vec![0_u16; len as usize + 1];
    let copied = GetWindowTextW(hwnd, &mut buffer);

    String::from_utf16_lossy(&buffer[..copied as usize])
        .trim()
        .to_string()
}

unsafe fn process_path(pid: u32) -> Option<String> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buffer = vec![0_u16; 32_768];
    let mut size = buffer.len() as u32;

    let result = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_WIN32,
        PWSTR(buffer.as_mut_ptr()),
        &mut size,
    );

    let _ = CloseHandle(handle);

    result
        .ok()
        .map(|_| String::from_utf16_lossy(&buffer[..size as usize]))
}

fn app_name_from_path(process_path: &str) -> Option<String> {
    Path::new(process_path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
}
