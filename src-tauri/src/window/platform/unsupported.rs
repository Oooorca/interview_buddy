use crate::{error::AppResult, storage::StorageManager};

pub(crate) fn build_main_window(
    app: &tauri::AppHandle,
    config: &tauri::utils::config::WindowConfig,
    _storage: &StorageManager,
) -> AppResult<tauri::WebviewWindow> {
    Ok(tauri::WebviewWindowBuilder::from_config(app, config)
        .map_err(|error| error.to_string())?
        .build()
        .map_err(|error| error.to_string())?)
}

pub(crate) fn configure_overlay(_window: &tauri::WebviewWindow) -> AppResult<()> {
    Ok(())
}

pub(crate) fn verify_capture_protection(_window: &tauri::WebviewWindow) {}
