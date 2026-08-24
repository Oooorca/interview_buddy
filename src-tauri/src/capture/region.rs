use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::jpeg::JpegEncoder, ColorType};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use xcap::Monitor;

use crate::app_state::{AppState, RegionCaptureSession};
#[cfg(target_os = "windows")]
use crate::window::query_display_affinity;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegionSelection {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureResult {
    data_url: String,
}

fn encode_capture(image: image::RgbaImage) -> Result<CaptureResult, String> {
    let rgb = image::DynamicImage::ImageRgba8(image).to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 82)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ColorType::Rgb8.into(),
        )
        .map_err(|error| error.to_string())?;
    Ok(CaptureResult {
        data_url: format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg)),
    })
}

fn cursor_position() -> Result<(i32, i32), String> {
    use mouse_position::mouse_position::Mouse;

    match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => Ok((x, y)),
        Mouse::Error => Err("读取鼠标位置失败".into()),
    }
}

fn capture_absolute_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<CaptureResult, String> {
    if width < 2 || height < 2 {
        return Err("截图区域太小".into());
    }
    let monitor = Monitor::from_point(x, y).map_err(|error| error.to_string())?;
    let monitor_x = monitor.x().map_err(|error| error.to_string())?;
    let monitor_y = monitor.y().map_err(|error| error.to_string())?;
    let full = monitor.capture_image().map_err(|error| error.to_string())?;
    let max_x = full.width() as i32;
    let max_y = full.height() as i32;
    let left = (x - monitor_x).clamp(0, max_x);
    let top = (y - monitor_y).clamp(0, max_y);
    let right = (x.saturating_add(width as i32) - monitor_x).clamp(0, max_x);
    let bottom = (y.saturating_add(height as i32) - monitor_y).clamp(0, max_y);
    let cropped_width = left.abs_diff(right);
    let cropped_height = top.abs_diff(bottom);
    if cropped_width < 2 || cropped_height < 2 {
        return Err("截图区域超出当前显示器或尺寸过小".into());
    }
    let cropped = image::imageops::crop_imm(
        &full,
        left.min(right) as u32,
        top.min(bottom) as u32,
        cropped_width,
        cropped_height,
    )
    .to_image();
    encode_capture(cropped)
}

fn take_region_session(state: &AppState) -> Option<RegionCaptureSession> {
    state
        .region_capture
        .lock()
        .ok()
        .and_then(|mut session| session.take())
}

pub(crate) fn restore_main_after_region(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some(session) = take_region_session(&state) else {
        return;
    };
    if session.restore_main_window {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    let _ = app.emit_to("main", "region-capture-cancelled", ());
}

#[tauri::command]
pub(crate) async fn open_region_selector(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if app.get_webview_window("region-selector").is_some() {
        return Err("区域截图选择器已经打开".into());
    }

    let cursor = cursor_position()?;
    let monitor = Monitor::from_point(cursor.0, cursor.1).map_err(|error| error.to_string())?;
    let monitor_x = monitor.x().map_err(|error| error.to_string())?;
    let monitor_y = monitor.y().map_err(|error| error.to_string())?;
    let monitor_width = monitor.width().map_err(|error| error.to_string())?;
    let monitor_height = monitor.height().map_err(|error| error.to_string())?;
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "没有找到主窗口".to_string())?;
    let restore_main_window = main.is_visible().unwrap_or(false);
    if restore_main_window {
        main.hide().map_err(|error| error.to_string())?;
    }
    *state
        .region_capture
        .lock()
        .map_err(|error| error.to_string())? = Some(RegionCaptureSession {
        restore_main_window,
    });

    let selector_builder = tauri::WebviewWindowBuilder::new(
        &app,
        "region-selector",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("选择截图区域")
    .inner_size(640.0, 480.0)
    .visible(false)
    .focused(true)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .content_protected(true);
    #[cfg(target_os = "windows")]
    let selector_builder = selector_builder.data_directory(state.storage.active_webview_path());
    let selector = match selector_builder.build() {
        Ok(selector) => selector,
        Err(error) => {
            restore_main_after_region(&app);
            return Err(format!("无法创建区域截图选择器：{error}"));
        }
    };

    let configured = selector
        .set_position(tauri::PhysicalPosition::new(monitor_x, monitor_y))
        .and_then(|_| selector.set_size(tauri::PhysicalSize::new(monitor_width, monitor_height)))
        .and_then(|_| selector.show())
        .and_then(|_| selector.set_focus());
    if let Err(error) = configured {
        let _ = selector.close();
        restore_main_after_region(&app);
        return Err(format!("无法显示区域截图选择器：{error}"));
    }

    #[cfg(target_os = "windows")]
    match query_display_affinity(&selector) {
        Ok(affinity) => eprintln!("Region selector display affinity: 0x{affinity:X}"),
        Err(error) => eprintln!("Region selector display affinity check failed: {error}"),
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn complete_region_selection(
    selection: RegionSelection,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !selection.x.is_finite()
        || !selection.y.is_finite()
        || !selection.width.is_finite()
        || !selection.height.is_finite()
        || selection.width < 2.0
        || selection.height < 2.0
    {
        return Err("截图区域无效或尺寸过小".into());
    }
    let selector = app
        .get_webview_window("region-selector")
        .ok_or_else(|| "区域截图选择器已经关闭".to_string())?;
    let position = selector
        .outer_position()
        .map_err(|error| error.to_string())?;
    let scale = selector.scale_factor().map_err(|error| error.to_string())?;
    let x = position
        .x
        .saturating_add((selection.x * scale).round() as i32);
    let y = position
        .y
        .saturating_add((selection.y * scale).round() as i32);
    let right = position
        .x
        .saturating_add(((selection.x + selection.width) * scale).round() as i32);
    let bottom = position
        .y
        .saturating_add(((selection.y + selection.height) * scale).round() as i32);
    let width = x.abs_diff(right);
    let height = y.abs_diff(bottom);
    let session = take_region_session(&state).ok_or_else(|| "区域截图会话已经结束".to_string())?;

    let _ = selector.hide();
    let capture = match tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(180));
        capture_absolute_region(x.min(right), y.min(bottom), width, height)
    })
    .await
    {
        Ok(result) => result,
        Err(error) => Err(format!("区域截图任务失败：{error}")),
    };
    let _ = selector.close();
    if session.restore_main_window {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    match capture {
        Ok(result) => app
            .emit_to("main", "region-captured", result)
            .map_err(|error| error.to_string()),
        Err(error) => {
            let _ = app.emit_to("main", "region-capture-error", error.clone());
            Err(error)
        }
    }
}

#[tauri::command]
pub(crate) fn cancel_region_selection(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session = take_region_session(&state);
    let close_result = app
        .get_webview_window("region-selector")
        .map(|selector| selector.close().map_err(|error| error.to_string()))
        .transpose();
    if session.is_some_and(|item| item.restore_main_window) {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    let emit_result = app
        .emit_to("main", "region-capture-cancelled", ())
        .map_err(|error| error.to_string());
    close_result?;
    emit_result
}
