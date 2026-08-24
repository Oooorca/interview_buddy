mod pcm;

#[cfg(target_os = "macos")]
#[path = "system_macos.rs"]
pub mod system;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[path = "system_unsupported.rs"]
pub mod system;
#[cfg(target_os = "windows")]
#[path = "system_windows.rs"]
pub mod system;

pub use pcm::pcm_wav;
