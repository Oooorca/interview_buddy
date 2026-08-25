use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        RwLock,
    },
};
use zeroize::Zeroizing;

mod cleanup;
mod commands;
mod migration;
mod platform;

pub(crate) use commands::{schedule_safe_cleanup, set_storage_root, storage_info};
pub(crate) use platform::path_text;

use cleanup::{directory_size, safe_cache_size, safe_cleanup};
use migration::{
    copy_managed_root, migrate_legacy_file, migrate_legacy_webview, migrate_managed_root,
};
use platform::same_path;

use crate::{
    security::{decrypt_envelope, encrypt_envelope, EnvelopeKind},
    settings::store::atomic_replace,
};

const POINTER_FILE: &str = "storage-location.secure.json";
const POINTER_BACKUP_FILE: &str = "storage-location.secure.bak";
const LEGACY_POINTER_FILE: &str = "storage-location.json";
const DEFAULT_DATA_DIR: &str = ".interview-buddy";
const DEFAULT_DEV_DATA_DIR: &str = ".interview-buddy-dev";
const LEGACY_PORTABLE_DATA_DIR: &str = "cache";
const SETTINGS_FILE: &str = "settings.secure.json";
const SETTINGS_BACKUP_FILE: &str = "settings.secure.bak";
const LEGACY_SETTINGS_FILE: &str = "settings.json";
const WEBVIEW_DIR: &str = "webview2";
const STORAGE_MARKER: &str = ".interview-buddy-storage";
const CLEANUP_MARKER: &str = ".cleanup-pending";
const MAX_PLAINTEXT_POINTER: u64 = 64 * 1024;

fn default_data_dir(service: &str) -> &'static str {
    if service.ends_with(".dev") {
        DEFAULT_DEV_DATA_DIR
    } else {
        DEFAULT_DATA_DIR
    }
}

