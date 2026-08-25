#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod implementation;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[path = "unsupported.rs"]
mod implementation;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod implementation;

pub(crate) use implementation::path_text;
pub(super) use implementation::same_path;
