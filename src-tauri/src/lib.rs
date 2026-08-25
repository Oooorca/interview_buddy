mod app_state;
mod audio;
mod capture;
mod error;
mod llm;
mod security;
mod settings;
mod shortcuts;
mod startup;
mod storage;
mod transcription;
mod window;

use capture::{cancel_region_selection, complete_region_selection, open_region_selector};
use llm::client::{ask_llm, cancel_llm};
use settings::commands::{load_settings, reset_secure_settings, save_settings};
use shortcuts::shortcut_warnings;
use storage::{schedule_safe_cleanup, set_storage_root, storage_info};
use transcription::{
    discard_system_audio_chunk, list_system_audio_devices, start_system_audio,
    stop_system_audio_and_transcribe, system_audio_level, transcribe_audio,
    transcribe_system_audio_chunk,
};
use window::{apply_window_size, quit_app, remember_window_size, window_size_info};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut_plugin = tauri_plugin_global_shortcut::Builder::new().build();
    tauri::Builder::default()
        .plugin(shortcut_plugin)
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(startup::handle_window_event)
        .setup(startup::setup)
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            reset_secure_settings,
            shortcut_warnings,
            storage_info,
            set_storage_root,
            schedule_safe_cleanup,
            open_region_selector,
            complete_region_selection,
            cancel_region_selection,
            quit_app,
            window_size_info,
            apply_window_size,
            remember_window_size,
            ask_llm,
            cancel_llm,
            transcribe_audio,
            list_system_audio_devices,
            start_system_audio,
            system_audio_level,
            discard_system_audio_chunk,
            stop_system_audio_and_transcribe,
            transcribe_system_audio_chunk
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Interview Buddy");
}
