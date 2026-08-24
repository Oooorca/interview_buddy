use tauri::State;

use super::{
    normalize_prompt_settings, SaveSettingsRequest, SecurityResetResult, SecurityState,
    SettingsLoadResult, SettingsSnapshot,
};
use crate::{
    app_state::AppState,
    error::{AppError, AppResult},
    storage,
};

#[tauri::command]
pub(crate) fn load_settings(state: State<'_, AppState>) -> AppResult<SettingsLoadResult> {
    if let Some(message) = state
        .security_error
        .read()
        .map_err(|error| error.to_string())?
        .clone()
    {
        let error = AppError::from_message(message);
        return Ok(SettingsLoadResult::Locked {
            reason: error.code().into(),
            message: error.detail().unwrap_or_default().into(),
        });
    }
    let settings = state.settings.read().map_err(|error| error.to_string())?;
    let security_state = *state
        .settings_security_state
        .read()
        .map_err(|error| error.to_string())?;
    Ok(SettingsLoadResult::Ready {
        snapshot: Box::new(SettingsSnapshot::new(&settings, security_state)),
    })
}

#[tauri::command]
pub(crate) fn save_settings(
    request: SaveSettingsRequest,
    state: State<'_, AppState>,
) -> AppResult<SettingsSnapshot> {
    ensure_security_ready(&state)?;
    let current = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    let mut settings = current.apply_update(request)?;
    normalize_prompt_settings(&mut settings, true)?;
    state
        .settings_store
        .read()
        .map_err(|error| error.to_string())?
        .as_ref()
        .ok_or_else(|| "安全设置存储不可用".to_string())?
        .save(&settings)?;
    *state.settings.write().map_err(|error| error.to_string())? = settings;
    *state
        .settings_security_state
        .write()
        .map_err(|error| error.to_string())? = SecurityState::Ready;
    let settings = state.settings.read().map_err(|error| error.to_string())?;
    Ok(SettingsSnapshot::new(&settings, SecurityState::Ready))
}

pub(crate) fn ensure_security_ready(state: &State<'_, AppState>) -> Result<(), String> {
    if let Some(message) = state
        .security_error
        .read()
        .map_err(|error| error.to_string())?
        .as_ref()
    {
        return Err(format!("安全设置已锁定：{message}"));
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn reset_secure_settings(state: State<'_, AppState>) -> AppResult<SecurityResetResult> {
    let pointer_quarantine = storage::quarantine_pointer(&state.config_dir)?;
    let (store, settings, quarantine) = super::store::SettingsStore::quarantine_and_reset(
        &state.config_dir,
        &state.service,
        state.storage.active_root(),
        state.storage.default_root(),
    )?;
    *state
        .settings_store
        .write()
        .map_err(|error| error.to_string())? = Some(store);
    *state.settings.write().map_err(|error| error.to_string())? = settings.clone();
    *state
        .settings_security_state
        .write()
        .map_err(|error| error.to_string())? = SecurityState::Ready;
    *state
        .security_error
        .write()
        .map_err(|error| error.to_string())? = None;
    Ok(SecurityResetResult {
        snapshot: SettingsSnapshot::new(&settings, SecurityState::Ready),
        quarantine_path: quarantine
            .or(pointer_quarantine)
            .map(|path| storage::path_text(&path)),
    })
}
