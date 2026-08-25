use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::{app_state::AppState, shortcuts::handle_shortcut_action};

const SHORTCUTS: &[(&str, &str)] = &[
    ("CommandOrControl+Shift+C", "clear"),
    ("CommandOrControl+Shift+S", "capture-region"),
    ("CommandOrControl+Shift+L", "listening-toggle"),
    ("CommandOrControl+Shift+A", "answer-toggle"),
    ("CommandOrControl+Shift+I", "send"),
    ("CommandOrControl+Shift+Space", "toggle-window"),
    ("CommandOrControl+Q", "quit"),
];

pub(super) fn register(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    for (shortcut, action) in SHORTCUTS {
        let action = *action;
        if let Err(error) =
            app.global_shortcut()
                .on_shortcut(*shortcut, move |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        handle_shortcut_action(app, action);
                    }
                })
        {
            let warning = format!("{shortcut}: {error}");
            eprintln!("Interview Buddy shortcut unavailable: {warning}");
            app.state::<AppState>()
                .shortcut_warnings
                .write()
                .map_err(|error| error.to_string())?
                .push(warning);
        }
    }
    Ok(())
}
