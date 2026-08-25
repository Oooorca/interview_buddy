use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::GetWindowDisplayAffinity};

use crate::{error::AppResult, storage::StorageManager};

pub(crate) fn build_main_window(
    app: &tauri::AppHandle,
    config: &tauri::utils::config::WindowConfig,
    storage: &StorageManager,
) -> AppResult<tauri::WebviewWindow> {
    Ok(tauri::WebviewWindowBuilder::from_config(app, config)
        .map_err(|error| error.to_string())?
        .data_directory(storage.active_webview_path())
        .build()
        .map_err(|error| error.to_string())?)
}

pub(crate) fn configure_overlay(_window: &tauri::WebviewWindow) -> AppResult<()> {
    Ok(())
}

fn query_display_affinity(window: &tauri::WebviewWindow) -> Result<u32, String> {
    let handle = window.window_handle().map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return Err("当前窗口不是 Win32 窗口".into());
    };
    let hwnd = HWND(win32.hwnd.get() as *mut std::ffi::c_void);
    let mut affinity = 0u32;
    unsafe { GetWindowDisplayAffinity(hwnd, &mut affinity) }
        .map_err(|error| format!("读取窗口捕获保护失败：{error}"))?;
    Ok(affinity)
}

pub(crate) fn verify_capture_protection(window: &tauri::WebviewWindow) {
    match query_display_affinity(window) {
        Ok(affinity) => eprintln!("{} display affinity: 0x{affinity:X}", window.label()),
        Err(error) => eprintln!("{} display affinity check failed: {error}", window.label()),
    }
}
