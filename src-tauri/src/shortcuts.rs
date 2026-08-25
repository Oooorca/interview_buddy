use tauri::{Emitter, State};

use crate::{app_state::AppState, error::AppResult, window::toggle_main_window};

#[tauri::command]
pub(crate) fn shortcut_warnings(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    Ok(state
        .shortcut_warnings
        .read()
        .map(|warnings| warnings.clone())
        .map_err(|error| error.to_string())?)
}

pub(crate) fn handle_shortcut_action(app: &tauri::AppHandle, action: &'static str) {
    match action {
        "toggle-window" => toggle_main_window(app),
        "quit" => app.exit(0),
        _ => {
            let _ = app.emit("shortcut-action", action);
        }
    }
}