fn legacy_default_data_dir(service: &str) -> &'static str {
    if service.ends_with(".dev") {
        "cache-dev"
    } else {
        LEGACY_PORTABLE_DATA_DIR
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoragePointer {
    data_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    migrate_from: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub data_root: String,
    pub default_data_root: String,
    pub webview_data_root: String,
    pub total_bytes: u64,
    pub safe_cache_bytes: u64,
    pub cleanup_pending: bool,
    pub restart_required: bool,
    pub is_default: bool,
}

pub struct StorageManager {
    active_root: PathBuf,
    configured_root: RwLock<PathBuf>,
    default_root: PathBuf,
    pointer_path: PathBuf,
    key: Zeroizing<Vec<u8>>,
    service: String,
    restart_required: AtomicBool,
}

impl StorageManager {
    pub fn initialize(
        legacy_settings_path: &Path,
        legacy_webview_path: &Path,
        key: Zeroizing<Vec<u8>>,
        service: &str,
    ) -> Result<Self, String> {
        let bootstrap_dir = legacy_settings_path
            .parent()
            .ok_or_else(|| "无法确定应用配置目录".to_string())?;
        let app_scoped_data_dir = legacy_webview_path
            .parent()
            .ok_or_else(|| "无法确定应用数据目录".to_string())?;
        let local_data_dir = app_scoped_data_dir
            .parent()
            .ok_or_else(|| "无法确定系统应用数据目录".to_string())?;
        let executable =
            std::env::current_exe().map_err(|error| format!("无法确定程序路径：{error}"))?;
        let executable_dir = executable
            .parent()
            .ok_or_else(|| "无法确定程序所在目录".to_string())?
            .to_path_buf();
        let pointer_path = bootstrap_dir.join(POINTER_FILE);
        let pointer_backup_path = bootstrap_dir.join(POINTER_BACKUP_FILE);
        let legacy_bootstrap_pointer = bootstrap_dir.join(LEGACY_POINTER_FILE);
        let legacy_portable_pointer = executable_dir.join(LEGACY_POINTER_FILE);
        let legacy_portable_root = executable_dir.join(LEGACY_PORTABLE_DATA_DIR);
        let legacy_default_root = app_scoped_data_dir.join(legacy_default_data_dir(service));
        let default_root = local_data_dir.join(default_data_dir(service));
        let mut pointer = read_secure_pointer(&pointer_path, &key, service)?;
        if pointer.is_none() && pointer_backup_path.is_file() {
            pointer = read_secure_pointer(&pointer_backup_path, &key, service)?;
        }
        let mut plaintext_pointer_source = None;
        if pointer.is_none() && legacy_bootstrap_pointer.is_file() {
            pointer = read_plaintext_pointer(&legacy_bootstrap_pointer)?;
            plaintext_pointer_source = Some(legacy_bootstrap_pointer.clone());
        }
        if pointer.is_none() && legacy_portable_pointer.is_file() {
            pointer = read_plaintext_pointer(&legacy_portable_pointer)?;
            plaintext_pointer_source = Some(legacy_portable_pointer.clone());
        }
        let requested_root = pointer
            .as_ref()
            .map(|item| PathBuf::from(&item.data_root))
            .unwrap_or_else(|| default_root.clone());
        let data_root = prepare_root(&requested_root)?;

        if let (Some(source), Some(pointer)) = (plaintext_pointer_source.as_ref(), pointer.as_ref())
        {
            write_secure_pointer(&pointer_path, &pointer_backup_path, pointer, &key, service)?;
            fs::remove_file(source).map_err(|error| format!("无法删除旧明文存储位置：{error}"))?;
        }

        if let Some(migrate_from) = pointer
            .as_ref()
            .and_then(|item| item.migrate_from.as_deref())
            .map(PathBuf::from)
        {
            migrate_managed_root(&migrate_from, &data_root)?;
            write_secure_pointer(
                &pointer_path,
                &pointer_backup_path,
                &StoragePointer {
                    data_root: path_text(&data_root),
                    migrate_from: None,
                },
                &key,
                service,
            )?;
        } else if pointer.is_none() {
            if legacy_default_root != data_root
                && legacy_default_root.join(STORAGE_MARKER).is_file()
            {
                copy_managed_root(&legacy_default_root, &data_root)?;
            }
            if legacy_portable_root != data_root
                && legacy_portable_root != legacy_default_root
                && legacy_portable_root.join(STORAGE_MARKER).is_file()
            {
                copy_managed_root(&legacy_portable_root, &data_root)?;
            }
            migrate_legacy_file(legacy_settings_path, &data_root.join(LEGACY_SETTINGS_FILE))?;
            migrate_legacy_webview(legacy_webview_path, &data_root.join(WEBVIEW_DIR))?;
        }

        Ok(Self {
            active_root: data_root.clone(),
            configured_root: RwLock::new(data_root.clone()),
            default_root,
            pointer_path,
            key,
            service: service.into(),
            restart_required: AtomicBool::new(false),
        })
    }

    pub fn initialize_locked(
        legacy_settings_path: &Path,
        legacy_webview_path: &Path,
        key: Zeroizing<Vec<u8>>,
        service: &str,
    ) -> Result<Self, String> {
        let bootstrap_dir = legacy_settings_path
            .parent()
            .ok_or_else(|| "无法确定应用配置目录".to_string())?;
        let app_scoped_data_dir = legacy_webview_path
            .parent()
            .ok_or_else(|| "无法确定应用数据目录".to_string())?;
        let local_data_dir = app_scoped_data_dir
            .parent()
            .ok_or_else(|| "无法确定系统应用数据目录".to_string())?;
        let default_root = prepare_root(&local_data_dir.join(default_data_dir(service)))?;
        Ok(Self {
            active_root: default_root.clone(),
            configured_root: RwLock::new(default_root.clone()),
            default_root,
            pointer_path: bootstrap_dir.join(POINTER_FILE),
            key,
            service: service.into(),
            restart_required: AtomicBool::new(false),
        })
    }

    pub fn active_webview_path(&self) -> PathBuf {
        self.active_root.join(WEBVIEW_DIR)
    }

    pub fn active_root(&self) -> &Path {
        &self.active_root
    }

    pub fn default_root(&self) -> &Path {
        &self.default_root
    }

    pub fn configured_root(&self) -> Result<PathBuf, String> {
        self.configured_root
            .read()
            .map(|path| path.clone())
            .map_err(|error| error.to_string())
    }

    pub fn info(&self) -> Result<StorageInfo, String> {
        let root = self.configured_root()?;
        let webview = root.join(WEBVIEW_DIR);
        Ok(StorageInfo {
            data_root: path_text(&root),
            default_data_root: path_text(&self.default_root),
            webview_data_root: path_text(&webview),
            total_bytes: directory_size(&root),
            safe_cache_bytes: safe_cache_size(&webview),
            cleanup_pending: root.join(CLEANUP_MARKER).exists(),
            restart_required: self.restart_required.load(Ordering::Relaxed),
            is_default: same_path(&root, &self.default_root),
        })
    }

    pub fn configure_root(&self, requested: &Path) -> Result<StorageInfo, String> {
        if self.restart_required.load(Ordering::Relaxed) {
            return Err("存储位置已经修改，请重启应用后再进行下一次修改".into());
        }
        let root = prepare_root(requested)?;
        let current = self.configured_root()?;
        if same_path(&root, &current) {
            return self.info();
        }
        let pointer_backup = self.pointer_path.with_file_name(POINTER_BACKUP_FILE);
        write_secure_pointer(
            &self.pointer_path,
            &pointer_backup,
            &StoragePointer {
                data_root: path_text(&root),
                migrate_from: Some(path_text(&self.active_root)),
            },
            &self.key,
            &self.service,
        )?;
        *self
            .configured_root
            .write()
            .map_err(|error| error.to_string())? = root.clone();
        self.restart_required.store(true, Ordering::Relaxed);
        self.info()
    }

    pub fn schedule_cleanup(&self) -> Result<StorageInfo, String> {
        let root = self.configured_root()?;
        verify_managed_root(&root)?;
        fs::write(root.join(CLEANUP_MARKER), b"cleanup on next launch\n")
            .map_err(|error| format!("无法安排缓存清理：{error}"))?;
        self.info()
    }

    pub fn run_startup_cleanup(&self, automatic: bool) -> Result<u64, String> {
        let marker = self.active_root.join(CLEANUP_MARKER);
        if !automatic && !marker.exists() {
            return Ok(0);
        }
        let freed = safe_cleanup(&self.active_root)?;
        if marker.exists() {
            fs::remove_file(&marker).map_err(|error| format!("无法移除清理标记：{error}"))?;
        }
        Ok(freed)
    }
}

fn prepare_root(requested: &Path) -> Result<PathBuf, String> {
    if !requested.is_absolute() {
        return Err("存储目录必须是绝对路径".into());
    }
    fs::create_dir_all(requested).map_err(|error| format!("无法创建存储目录：{error}"))?;
    let root = requested
        .canonicalize()
        .map_err(|error| format!("无法解析存储目录：{error}"))?;
    let probe = root.join(format!(".write-test-{}", std::process::id()));
    fs::write(&probe, b"ok").map_err(|error| format!("存储目录不可写：{error}"))?;
    fs::remove_file(&probe).map_err(|error| format!("无法完成目录写入测试：{error}"))?;
    fs::write(
        root.join(STORAGE_MARKER),
        b"Interview Buddy managed storage\n",
    )
    .map_err(|error| format!("无法初始化存储目录：{error}"))?;
    fs::create_dir_all(root.join(WEBVIEW_DIR))
        .map_err(|error| format!("无法创建 WebView2 数据目录：{error}"))?;
    Ok(root)
}

fn verify_managed_root(root: &Path) -> Result<(), String> {
    if !root.is_absolute() || !root.join(STORAGE_MARKER).is_file() {
        return Err("拒绝操作未经 Interview Buddy 标记的目录".into());
    }
    Ok(())
}

fn read_plaintext_pointer(path: &Path) -> Result<Option<StoragePointer>, String> {
    if !path.exists() {
        return Ok(None);
    }
    if fs::metadata(path)
        .map_err(|error| format!("无法检查存储引导文件大小：{error}"))?
        .len()
        > MAX_PLAINTEXT_POINTER
    {
        return Err("明文存储引导文件超过 64 KiB 安全上限".into());
    }
    let text =
        fs::read_to_string(path).map_err(|error| format!("无法读取存储引导文件：{error}"))?;
    let pointer: StoragePointer =
        serde_json::from_str(&text).map_err(|error| format!("存储引导文件格式错误：{error}"))?;
    if !Path::new(&pointer.data_root).is_absolute() {
        return Err("存储引导文件中的目录不是绝对路径".into());
    }
    Ok(Some(pointer))
}

pub fn quarantine_pointer(config_dir: &Path) -> Result<Option<PathBuf>, String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let quarantine = config_dir.join(format!("recovery-{timestamp}"));
    let mut moved = false;
    for name in [POINTER_FILE, POINTER_BACKUP_FILE, LEGACY_POINTER_FILE] {
        let source = config_dir.join(name);
        if source.is_file() {
            fs::create_dir_all(&quarantine)
                .map_err(|error| format!("无法创建存储恢复目录：{error}"))?;
            fs::rename(&source, quarantine.join(name))
                .map_err(|error| format!("无法隔离存储位置文件：{error}"))?;
            moved = true;
        }
    }
    Ok(moved.then_some(quarantine))
}

