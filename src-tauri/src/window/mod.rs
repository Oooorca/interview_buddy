mod commands;
mod lifecycle;
mod platform;
mod sizing;

pub(crate) use commands::{apply_window_size, remember_window_size, window_size_info};
pub(crate) use lifecycle::{quit_app, toggle_main_window};
pub(crate) use sizing::apply_saved_window_size;

use crate::{error::AppResult, storage::StorageManager};

pub(crate) fn build_main_window(
    app: &tauri::AppHandle,
    config: &tauri::utils::config::WindowConfig,
    storage: &StorageManager,
) -> AppResult<tauri::WebviewWindow> {
    platform::build_main_window(app, config, storage)
}

pub(crate) fn initialize_overlay(window: &tauri::WebviewWindow) -> AppResult<()> {
    window
        .set_content_protected(true)
        .map_err(|error| error.to_string())?;
    window
        .set_always_on_top(true)
        .map_err(|error| error.to_string())?;
    configure_platform_overlay(window)?;
    Ok(())
}

pub(crate) fn configure_platform_overlay(window: &tauri::WebviewWindow) -> AppResult<()> {
    platform::configure_overlay(window)?;
    platform::verify_capture_protection(window);
    Ok(())
}
