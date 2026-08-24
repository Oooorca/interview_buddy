use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        RwLock,
    },
};

const POINTER_FILE: &str = "storage-location.json";
const DEFAULT_DATA_DIR: &str = "cache";
const SETTINGS_FILE: &str = "settings.json";
const WEBVIEW_DIR: &str = "webview2";
const STORAGE_MARKER: &str = ".interview-buddy-storage";
const CLEANUP_MARKER: &str = ".cleanup-pending";

const SAFE_CACHE_PATHS: &[&str] = &[
    "Default/Cache",
    "Default/Code Cache",
    "Default/GPUCache",
    "Default/DawnGraphiteCache",
    "Default/DawnWebGPUCache",
    "GPUCache",
    "ShaderCache",
    "GrShaderCache",
    "GPUPersistentCache",
    "component_crx_cache",
    "extensions_crx_cache",
    "Crashpad/reports",
    "Crashpad/attachments",
];

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
    settings_path: RwLock<PathBuf>,
    restart_required: AtomicBool,
}

impl StorageManager {
    pub fn initialize(
        legacy_settings_path: &Path,
        legacy_webview_path: &Path,
    ) -> Result<Self, String> {
        let executable =
            std::env::current_exe().map_err(|error| format!("无法确定程序路径：{error}"))?;
        let executable_dir = executable
            .parent()
            .ok_or_else(|| "无法确定程序所在目录".to_string())?
            .to_path_buf();
        let pointer_path = executable_dir.join(POINTER_FILE);
        let default_root = executable_dir.join(DEFAULT_DATA_DIR);
        let pointer = read_pointer(&pointer_path)?;
        let requested_root = pointer
            .as_ref()
            .map(|item| PathBuf::from(&item.data_root))
            .unwrap_or_else(|| default_root.clone());
        let data_root = prepare_root(&requested_root)?;

        if let Some(migrate_from) = pointer
            .as_ref()
            .and_then(|item| item.migrate_from.as_deref())
            .map(PathBuf::from)
        {
            migrate_managed_root(&migrate_from, &data_root)?;
            write_pointer(
                &pointer_path,
                &StoragePointer {
                    data_root: path_text(&data_root),
                    migrate_from: None,
                },
            )?;
        } else if pointer.is_none() {
            migrate_legacy_file(legacy_settings_path, &data_root.join(SETTINGS_FILE))?;
            migrate_legacy_webview(legacy_webview_path, &data_root.join(WEBVIEW_DIR))?;
        }

        Ok(Self {
            active_root: data_root.clone(),
            configured_root: RwLock::new(data_root.clone()),
            default_root,
            pointer_path,
            settings_path: RwLock::new(data_root.join(SETTINGS_FILE)),
            restart_required: AtomicBool::new(false),
        })
    }

    pub fn settings_path(&self) -> Result<PathBuf, String> {
        self.settings_path
            .read()
            .map(|path| path.clone())
            .map_err(|error| error.to_string())
    }