fn read_secure_pointer(
    path: &Path,
    key: &[u8],
    service: &str,
) -> Result<Option<StoragePointer>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let encoded = fs::read(path).map_err(|error| format!("无法读取加密存储位置：{error}"))?;
    let plaintext = decrypt_envelope(key, service, EnvelopeKind::StoragePointer, &encoded)?;
    let pointer: StoragePointer = serde_json::from_slice(&plaintext)
        .map_err(|error| format!("加密存储位置格式无效：{error}"))?;
    validate_pointer(&pointer)?;
    Ok(Some(pointer))
}

fn write_secure_pointer(
    path: &Path,
    backup: &Path,
    pointer: &StoragePointer,
    key: &[u8],
    service: &str,
) -> Result<(), String> {
    validate_pointer(pointer)?;
    let plaintext = serde_json::to_vec(pointer).map_err(|error| error.to_string())?;
    let encoded = encrypt_envelope(key, service, EnvelopeKind::StoragePointer, &plaintext)?;
    decrypt_envelope(key, service, EnvelopeKind::StoragePointer, &encoded)?;
    atomic_replace(path, backup, &encoded)
}

fn validate_pointer(pointer: &StoragePointer) -> Result<(), String> {
    if !Path::new(&pointer.data_root).is_absolute() {
        return Err("存储引导文件中的目录不是绝对路径".into());
    }
    if pointer
        .migrate_from
        .as_deref()
        .is_some_and(|path| !Path::new(path).is_absolute())
    {
        return Err("存储引导文件中的迁移目录不是绝对路径".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "interview-buddy-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn cleanup_removes_cache_and_preserves_identity_data() {
        let root = test_root("cleanup");
        prepare_root(&root).expect("prepare");
        let cache = root.join(WEBVIEW_DIR).join("Default/Cache/item.bin");
        let identity = root.join(WEBVIEW_DIR).join("Default/MediaDeviceSalts");
        fs::create_dir_all(cache.parent().expect("cache parent")).expect("cache dir");
        fs::create_dir_all(identity.parent().expect("identity parent")).expect("identity dir");
        fs::write(&cache, [1, 2, 3]).expect("cache file");
        fs::write(&identity, [4, 5, 6]).expect("identity file");
        fs::write(root.join(SETTINGS_FILE), b"encrypted-settings").expect("settings file");
        fs::write(root.join(SETTINGS_BACKUP_FILE), b"encrypted-backup").expect("backup file");

        assert_eq!(safe_cleanup(&root).expect("cleanup"), 3);
        assert!(!cache.exists());
        assert!(identity.exists());
        assert!(root.join(SETTINGS_FILE).exists());
        assert!(root.join(SETTINGS_BACKUP_FILE).exists());
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn migration_preserves_new_settings_and_merges_webview_identity() {
        let source = test_root("migration-source");
        let destination = test_root("migration-destination");
        prepare_root(&source).expect("prepare source");
        prepare_root(&destination).expect("prepare destination");
        fs::write(source.join(SETTINGS_FILE), b"old").expect("old settings");
        fs::write(destination.join(SETTINGS_FILE), b"new").expect("new settings");
        let identity = source.join(WEBVIEW_DIR).join("Default/MediaDeviceSalts");
        fs::create_dir_all(identity.parent().expect("identity parent")).expect("identity dir");
        fs::write(&identity, [7, 8, 9]).expect("identity file");

        migrate_managed_root(&source, &destination).expect("migrate");

        assert_eq!(
            fs::read(destination.join(SETTINGS_FILE)).expect("destination settings"),
            b"new"
        );
        assert_eq!(
            fs::read(
                destination
                    .join(WEBVIEW_DIR)
                    .join("Default/MediaDeviceSalts")
            )
            .expect("destination identity"),
            [7, 8, 9]
        );
        assert!(!source.join(SETTINGS_FILE).exists());
        assert!(!source.join(WEBVIEW_DIR).exists());
        fs::remove_dir_all(source).expect("remove source root");
        fs::remove_dir_all(destination).expect("remove destination root");
    }

    #[test]
    fn portable_migration_copies_without_modifying_the_bundle_source() {
        let source = test_root("portable-source");
        let destination = test_root("portable-destination");
        prepare_root(&source).expect("prepare source");
        prepare_root(&destination).expect("prepare destination");
        fs::write(source.join(SETTINGS_FILE), b"portable").expect("portable settings");

        copy_managed_root(&source, &destination).expect("copy portable root");

        assert_eq!(
            fs::read(destination.join(SETTINGS_FILE)).expect("destination settings"),
            b"portable"
        );
        assert_eq!(
            fs::read(source.join(SETTINGS_FILE)).expect("source settings"),
            b"portable"
        );
        fs::remove_dir_all(source).expect("remove source root");
        fs::remove_dir_all(destination).expect("remove destination root");
    }

    #[test]
    fn portable_copy_does_not_restore_plaintext_when_encrypted_settings_exist() {
        let source = test_root("portable-plaintext-source");
        let destination = test_root("portable-encrypted-destination");
        prepare_root(&source).expect("prepare source");
        prepare_root(&destination).expect("prepare destination");
        fs::write(source.join(LEGACY_SETTINGS_FILE), b"legacy plaintext").expect("legacy settings");
        fs::write(destination.join(SETTINGS_FILE), b"encrypted envelope")
            .expect("encrypted settings");

        copy_managed_root(&source, &destination).expect("copy portable root");

        assert!(!destination.join(LEGACY_SETTINGS_FILE).exists());
        assert!(source.join(LEGACY_SETTINGS_FILE).is_file());
        fs::remove_dir_all(source).expect("remove source root");
        fs::remove_dir_all(destination).expect("remove destination root");
    }

    #[test]
    fn storage_pointer_is_encrypted_and_bound_to_its_kind() {
        let root = test_root("secure-pointer");
        fs::create_dir_all(&root).expect("root");
        let path = root.join(POINTER_FILE);
        let backup = root.join(POINTER_BACKUP_FILE);
        let key = Zeroizing::new(vec![8u8; 32]);
        let pointer = StoragePointer {
            data_root: path_text(&root.join("private-location")),
            migrate_from: None,
        };
        write_secure_pointer(&path, &backup, &pointer, &key, "test.app").expect("write");
        let encoded = fs::read_to_string(&path).expect("encrypted pointer");
        assert!(!encoded.contains("private-location"));
        let loaded = read_secure_pointer(&path, &key, "test.app")
            .expect("read")
            .expect("pointer");
        assert_eq!(loaded.data_root, pointer.data_root);
        assert!(
            decrypt_envelope(&key, "test.app", EnvelopeKind::Settings, encoded.as_bytes()).is_err()
        );
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn development_storage_uses_an_isolated_directory_name() {
        assert_eq!(
            default_data_dir("com.oooorca.interview-buddy"),
            ".interview-buddy"
        );
        assert_eq!(
            default_data_dir("com.oooorca.interview-buddy.dev"),
            ".interview-buddy-dev"
        );
        assert_eq!(
            legacy_default_data_dir("com.oooorca.interview-buddy"),
            "cache"
        );
    }

    #[test]
    fn corrupt_encrypted_pointer_fails_closed_without_plaintext_fallback() {
        let root = test_root("corrupt-pointer");
        fs::create_dir_all(&root).expect("root");
        let secure = root.join(POINTER_FILE);
        let plaintext = root.join(LEGACY_POINTER_FILE);
        fs::write(&secure, b"corrupted").expect("secure pointer");
        fs::write(
            &plaintext,
            serde_json::to_vec(&StoragePointer {
                data_root: path_text(&root.join("plaintext-root")),
                migrate_from: None,
            })
            .expect("serialize"),
        )
        .expect("plaintext pointer");
        assert!(read_secure_pointer(&secure, &[1u8; 32], "test.app").is_err());
        assert!(secure.exists());
        assert!(plaintext.exists());
        fs::remove_dir_all(root).expect("remove test root");
    }
}
