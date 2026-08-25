use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::{
    app_state::AppState,
    error::{AppError, AppResult},
    settings::{AppSettings, WindowSizePreset},
};

const MIN_WIDTH: f64 = 680.0;
const MIN_HEIGHT: f64 = 340.0;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowSizeRequest {
    preset: WindowSizePreset,
    custom_width: u32,
    custom_height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowSizeInfo {
    preset: WindowSizePreset,
    width: u32,
    height: u32,
    monitor_width: u32,
    monitor_height: u32,
    scale_factor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LogicalWindowSize {
    width: f64,
    height: f64,
}

fn round_to_ten(value: f64) -> f64 {
    (value / 10.0).round() * 10.0
}

fn target_size(
    preset: WindowSizePreset,
    work_width: f64,
    work_height: f64,
    custom_width: u32,
    custom_height: u32,
) -> LogicalWindowSize {
    let (width, height, min_width, min_height, max_width, max_height): (
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
    ) = match preset {
        WindowSizePreset::Compact => (
            work_width * 0.35,
            work_height * 0.40,
            680.0,
            400.0,
            1_120.0,
            680.0,
        ),
        WindowSizePreset::Standard => (
            work_width * 0.40,
            work_height * 0.46,
            800.0,
            480.0,
            1_360.0,
            820.0,
        ),
        WindowSizePreset::Spacious => (
            work_width * 0.46,
            work_height * 0.52,
            880.0,
            540.0,
            1_600.0,
            960.0,
        ),
        WindowSizePreset::Custom => (
            f64::from(custom_width),
            f64::from(custom_height),
            MIN_WIDTH,
            MIN_HEIGHT,
            3_840.0,
            2_160.0,
        ),
    };
    let usable_width = (work_width * 0.96).max(1.0);
    let usable_height = (work_height * 0.94).max(1.0);
    let lower_width = min_width.min(usable_width);
    let lower_height = min_height.min(usable_height);
    LogicalWindowSize {
        width: round_to_ten(width).clamp(lower_width, max_width.min(usable_width)),
        height: round_to_ten(height).clamp(lower_height, max_height.min(usable_height)),
    }
}

fn monitor_metrics(
    window: &tauri::WebviewWindow,
) -> Result<(tauri::Monitor, f64, f64, f64), AppError> {
    let monitor = window
        .current_monitor()
        .map_err(|error| format!("读取当前显示器失败：{error}"))?
        .or(window
            .primary_monitor()
            .map_err(|error| format!("读取主显示器失败：{error}"))?)
        .ok_or_else(|| AppError::from("没有找到窗口所在显示器"))?;
    let scale_factor = monitor.scale_factor();
    let work_size = monitor.work_area().size.to_logical::<f64>(scale_factor);
    Ok((monitor, work_size.width, work_size.height, scale_factor))
}

fn size_info(window: &tauri::WebviewWindow, preset: WindowSizePreset) -> AppResult<WindowSizeInfo> {
    let (_, monitor_width, monitor_height, scale_factor) = monitor_metrics(window)?;
    let actual = window
        .inner_size()
        .map_err(|error| format!("读取窗口尺寸失败：{error}"))?
        .to_logical::<f64>(scale_factor);
    Ok(WindowSizeInfo {
        preset,
        width: actual.width.round() as u32,
        height: actual.height.round() as u32,
        monitor_width: monitor_width.round() as u32,
        monitor_height: monitor_height.round() as u32,
        scale_factor,
    })
}

fn resize_window(
    window: &tauri::WebviewWindow,
    request: WindowSizeRequest,
    center: bool,
) -> AppResult<WindowSizeInfo> {
    let (_, work_width, work_height, _) = monitor_metrics(window)?;
    let target = target_size(
        request.preset,
        work_width,
        work_height,
        request.custom_width,
        request.custom_height,
    );
    window
        .set_size(tauri::LogicalSize::new(target.width, target.height))
        .map_err(|error| format!("调整窗口尺寸失败：{error}"))?;
    if center {
        window
            .center()
            .map_err(|error| format!("居中窗口失败：{error}"))?;
    }
    size_info(window, request.preset)
}

pub(crate) fn apply_saved_window_size(
    window: &tauri::WebviewWindow,
    settings: &AppSettings,
) -> AppResult<WindowSizeInfo> {
    resize_window(
        window,
        WindowSizeRequest {
            preset: settings.window_size_preset,
            custom_width: settings.custom_window_width,
            custom_height: settings.custom_window_height,
        },
        true,
    )
}

#[tauri::command]
pub(crate) fn window_size_info(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<WindowSizeInfo> {
    let preset = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .window_size_preset;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::from("没有找到主窗口"))?;
    size_info(&window, preset)
}

#[tauri::command]
pub(crate) fn apply_window_size(
    request: WindowSizeRequest,
    app: tauri::AppHandle,
) -> AppResult<WindowSizeInfo> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::from("没有找到主窗口"))?;
    resize_window(&window, request, false)
}