    pub fn active_webview_path(&self) -> PathBuf {
        self.active_root.join(WEBVIEW_DIR)
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

    pub fn configure_root(
        &self,
        requested: &Path,
        settings_json: &str,
    ) -> Result<StorageInfo, String> {
        if self.restart_required.load(Ordering::Relaxed) {
            return Err("存储位置已经修改，请重启应用后再进行下一次修改".into());
        }
        let root = prepare_root(requested)?;
        let current = self.configured_root()?;
        if same_path(&root, &current) {
            return self.info();
        }
        fs::write(root.join(SETTINGS_FILE), settings_json)
            .map_err(|error| format!("无法写入新设置目录：{error}"))?;
        write_pointer(
            &self.pointer_path,
            &StoragePointer {
                data_root: path_text(&root),
                migrate_from: Some(path_text(&self.active_root)),
            },
        )?;
        *self
            .configured_root
            .write()
            .map_err(|error| error.to_string())? = root.clone();
        *self
            .settings_path
            .write()
            .map_err(|error| error.to_string())? = root.join(SETTINGS_FILE);
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

fn read_pointer(path: &Path) -> Result<Option<StoragePointer>, String> {
    if !path.exists() {
        return Ok(None);
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

fn write_pointer(path: &Path, pointer: &StoragePointer) -> Result<(), String> {
    let text = serde_json::to_string_pretty(pointer).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| format!("无法写入 EXE 同级存储引导文件：{error}"))
}

fn migrate_managed_root(source: &Path, destination: &Path) -> Result<(), String> {
    if same_path(source, destination) {
        return Ok(());
    }
    verify_managed_root(source)?;
    verify_managed_root(destination)?;
    let source_settings = source.join(SETTINGS_FILE);
    let destination_settings = destination.join(SETTINGS_FILE);
    if source_settings.exists() {
        if !destination_settings.exists() {
            fs::copy(&source_settings, &destination_settings)
                .map_err(|error| format!("迁移设置失败：{error}"))?;
        }
        fs::remove_file(&source_settings).map_err(|error| format!("清理旧设置失败：{error}"))?;
    }
    migrate_directory(&source.join(WEBVIEW_DIR), &destination.join(WEBVIEW_DIR))?;
    Ok(())
}

fn migrate_legacy_file(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_file() || destination.exists() {
        return Ok(());
    }
    fs::copy(source, destination).map_err(|error| format!("迁移旧设置失败：{error}"))?;
    fs::remove_file(source).map_err(|error| format!("清理旧设置失败：{error}"))
}

fn migrate_legacy_webview(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() || directory_size(source) == 0 {
        return Ok(());
    }
    migrate_directory(source, destination)
}

fn migrate_directory(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Ok(());
    }
    if destination.exists() && directory_size(destination) > 0 {
        copy_directory(source, destination)?;
        return remove_directory_checked(source);
    } else if destination.exists() {
        fs::remove_dir_all(destination)
            .map_err(|error| format!("准备迁移目标目录失败：{error}"))?;
    }
    if fs::rename(source, destination).is_ok() {
        return Ok(());
    }
    copy_directory(source, destination)?;
    remove_directory_checked(source)
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn remove_directory_checked(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || path.parent().is_none() {
        return Err("拒绝删除不安全的目录路径".into());
    }
    fs::remove_dir_all(path).map_err(|error| format!("删除旧数据目录失败：{error}"))
}

fn safe_cleanup(root: &Path) -> Result<u64, String> {
    verify_managed_root(root)?;
    let webview_root = root.join(WEBVIEW_DIR);
    let before = safe_cache_size(&webview_root);
    for relative in SAFE_CACHE_PATHS {
        let target = webview_root.join(relative);
        if !target.starts_with(&webview_root) {
            return Err("拒绝清理 WebView2 数据目录之外的路径".into());
        }
        if target.is_dir() {
            fs::remove_dir_all(&target)
                .map_err(|error| format!("清理 {} 失败：{error}", target.display()))?;
        } else if target.is_file() {
            fs::remove_file(&target)
                .map_err(|error| format!("清理 {} 失败：{error}", target.display()))?;
        }
    }
    Ok(before.saturating_sub(safe_cache_size(&webview_root)))
}

fn safe_cache_size(webview_root: &Path) -> u64 {
    SAFE_CACHE_PATHS
        .iter()
        .map(|relative| directory_size(&webview_root.join(relative)))
        .sum()
}

fn directory_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| directory_size(&entry.path()))
        .sum()
}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        path_text(left).eq_ignore_ascii_case(&path_text(right))
    } else {
        left == right
    }
}

fn path_text(path: &Path) -> String {
    let text = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
    }
    text.into_owned()
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

        assert_eq!(safe_cleanup(&root).expect("cleanup"), 3);
        assert!(!cache.exists());
        assert!(identity.exists());
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

    #[cfg(windows)]
    #[test]
    fn display_path_hides_windows_extended_length_prefix() {
        assert_eq!(
            path_text(Path::new(r"\\?\C:\Tools\Interview Buddy\cache")),
            r"C:\Tools\Interview Buddy\cache"
        );
        assert_eq!(
            path_text(Path::new(r"\\?\UNC\server\share\cache")),
            r"\\server\share\cache"
        );
    }
}
