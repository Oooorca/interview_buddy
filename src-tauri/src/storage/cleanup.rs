use std::{fs, path::Path};

use super::{verify_managed_root, WEBVIEW_DIR};

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

pub(super) fn safe_cleanup(root: &Path) -> Result<u64, String> {
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

pub(super) fn safe_cache_size(webview_root: &Path) -> u64 {
    SAFE_CACHE_PATHS
        .iter()
        .map(|relative| directory_size(&webview_root.join(relative)))
        .sum()
}

pub(super) fn directory_size(path: &Path) -> u64 {
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
