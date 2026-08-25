use tauri::{Manager, State};

use crate::{
    app_state::AppState,
    error::{AppError, AppResult},
    settings::WindowSizePreset,
};

use super::sizing::{
    apply_requested_window_size, monitor_metrics, size_info, WindowSizeInfo, WindowSizeRequest,
    MIN_HEIGHT, MIN_WIDTH,
};

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
    apply_requested_window_size(&window, request)
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
