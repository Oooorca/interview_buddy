use tauri::State;

use crate::{app_state::AppState, error::AppResult, settings::commands::ensure_security_ready};

use super::StorageInfo;

#[tauri::command(async)]
pub(crate) fn storage_info(state: State<'_, AppState>) -> AppResult<StorageInfo> {
    Ok(state.storage.info()?)
}

#[tauri::command(async)]
pub(crate) fn set_storage_root(path: String, state: State<'_, AppState>) -> AppResult<StorageInfo> {
    ensure_security_ready(&state)?;
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    state
        .settings_store
        .read()
        .map_err(|error| error.to_string())?
        .as_ref()
        .ok_or_else(|| "安全设置存储不可用".to_string())?
        .save(&settings)?;
    Ok(state
        .storage
        .configure_root(std::path::Path::new(path.trim()))?)
}

#[tauri::command(async)]
pub(crate) fn schedule_safe_cleanup(state: State<'_, AppState>) -> AppResult<StorageInfo> {
    Ok(state.storage.schedule_cleanup()?)
}
