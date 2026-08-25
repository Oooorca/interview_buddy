use std::{fs, io::Write, path::Path};
use zeroize::Zeroizing;

use super::{migrate_legacy_settings, normalize_prompt_settings, AppSettings};
use crate::security::{self, decrypt_envelope, encrypt_envelope, EnvelopeKind};

const SETTINGS_FILE: &str = "settings.secure.json";
const SETTINGS_BACKUP: &str = "settings.secure.bak";
const KEY_FILE: &str = "vault-key-v1.dpapi";
const MAX_PLAINTEXT_FILE: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    Ready,
    Migrated,
    Recovered,
}

impl From<LoadStatus> for super::SecurityState {
    fn from(status: LoadStatus) -> Self {
        match status {
            LoadStatus::Ready => super::SecurityState::Ready,
            LoadStatus::Migrated => super::SecurityState::Migrated,
            LoadStatus::Recovered => super::SecurityState::Recovered,
        }
    }
}

pub struct SettingsStore {
    service: String,
    key: Zeroizing<Vec<u8>>,
    settings_path: std::path::PathBuf,
    backup_path: std::path::PathBuf,
}

impl SettingsStore {
    pub fn quarantine_and_reset(
        config_dir: &Path,
        service: &str,
        source_root: &Path,
        reset_root: &Path,
    ) -> Result<(Self, AppSettings, Option<std::path::PathBuf>), String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs();
        let quarantine = source_root.join(format!("recovery-{timestamp}"));
        let mut moved = false;
        for name in [SETTINGS_FILE, SETTINGS_BACKUP, "settings.json"] {
            let source = source_root.join(name);
            if source.is_file() {
                fs::create_dir_all(&quarantine)
                    .map_err(|error| format!("无法创建恢复目录：{error}"))?;
                fs::rename(&source, quarantine.join(name))
                    .map_err(|error| format!("无法隔离旧设置：{error}"))?;
                moved = true;
            }
        }
        let key_path = config_dir.join(KEY_FILE);
        security::reset_key(&key_path, service)?;
        let (store, settings, _) = Self::bootstrap(config_dir, service, reset_root, &[])?;
        Ok((store, settings, moved.then_some(quarantine)))
    }

    pub fn bootstrap(
        config_dir: &Path,
        service: &str,
        data_root: &Path,
        plaintext_candidates: &[std::path::PathBuf],
    ) -> Result<(Self, AppSettings, LoadStatus), String> {
        let key_path = config_dir.join(KEY_FILE);
        let settings_path = data_root.join(SETTINGS_FILE);
        let backup_path = data_root.join(SETTINGS_BACKUP);
        let encrypted_exists = settings_path.is_file() || backup_path.is_file();
        let key = security::load_key(&key_path, service, !encrypted_exists)?;
        Self::bootstrap_with_key(service, data_root, plaintext_candidates, key)
    }

    pub fn bootstrap_with_key(
        service: &str,
        data_root: &Path,
        plaintext_candidates: &[std::path::PathBuf],
        key: Zeroizing<Vec<u8>>,
    ) -> Result<(Self, AppSettings, LoadStatus), String> {
        let settings_path = data_root.join(SETTINGS_FILE);
        let backup_path = data_root.join(SETTINGS_BACKUP);
        let store = Self {
            service: service.into(),
            key,
            settings_path,
            backup_path,
        };

        match store.load_encrypted(&store.settings_path) {
            Ok(settings) => {
                let removed_plaintext = remove_plaintext_candidates(plaintext_candidates)?;
                let status = if removed_plaintext {
                    LoadStatus::Migrated
                } else {
                    LoadStatus::Ready
                };
                return Ok((store, settings, status));
            }
            Err(main_error) if store.settings_path.is_file() => {
                if let Ok(settings) = store.load_encrypted(&store.backup_path) {
                    store.quarantine_file(&store.settings_path, "corrupt")?;
                    store.save(&settings)?;
                    remove_plaintext_candidates(plaintext_candidates)?;
                    return Ok((store, settings, LoadStatus::Recovered));
                }
                return Err(main_error);
            }
            Err(_) => {}
        }

        if store.backup_path.is_file() {
            let settings = store.load_encrypted(&store.backup_path)?;
            store.save(&settings)?;
            remove_plaintext_candidates(plaintext_candidates)?;
            return Ok((store, settings, LoadStatus::Recovered));
        }

        for candidate in plaintext_candidates {
            if !candidate.is_file() {
                continue;
            }
            if fs::metadata(candidate)
                .map_err(|error| format!("无法检查旧设置大小：{error}"))?
                .len()
                > MAX_PLAINTEXT_FILE
            {
                return Err("旧明文设置超过 1 MiB 安全上限".into());
            }
            let source = Zeroizing::new(
                fs::read_to_string(candidate)
                    .map_err(|error| format!("无法读取旧设置：{error}"))?,
            );
            let mut settings: AppSettings = serde_json::from_str(&source)
                .map_err(|error| format!("旧设置格式无效：{error}"))?;
            migrate_legacy_settings(&source, &mut settings);
            normalize_prompt_settings(&mut settings, false)?;
            store.save(&settings)?;
            let verified = store.load_encrypted(&store.settings_path)?;
            let verified_json =
                Zeroizing::new(serde_json::to_vec(&verified).map_err(|error| error.to_string())?);
            let expected_json =
                Zeroizing::new(serde_json::to_vec(&settings).map_err(|error| error.to_string())?);
            if verified_json.as_slice() != expected_json.as_slice() {
                return Err("加密设置迁移校验失败".into());
            }
            remove_plaintext_candidates(plaintext_candidates)?;
            return Ok((store, settings, LoadStatus::Migrated));
        }

        let settings = AppSettings::default();
        store.save(&settings)?;
        Ok((store, settings, LoadStatus::Ready))
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), String> {
        let plaintext =
            Zeroizing::new(serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?);
        let encoded =
            encrypt_envelope(&self.key, &self.service, EnvelopeKind::Settings, &plaintext)?;
        let verified =
            decrypt_envelope(&self.key, &self.service, EnvelopeKind::Settings, &encoded)?;
        serde_json::from_slice::<AppSettings>(&verified)
            .map_err(|error| format!("加密设置回读校验失败：{error}"))?;
        atomic_replace(&self.settings_path, &self.backup_path, &encoded)
    }

    fn load_encrypted(&self, path: &Path) -> Result<AppSettings, String> {
        let encoded = fs::read(path).map_err(|error| format!("无法读取加密设置：{error}"))?;
        let plaintext =
            decrypt_envelope(&self.key, &self.service, EnvelopeKind::Settings, &encoded)?;
        serde_json::from_slice(&plaintext).map_err(|error| format!("解密设置格式无效：{error}"))
    }

    fn quarantine_file(&self, path: &Path, label: &str) -> Result<(), String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs();
        let quarantine = path.with_extension(format!("{label}-{timestamp}"));
        fs::rename(path, quarantine).map_err(|error| format!("无法隔离损坏设置：{error}"))
    }
}

