use std::{
    collections::HashSet,
    sync::{Mutex, RwLock},
};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

mod app_state;
mod audio;
mod capture;
mod llm;
mod security;
mod settings;
mod shortcuts;
mod storage;
mod transcription;
mod window;

use app_state::AppState;
use capture::region::{
    cancel_region_selection, complete_region_selection, open_region_selector,
    restore_main_after_region,
};
use llm::client::{ask_llm, cancel_llm};
#[cfg(test)]
use llm::{
    client::{bounded_history, CancellationGuard, ConversationMessage},
    stream::parse_sse_delta,
};
use settings::{
    commands::{ensure_security_ready, load_settings, reset_secure_settings, save_settings},
    AppSettings, SecurityState,
};
use shortcuts::{handle_shortcut_action, shortcut_warnings};
use transcription::{
    discard_system_audio_chunk, is_dashscope, list_system_audio_devices, start_system_audio,
    stop_system_audio_and_transcribe, system_audio_level, transcribe_audio,
    transcribe_system_audio_chunk,
};
#[cfg(target_os = "windows")]
use window::query_display_affinity;
use window::{quit_app, toggle_main_window};

#[tauri::command]
fn storage_info(state: State<'_, AppState>) -> Result<storage::StorageInfo, String> {
    state.storage.info()
}

#[tauri::command]
fn set_storage_root(
    path: String,
    state: State<'_, AppState>,
) -> Result<storage::StorageInfo, String> {
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
    state
        .storage
        .configure_root(std::path::Path::new(path.trim()))
}

