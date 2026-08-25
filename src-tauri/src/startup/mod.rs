mod bootstrap;
mod shortcuts;
mod tray;
mod window;

use tauri::Manager;

pub(crate) fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    bootstrap::initialize_state(app)?;
    window::create_main_window(app)?;
    tray::register(app)?;
    shortcuts::register(app)?;
    Ok(())
}

pub(crate) fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() == "region-selector" && matches!(event, tauri::WindowEvent::Destroyed) {
        crate::capture::restore_main_after_region(window.app_handle());
    }
}
