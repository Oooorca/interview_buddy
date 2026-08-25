use objc2_core_graphics::{CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
use xcap::Monitor;

use crate::{app_state::AppState, window::configure_platform_overlay};

use super::super::{
    crop::{encode_capture, scaled_selection_bounds},
    model::{CaptureError, CaptureResult, MonitorGeometry, RegionSelection},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct CaptureContext {
    monitor_x: i32,
    monitor_y: i32,
    monitor_scale: f64,
}

pub(crate) struct CaptureRequest {
    context: CaptureContext,
    selection: RegionSelection,
}

const PERMISSION_REQUIRED: &str = "macOS 尚未向当前版本的 Interview Buddy 授予屏幕录制权限。系统授权窗口已打开；授权后请彻底退出并重新启动应用。临时签名构建每次更新后都需要重新授权。";

pub(crate) fn ensure_permission() -> Result<(), CaptureError> {
    if CGPreflightScreenCaptureAccess() {
        return Ok(());
    }
    if CGRequestScreenCaptureAccess() && CGPreflightScreenCaptureAccess() {
        return Ok(());
    }
    Err(CaptureError::Operation(PERMISSION_REQUIRED.into()))
}

pub(crate) fn monitor_at(
    cursor: (i32, i32),
) -> Result<(MonitorGeometry, CaptureContext), CaptureError> {
    let monitor = Monitor::from_point(cursor.0, cursor.1)
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    let geometry = MonitorGeometry {
        x: monitor
            .x()
            .map_err(|error| CaptureError::Operation(error.to_string()))?,
        y: monitor
            .y()
            .map_err(|error| CaptureError::Operation(error.to_string()))?,
        width: monitor
            .width()
            .map_err(|error| CaptureError::Operation(error.to_string()))?,
        height: monitor
            .height()
            .map_err(|error| CaptureError::Operation(error.to_string()))?,
    };
    let context = CaptureContext {
        monitor_x: geometry.x,
        monitor_y: geometry.y,
        monitor_scale: monitor
            .scale_factor()
            .map_err(|error| CaptureError::Operation(error.to_string()))?
            as f64,
    };
    Ok((geometry, context))
}

pub(crate) fn create_selector(
    app: &tauri::AppHandle,
    _state: &AppState,
    monitor: MonitorGeometry,
) -> Result<tauri::WebviewWindow, CaptureError> {
    let selector = tauri::WebviewWindowBuilder::new(
        app,
        "region-selector",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Interview Buddy")
    .visible(false)
    .focused(true)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .content_protected(true)
    .visible_on_all_workspaces(true)
    .position(monitor.x as f64, monitor.y as f64)
    .inner_size(monitor.width as f64, monitor.height as f64)
    .build()
    .map_err(|error| CaptureError::CreateSelector(error.to_string()))?;
    configure_platform_overlay(&selector)
        .map_err(|error| CaptureError::ShowSelector(error.to_string()))?;
    selector
        .show()
        .and_then(|_| selector.set_focus())
        .map_err(|error| CaptureError::ShowSelector(error.to_string()))?;
    Ok(selector)
}

pub(crate) fn prepare_request(
    _selector: &tauri::WebviewWindow,
    context: CaptureContext,
    selection: RegionSelection,
) -> Result<CaptureRequest, CaptureError> {
    Ok(CaptureRequest { context, selection })
}

pub(crate) fn capture(request: CaptureRequest) -> Result<CaptureResult, CaptureError> {
    let monitor = Monitor::from_point(
        request.context.monitor_x.saturating_add(1),
        request.context.monitor_y.saturating_add(1),
    )
    .map_err(|error| CaptureError::Operation(error.to_string()))?;
    let full = monitor
        .capture_image()
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    let bounds = scaled_selection_bounds(
        &request.selection,
        request.context.monitor_scale,
        Some((full.width(), full.height())),
    )?;
    encode_capture(
        image::imageops::crop_imm(
            &full,
            bounds.left as u32,
            bounds.top as u32,
            bounds.width,
            bounds.height,
        )
        .to_image(),
    )
}
