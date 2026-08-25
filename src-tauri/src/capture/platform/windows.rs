use xcap::Monitor;

use crate::{app_state::AppState, window::configure_platform_overlay};

use super::{
    super::{
        crop::scaled_selection_bounds,
        model::{CaptureError, CaptureResult, MonitorGeometry, RegionSelection},
    },
    non_macos::capture_absolute_region,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct CaptureContext;

pub(crate) struct CaptureRequest {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

pub(crate) fn ensure_permission() -> Result<(), CaptureError> {
    Ok(())
}

pub(crate) fn monitor_at(
    cursor: (i32, i32),
) -> Result<(MonitorGeometry, CaptureContext), CaptureError> {
    let monitor = Monitor::from_point(cursor.0, cursor.1)
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    Ok((
        MonitorGeometry {
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
        },
        CaptureContext,
    ))
}

pub(crate) fn create_selector(
    app: &tauri::AppHandle,
    state: &AppState,
    monitor: MonitorGeometry,
) -> Result<tauri::WebviewWindow, CaptureError> {
    let selector = tauri::WebviewWindowBuilder::new(
        app,
        "region-selector",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Interview Buddy")
    .inner_size(640.0, 480.0)
    .visible(false)
    .focused(true)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .content_protected(true)
    .data_directory(state.storage.active_webview_path())
    .build()
    .map_err(|error| CaptureError::CreateSelector(error.to_string()))?;
    selector
        .set_position(tauri::PhysicalPosition::new(monitor.x, monitor.y))
        .and_then(|_| selector.set_size(tauri::PhysicalSize::new(monitor.width, monitor.height)))
        .and_then(|_| selector.show())
        .and_then(|_| selector.set_focus())
        .map_err(|error| CaptureError::ShowSelector(error.to_string()))?;
    configure_platform_overlay(&selector)
        .map_err(|error| CaptureError::ShowSelector(error.to_string()))?;
    Ok(selector)
}

pub(crate) fn prepare_request(
    selector: &tauri::WebviewWindow,
    _context: CaptureContext,
    selection: RegionSelection,
) -> Result<CaptureRequest, CaptureError> {
    let position = selector
        .outer_position()
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    let scale = selector
        .scale_factor()
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    let bounds = scaled_selection_bounds(&selection, scale, None)?;
    Ok(CaptureRequest {
        x: position.x.saturating_add(bounds.left),
        y: position.y.saturating_add(bounds.top),
        width: bounds.width,
        height: bounds.height,
    })
}

pub(crate) fn capture(request: CaptureRequest) -> Result<CaptureResult, CaptureError> {
    capture_absolute_region(request.x, request.y, request.width, request.height)
}