#[tauri::command]
fn schedule_safe_cleanup(state: State<'_, AppState>) -> Result<storage::StorageInfo, String> {
    state.storage.schedule_cleanup()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut_plugin = tauri_plugin_global_shortcut::Builder::new().build();
    tauri::Builder::default()
        .plugin(shortcut_plugin)
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if window.label() == "region-selector" && matches!(event, tauri::WindowEvent::Destroyed)
            {
                restore_main_after_region(window.app_handle());
            }
        })
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let service = app.config().identifier.clone();
            let old_settings_path = config_dir.join("settings.json");
            let old_webview_path = app.path().app_local_data_dir()?.join("EBWebView");
            let default_data_dir = if service.ends_with(".dev") {
                "cache-dev"
            } else {
                "cache"
            };
            let default_data_root = old_webview_path
                .parent()
                .ok_or_else(|| "无法确定默认数据目录".to_string())?
                .join(default_data_dir);
            let key_path = config_dir.join("vault-key-v1.dpapi");
            let encrypted_exists = config_dir.join("storage-location.secure.json").is_file()
                || default_data_root.join("settings.secure.json").is_file()
                || default_data_root.join("settings.secure.bak").is_file();
            let legacy_settings_path = app
                .path()
                .config_dir()?
                .join("local.desktop-copilot")
                .join("settings.json");
            let security_bootstrap = (|| -> Result<_, String> {
                let vault_key = security::load_key(&key_path, &service, !encrypted_exists)?;
                let storage = storage::StorageManager::initialize(
                    &old_settings_path,
                    &old_webview_path,
                    vault_key.clone(),
                    &service,
                )?;
                let plaintext_candidates = vec![
                    storage.active_root().join("settings.json"),
                    old_settings_path.clone(),
                    legacy_settings_path,
                ];
                let bootstrap = settings::store::SettingsStore::bootstrap_with_key(
                    &service,
                    storage.active_root(),
                    &plaintext_candidates,
                    vault_key,
                );
                Ok((storage, bootstrap))
            })();
            let (storage, bootstrap) = match security_bootstrap {
                Ok(result) => result,
                Err(error) => (
                    storage::StorageManager::initialize_locked(
                        &old_settings_path,
                        &old_webview_path,
                        security::random_key(),
                        &service,
                    )?,
                    Err(error),
                ),
            };
            let (settings_store, mut settings, settings_security_state, security_error) =
                match bootstrap {
                    Ok((store, settings, status)) => (Some(store), settings, status.into(), None),
                    Err(error) => (
                        None,
                        AppSettings::default(),
                        SecurityState::Ready,
                        Some(error),
                    ),
                };
            let mut settings_changed = false;
            if settings.transcription_language != "auto"
                && settings.my_transcription_language == "auto"
                && settings.their_transcription_language == "auto"
            {
                settings.my_transcription_language = settings.transcription_language.clone();
                settings.their_transcription_language = settings.transcription_language.clone();
                settings.transcription_language = "auto".into();
                settings_changed = true;
            }
            if is_dashscope(&settings.base_url)
                && (!settings.transcription_model.starts_with("qwen3-asr-flash")
                    || settings.transcription_model.contains("realtime")
                    || settings.transcription_model.contains("filetrans"))
            {
                settings.transcription_model = "qwen3-asr-flash".into();
                settings_changed = true;
            }
            if settings_changed {
                if let Some(store) = &settings_store {
                    store.save(&settings)?;
                }
            }
            if let Err(error) = storage.run_startup_cleanup(settings.auto_safe_cleanup) {
                eprintln!("Interview Buddy safe cleanup deferred after error: {error}");
            }
            let webview_data_path = storage.active_webview_path();
            app.manage(AppState {
                settings: RwLock::new(settings),
                settings_store: RwLock::new(settings_store),
                settings_security_state: RwLock::new(settings_security_state),
                security_error: RwLock::new(security_error),
                config_dir,
                service,
                storage,
                shortcut_warnings: RwLock::new(Vec::new()),
                system_audio: Mutex::new(None),
                region_capture: Mutex::new(None),
                cancelled_requests: Mutex::new(HashSet::new()),
            });
            let main_config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or_else(|| "缺少 main 窗口配置".to_string())?;
            let main_builder =
                tauri::WebviewWindowBuilder::from_config(app.handle(), &main_config)?;
            #[cfg(target_os = "windows")]
            let main_builder = main_builder.data_directory(webview_data_path);
            let window = main_builder.build()?;
            window.set_content_protected(true)?;
            window.set_always_on_top(true)?;
            window.show()?;
            #[cfg(target_os = "windows")]
            match query_display_affinity(&window) {
                Ok(affinity) => eprintln!("Interview Buddy display affinity: 0x{affinity:X}"),
                Err(error) => {
                    eprintln!("Interview Buddy display affinity check failed: {error}")
                }
            }
            if let Some(icon) = app.default_window_icon().cloned() {
                TrayIconBuilder::with_id("interview-buddy-tray")
                    .icon(icon)
                    .tooltip("Interview Buddy — 点击显示或隐藏")
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
            let shortcuts = [
                ("CommandOrControl+Shift+C", "clear"),
                ("CommandOrControl+Shift+S", "capture-region"),
                ("CommandOrControl+Shift+L", "listening-toggle"),
                ("CommandOrControl+Shift+A", "answer-toggle"),
                ("CommandOrControl+Shift+I", "send"),
                ("CommandOrControl+Shift+Space", "toggle-window"),
                ("CommandOrControl+Q", "quit"),
            ];
            for (shortcut, action) in shortcuts {
                if let Err(error) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, move |app, _shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                handle_shortcut_action(app, action);
                            }
                        })
                {
                    let warning = format!("{shortcut}：{error}");
                    eprintln!("Interview Buddy shortcut unavailable: {warning}");
                    app.state::<AppState>()
                        .shortcut_warnings
                        .write()
                        .map_err(|error| error.to_string())?
                        .push(warning);
                }
            }
            Ok(())
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{
        migrate_legacy_settings as migrate_legacy_prompt_settings,
        migration::resolved_system_prompt, normalize_prompt_settings, PromptMode,
    };

    #[test]
    fn openai_compatible_sse_delta_is_extracted() {
        let delta =
            parse_sse_delta(r#"{"choices":[{"delta":{"content":"你好"},"finish_reason":null}]}"#)
                .expect("valid chunk");
        assert_eq!(delta.as_deref(), Some("你好"));
        assert_eq!(
            parse_sse_delta(r#"{"choices":[{"delta":{}}]}"#).unwrap(),
            None
        );
    }

    #[test]
    fn history_keeps_only_supported_roles_in_order() {
        let history = vec![
            ConversationMessage {
                role: "system".into(),
                content: "ignore".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "question".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "answer".into(),
            },
        ];
        let bounded = bounded_history(&history);
        assert_eq!(bounded.len(), 2);
        assert_eq!(bounded[0]["role"], "user");
        assert_eq!(bounded[1]["role"], "assistant");
    }

    #[test]
    fn legacy_prompts_migrate_empty_and_known_defaults_to_default_mode() {
        let mut empty = AppSettings {
            system_prompt: Some(String::new()),
            ..AppSettings::default()
        };
        assert!(migrate_legacy_prompt_settings("{}", &mut empty));
        assert_eq!(empty.system_prompt_mode, PromptMode::Default);

        let mut builtin = AppSettings {
            system_prompt: Some(settings::system_prompt().into()),
            ..AppSettings::default()
        };
        migrate_legacy_prompt_settings("{}", &mut builtin);
        assert_eq!(builtin.system_prompt_mode, PromptMode::Default);

        let mut custom = AppSettings {
            system_prompt: Some("我的专用回答规则".into()),
            ..AppSettings::default()
        };
        migrate_legacy_prompt_settings("{}", &mut custom);
        assert_eq!(custom.system_prompt_mode, PromptMode::Custom);
    }

    #[test]
    fn prompt_modes_resolve_and_normalize_safely() {
        let mut settings = AppSettings::default();
        assert_eq!(
            resolved_system_prompt(&settings),
            Some(settings::system_prompt())
        );

        settings.system_prompt_mode = PromptMode::Disabled;
        settings.system_prompt = Some("ignored".into());
        assert!(normalize_prompt_settings(&mut settings, true).expect("normalize"));
        assert_eq!(settings.system_prompt, None);
        assert_eq!(resolved_system_prompt(&settings), None);

        settings.system_prompt_mode = PromptMode::Custom;
        assert!(normalize_prompt_settings(&mut settings, true).is_err());
        settings.system_prompt = Some("custom".into());
        assert!(!normalize_prompt_settings(&mut settings, true).expect("custom"));
        assert_eq!(resolved_system_prompt(&settings), Some("custom"));
    }

    #[test]
    fn default_prompt_settings_serialize_without_copying_builtin_text() {
        let value = serde_json::to_value(AppSettings::default()).expect("serialize settings");
        assert_eq!(value["systemPromptMode"], "default");
        assert_eq!(value["codingPromptMode"], "default");
        assert!(value["systemPrompt"].is_null());
        assert!(value["codingPrompt"].is_null());
    }

    #[test]
    fn cancellation_guard_clears_stale_and_completed_requests() {
        let requests = Mutex::new(HashSet::from(["request".to_string()]));
        {
            let _guard = CancellationGuard::new(&requests, "request".into()).expect("create guard");
            assert!(!requests.lock().expect("requests").contains("request"));
            requests.lock().expect("requests").insert("request".into());
        }
        assert!(!requests.lock().expect("requests").contains("request"));
    }
}