#[tauri::command]
pub(crate) fn remember_window_size(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<WindowSizeInfo> {
    crate::settings::commands::ensure_security_ready(&state)?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::from("没有找到主窗口"))?;
    let (_, _, _, scale_factor) = monitor_metrics(&window)?;
    let actual = window
        .inner_size()
        .map_err(|error| format!("读取窗口尺寸失败：{error}"))?
        .to_logical::<f64>(scale_factor);
    let mut updated = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    updated.window_size_preset = WindowSizePreset::Custom;
    updated.custom_window_width = actual.width.round().clamp(MIN_WIDTH, 3_840.0) as u32;
    updated.custom_window_height = actual.height.round().clamp(MIN_HEIGHT, 2_160.0) as u32;
    state
        .settings_store
        .read()
        .map_err(|error| error.to_string())?
        .as_ref()
        .ok_or_else(|| AppError::from("安全设置存储不可用"))?
        .save(&updated)?;
    *state.settings.write().map_err(|error| error.to_string())? = updated;
    size_info(&window, WindowSizePreset::Custom)
}

#[tauri::command]
pub(crate) fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

pub(crate) fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn query_display_affinity(window: &tauri::WebviewWindow) -> Result<u32, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowDisplayAffinity;
    let handle = window.window_handle().map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return Err("当前窗口不是 Win32 窗口".into());
    };
    let hwnd = HWND(win32.hwnd.get() as *mut std::ffi::c_void);
    let mut affinity = 0u32;
    unsafe { GetWindowDisplayAffinity(hwnd, &mut affinity) }
        .map_err(|error| format!("读取窗口捕获保护失败：{error}"))?;
    Ok(affinity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(preset: WindowSizePreset, width: f64, height: f64) -> (u32, u32) {
        let result = target_size(preset, width, height, 880, 540);
        (result.width as u32, result.height as u32)
    }

    #[test]
    fn presets_scale_across_mainstream_workspaces() {
        assert_eq!(size(WindowSizePreset::Compact, 1920.0, 1040.0), (680, 420));
        assert_eq!(size(WindowSizePreset::Standard, 1920.0, 1040.0), (800, 480));
        assert_eq!(size(WindowSizePreset::Spacious, 1920.0, 1040.0), (880, 540));
        assert_eq!(size(WindowSizePreset::Compact, 2560.0, 1400.0), (900, 560));
        assert_eq!(
            size(WindowSizePreset::Standard, 2560.0, 1400.0),
            (1020, 640)
        );
        assert_eq!(
            size(WindowSizePreset::Spacious, 2560.0, 1400.0),
            (1180, 730)
        );
        assert_eq!(size(WindowSizePreset::Compact, 3840.0, 2100.0), (1120, 680));
        assert_eq!(
            size(WindowSizePreset::Standard, 3840.0, 2100.0),
            (1360, 820)
        );
        assert_eq!(
            size(WindowSizePreset::Spacious, 3840.0, 2100.0),
            (1600, 960)
        );
    }

    #[test]
    fn custom_size_is_clamped_to_safe_workspace() {
        let tiny = target_size(WindowSizePreset::Custom, 1000.0, 700.0, 4000, 3000);
        assert_eq!(tiny.width, 960.0);
        assert_eq!(tiny.height, 658.0);
        let minimum = target_size(WindowSizePreset::Custom, 1920.0, 1040.0, 100, 100);
        assert_eq!(minimum.width, 680.0);
        assert_eq!(minimum.height, 340.0);
    }
}
