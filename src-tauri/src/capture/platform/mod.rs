#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod implementation;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[path = "unsupported.rs"]
mod implementation;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod implementation;

#[cfg(not(target_os = "macos"))]
mod non_macos;

pub(super) use implementation::{
    capture, create_selector, ensure_permission, monitor_at, prepare_request, CaptureContext,
};