fn remove_plaintext_candidates(candidates: &[std::path::PathBuf]) -> Result<bool, String> {
    let mut removed = false;
    for candidate in candidates {
        if candidate.is_file() {
            fs::remove_file(candidate)
                .map_err(|error| format!("无法删除已迁移的旧明文设置：{error}"))?;
            removed = true;
        }
    }
    Ok(removed)
}

#[cfg(target_os = "windows")]
pub fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "无法确定安全文件目录".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("无法创建安全文件目录：{error}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("无法创建安全临时文件：{error}"))?;
    temporary
        .write_all(bytes)
        .map_err(|error| format!("无法写入安全临时文件：{error}"))?;
    temporary
        .as_file_mut()
        .sync_all()
        .map_err(|error| format!("无法同步安全临时文件：{error}"))?;
    temporary
        .persist_noclobber(path)
        .map_err(|error| format!("无法保存安全文件：{}", error.error))?;
    Ok(())
}

pub fn atomic_replace(path: &Path, backup: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "无法确定设置文件目录".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("无法创建设置目录：{error}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("无法创建设置临时文件：{error}"))?;
    temporary
        .write_all(bytes)
        .map_err(|error| format!("无法写入设置临时文件：{error}"))?;
    temporary
        .as_file_mut()
        .sync_all()
        .map_err(|error| format!("无法同步设置临时文件：{error}"))?;

    let mut rotated = false;
    if path.is_file() {
        if backup.is_file() {
            fs::remove_file(backup).map_err(|error| format!("无法轮换设置备份：{error}"))?;
        }
        fs::rename(path, backup).map_err(|error| format!("无法创建设置备份：{error}"))?;
        rotated = true;
    }
    if let Err(error) = temporary.persist_noclobber(path) {
        if rotated && backup.is_file() && !path.exists() {
            let _ = fs::rename(backup, path);
        }
        return Err(format!("无法替换设置文件：{}", error.error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::SecretString;

    fn configured_settings(key: &str) -> AppSettings {
        AppSettings {
            api_key: SecretString(key.into()),
            fixed_context: "private resume context".into(),
            system_prompt: Some("private custom prompt".into()),
            system_prompt_mode: crate::settings::PromptMode::Custom,
            ..AppSettings::default()
        }
    }

    #[test]
    fn plaintext_migration_encrypts_and_removes_source_after_verification() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("settings.json");
        let settings = configured_settings("synthetic-api-secret");
        fs::write(&source, serde_json::to_vec_pretty(&settings).unwrap()).unwrap();

        let (_store, loaded, status) = SettingsStore::bootstrap_with_key(
            "test.app",
            root.path(),
            std::slice::from_ref(&source),
            Zeroizing::new(vec![5u8; 32]),
        )
        .unwrap();

        assert_eq!(status, LoadStatus::Migrated);
        assert_eq!(loaded.api_key.expose(), "synthetic-api-secret");
        assert!(!source.exists());
        let encrypted = fs::read_to_string(root.path().join(SETTINGS_FILE)).unwrap();
        assert!(!encrypted.contains("synthetic-api-secret"));
        assert!(!encrypted.contains("private resume context"));
        assert!(!encrypted.contains("private custom prompt"));
    }

    #[test]
    fn valid_backup_recovers_a_tampered_main_file() {
        let root = tempfile::tempdir().unwrap();
        let key = Zeroizing::new(vec![6u8; 32]);
        let (store, _, _) =
            SettingsStore::bootstrap_with_key("test.app", root.path(), &[], key.clone()).unwrap();
        let first = configured_settings("backup-key");
        store.save(&first).unwrap();
        let second = configured_settings("main-key");
        store.save(&second).unwrap();
        fs::write(root.path().join(SETTINGS_FILE), b"tampered").unwrap();

        let (_store, recovered, status) =
            SettingsStore::bootstrap_with_key("test.app", root.path(), &[], key).unwrap();
        assert_eq!(status, LoadStatus::Recovered);
        assert_eq!(recovered.api_key.expose(), "backup-key");
        assert!(root.path().join(SETTINGS_FILE).is_file());
    }

    #[test]
    fn valid_main_file_has_priority_over_backup() {
        let root = tempfile::tempdir().unwrap();
        let key = Zeroizing::new(vec![7u8; 32]);
        let (store, _, _) =
            SettingsStore::bootstrap_with_key("test.app", root.path(), &[], key.clone()).unwrap();
        store.save(&configured_settings("older-key")).unwrap();
        store.save(&configured_settings("current-key")).unwrap();

        let (_store, loaded, status) =
            SettingsStore::bootstrap_with_key("test.app", root.path(), &[], key).unwrap();
        assert_eq!(status, LoadStatus::Ready);
        assert_eq!(loaded.api_key.expose(), "current-key");
    }

    #[test]
    fn valid_encrypted_settings_remove_stale_plaintext_candidates() {
        let root = tempfile::tempdir().unwrap();
        let legacy = tempfile::tempdir().unwrap();
        let key = Zeroizing::new(vec![9u8; 32]);
        let (store, _, _) =
            SettingsStore::bootstrap_with_key("test.app", root.path(), &[], key.clone()).unwrap();
        store.save(&configured_settings("encrypted-key")).unwrap();
        let stale = legacy.path().join("settings.json");
        fs::write(
            &stale,
            serde_json::to_vec(&configured_settings("stale-key")).unwrap(),
        )
        .unwrap();

        let (_store, loaded, status) = SettingsStore::bootstrap_with_key(
            "test.app",
            root.path(),
            std::slice::from_ref(&stale),
            key,
        )
        .unwrap();

        assert_eq!(status, LoadStatus::Migrated);
        assert_eq!(loaded.api_key.expose(), "encrypted-key");
        assert!(!stale.exists());
    }

    #[test]
    fn encrypted_failure_never_falls_back_to_plaintext_or_overwrites_files() {
        let root = tempfile::tempdir().unwrap();
        let (store, _, _) = SettingsStore::bootstrap_with_key(
            "test.app",
            root.path(),
            &[],
            Zeroizing::new(vec![10u8; 32]),
        )
        .unwrap();
        store.save(&configured_settings("encrypted-key")).unwrap();
        let encrypted_before = fs::read(root.path().join(SETTINGS_FILE)).unwrap();
        let plaintext = root.path().join("settings.json");
        fs::write(
            &plaintext,
            serde_json::to_vec_pretty(&configured_settings("plaintext-key")).unwrap(),
        )
        .unwrap();

        let result = SettingsStore::bootstrap_with_key(
            "test.app",
            root.path(),
            std::slice::from_ref(&plaintext),
            Zeroizing::new(vec![11u8; 32]),
        );
        assert!(result.is_err());
        assert_eq!(
            fs::read(root.path().join(SETTINGS_FILE)).unwrap(),
            encrypted_before
        );
        assert!(plaintext.exists());
    }
}
