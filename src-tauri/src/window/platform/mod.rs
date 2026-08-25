#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod implementation;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[path = "unsupported.rs"]
mod implementation;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod implementation;

pub(super) use implementation::{build_main_window, configure_overlay, verify_capture_protection};
