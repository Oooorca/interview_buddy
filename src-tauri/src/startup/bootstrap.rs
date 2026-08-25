use std::{
    collections::HashSet,
    sync::{Mutex, RwLock},
};

use tauri::Manager;

use crate::{
    app_state::AppState,
    security,
    settings::{self, AppSettings, SecurityState},
    storage,
    transcription::is_dashscope,
};

pub(super) fn initialize_state(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = app.path().app_config_dir()?;
    let service = app.config().identifier.clone();
    let old_settings_path = config_dir.join("settings.json");
    let old_webview_path = app.path().app_local_data_dir()?.join("EBWebView");
    let (default_data_dir, legacy_default_data_dir) = if service.ends_with(".dev") {
        (".interview-buddy-dev", "cache-dev")
    } else {
        (".interview-buddy", "cache")
    };
    let app_scoped_data_dir = old_webview_path
        .parent()
        .ok_or_else(|| "无法确定应用数据目录".to_string())?;
    let default_data_root = app_scoped_data_dir
        .parent()
        .ok_or_else(|| "无法确定系统应用数据目录".to_string())?
        .join(default_data_dir);
    let legacy_default_data_root = app_scoped_data_dir.join(legacy_default_data_dir);
    let portable_data_root = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("cache")));
    let key_path = config_dir.join("vault-key-v1.dpapi");
    let encrypted_exists = config_dir.join("storage-location.secure.json").is_file()
        || config_dir.join("storage-location.secure.bak").is_file()
        || default_data_root.join("settings.secure.json").is_file()
        || default_data_root.join("settings.secure.bak").is_file()
        || legacy_default_data_root
            .join("settings.secure.json")
            .is_file()
        || legacy_default_data_root
            .join("settings.secure.bak")
            .is_file()
        || portable_data_root.as_ref().is_some_and(|root| {
            root.join("settings.secure.json").is_file()
                || root.join("settings.secure.bak").is_file()
        });
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
            legacy_default_data_root.join("settings.json"),
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
    let (settings_store, mut settings, settings_security_state, security_error) = match bootstrap {
        Ok((store, settings, status)) => (Some(store), settings, status.into(), None),
        Err(error) => (
            None,
            AppSettings::default(),
            SecurityState::Ready,
            Some(error),
        ),
    };
    migrate_runtime_settings(&mut settings, settings_store.as_ref())?;
    if let Err(error) = storage.run_startup_cleanup(settings.auto_safe_cleanup) {
        eprintln!("Interview Buddy safe cleanup deferred after error: {error}");
    }
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
    Ok(())
}

fn migrate_runtime_settings(
    settings: &mut AppSettings,
    store: Option<&settings::store::SettingsStore>,
) -> Result<(), String> {
    let mut changed = false;
    if settings.transcription_language != "auto"
        && settings.my_transcription_language == "auto"
        && settings.their_transcription_language == "auto"
    {
        settings.my_transcription_language = settings.transcription_language.clone();
        settings.their_transcription_language = settings.transcription_language.clone();
        settings.transcription_language = "auto".into();
        changed = true;
    }
    if is_dashscope(&settings.base_url)
        && (!settings.transcription_model.starts_with("qwen3-asr-flash")
            || settings.transcription_model.contains("realtime")
            || settings.transcription_model.contains("filetrans"))
    {
        settings.transcription_model = "qwen3-asr-flash".into();
        changed = true;
    }
    if changed {
        if let Some(store) = store {
            store.save(settings)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_single_language_is_moved_to_both_speakers() {
        let mut settings = AppSettings {
            transcription_language: "en-US".into(),
            ..AppSettings::default()
        };

        migrate_runtime_settings(&mut settings, None).expect("migrate runtime settings");

        assert_eq!(settings.transcription_language, "auto");
        assert_eq!(settings.my_transcription_language, "en-US");
        assert_eq!(settings.their_transcription_language, "en-US");
    }

    #[test]
    fn explicit_speaker_languages_are_not_overwritten() {
        let mut settings = AppSettings {
            transcription_language: "zh-CN".into(),
            my_transcription_language: "en-US".into(),
            their_transcription_language: "auto".into(),
            ..AppSettings::default()
        };

        migrate_runtime_settings(&mut settings, None).expect("migrate runtime settings");

        assert_eq!(settings.transcription_language, "zh-CN");
        assert_eq!(settings.my_transcription_language, "en-US");
        assert_eq!(settings.their_transcription_language, "auto");
    }

    #[test]
    fn dashscope_legacy_models_are_normalized() {
        let mut settings = AppSettings {
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
            transcription_model: "qwen3-asr-flash-realtime".into(),
            ..AppSettings::default()
        };

        migrate_runtime_settings(&mut settings, None).expect("migrate runtime settings");

        assert_eq!(settings.transcription_model, "qwen3-asr-flash");
    }
}
