use tauri::Manager;

use crate::{app_state::AppState, window};

pub(super) fn create_main_window(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let main_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .cloned()
        .ok_or_else(|| "缺少 main 窗口配置".to_string())?;
    let state = app.state::<AppState>();
    let main = window::build_main_window(app.handle(), &main_config, &state.storage)?;
    {
        let settings = state.settings.read().map_err(|error| error.to_string())?;
        window::apply_saved_window_size(&main, &settings)?;
    }
    window::initialize_overlay(&main)?;
    main.show()?;
    Ok(())
}
