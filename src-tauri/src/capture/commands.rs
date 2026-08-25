use std::time::Duration;

use tauri::{Emitter, Manager, State};

use crate::{app_state::AppState, error::AppResult};

use super::{
    model::{CaptureError, RegionCaptureSession, RegionSelection},
    platform,
};

fn cursor_position() -> Result<(i32, i32), CaptureError> {
    use mouse_position::mouse_position::Mouse;

    match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => Ok((x, y)),
        Mouse::Error => Err(CaptureError::CursorUnavailable),
    }
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
// Keep this command async: creating a second WebView2 from a synchronous IPC handler can
// block the Windows event loop while WebView2 waits for its environment callback.
pub(crate) async fn open_region_selector(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    if app.get_webview_window("region-selector").is_some() {
        return Err(CaptureError::AlreadyOpen.into());
    }
    platform::ensure_permission()?;
    let (monitor, context) = platform::monitor_at(cursor_position()?)?;
    let main = app
        .get_webview_window("main")
        .ok_or(CaptureError::MainWindowMissing)?;
    let restore_main_window = main.is_visible().unwrap_or(false);
    if restore_main_window {
        main.hide().map_err(|error| error.to_string())?;
    }
    *state
        .region_capture
        .lock()
        .map_err(|error| error.to_string())? = Some(RegionCaptureSession {
        restore_main_window,
        context,
    });

    if let Err(error) = platform::create_selector(&app, &state, monitor) {
        restore_main_after_region(&app);
        return Err(error.into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn complete_region_selection(
    selection: RegionSelection,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    selection.validate()?;
    let selector = app
        .get_webview_window("region-selector")
        .ok_or(CaptureError::SessionEnded)?;
    let session = take_region_session(&state).ok_or(CaptureError::SessionEnded)?;
    let request = platform::prepare_request(&selector, session.context, selection)?;

    let _ = selector.hide();
    let capture = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(180));
        platform::capture(request)
    })
    .await
    .map_err(|error| CaptureError::Task(error.to_string()))?;
    let _ = selector.close();
    if session.restore_main_window {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    match capture {
        Ok(result) => {
            app.emit_to("main", "region-captured", result)
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        Err(error) => {
            let message = error.to_string();
            let _ = app.emit_to("main", "region-capture-error", message);
            Err(error.into())
        }
    }
}

#[tauri::command]
pub(crate) fn cancel_region_selection(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
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
    Ok(emit_result?)
}
