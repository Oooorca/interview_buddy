use std::{fs, path::Path};

use super::{
    cleanup::directory_size, same_path, verify_managed_root, LEGACY_SETTINGS_FILE,
    SETTINGS_BACKUP_FILE, SETTINGS_FILE, WEBVIEW_DIR,
};

pub(super) fn migrate_managed_root(source: &Path, destination: &Path) -> Result<(), String> {
    if same_path(source, destination) {
        return Ok(());
    }
    verify_managed_root(source)?;
    verify_managed_root(destination)?;
    for file in [SETTINGS_FILE, SETTINGS_BACKUP_FILE, LEGACY_SETTINGS_FILE] {
        let source_settings = source.join(file);
        let destination_settings = destination.join(file);
        if source_settings.exists() {
            if !destination_settings.exists() {
                fs::copy(&source_settings, &destination_settings)
                    .map_err(|error| format!("迁移设置失败：{error}"))?;
            }
            fs::remove_file(&source_settings)
                .map_err(|error| format!("清理旧设置失败：{error}"))?;
        }
    }
    migrate_directory(&source.join(WEBVIEW_DIR), &destination.join(WEBVIEW_DIR))?;
    Ok(())
}

pub(super) fn copy_managed_root(source: &Path, destination: &Path) -> Result<(), String> {
    verify_managed_root(source)?;
    verify_managed_root(destination)?;
    let destination_has_encrypted_settings = destination.join(SETTINGS_FILE).is_file()
        || destination.join(SETTINGS_BACKUP_FILE).is_file();
    for file in [SETTINGS_FILE, SETTINGS_BACKUP_FILE, LEGACY_SETTINGS_FILE] {
        if file == LEGACY_SETTINGS_FILE && destination_has_encrypted_settings {
            continue;
        }
        let source_settings = source.join(file);
        let destination_settings = destination.join(file);
        if source_settings.is_file() && !destination_settings.exists() {
            fs::copy(source_settings, destination_settings)
                .map_err(|error| format!("迁移便携版设置失败：{error}"))?;
        }
    }
    copy_directory(&source.join(WEBVIEW_DIR), &destination.join(WEBVIEW_DIR))
}

pub(super) fn migrate_legacy_file(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_file() || destination.exists() {
        return Ok(());
    }
    fs::copy(source, destination).map_err(|error| format!("迁移旧设置失败：{error}"))?;
    fs::remove_file(source).map_err(|error| format!("清理旧设置失败：{error}"))
}

pub(super) fn migrate_legacy_webview(source: &Path, destination: &Path) -> Result<(), String> {
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
    if !source.is_dir() {
        return Ok(());
    }
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
