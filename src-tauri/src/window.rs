use tauri::Manager;

#[tauri::command]
pub(crate) fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

pub(crate) fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn query_display_affinity(window: &tauri::WebviewWindow) -> Result<u32, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowDisplayAffinity;
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
