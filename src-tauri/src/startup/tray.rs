use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::window::toggle_main_window;

pub(super) fn register(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(icon) = app.default_window_icon().cloned() {
        TrayIconBuilder::with_id("interview-buddy-tray")
            .icon(icon)
            .tooltip("Interview Buddy")
            .show_menu_on_left_click(false)
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    toggle_main_window(tray.app_handle());
                }
            })
            .build(app)?;
    }
    Ok(())
}
