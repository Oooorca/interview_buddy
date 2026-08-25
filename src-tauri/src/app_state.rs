use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Mutex, RwLock},
};

use crate::{audio, settings, storage};

#[derive(Debug, Clone, Copy)]
pub(crate) struct RegionCaptureSession {
    pub(crate) restore_main_window: bool,
    #[cfg(target_os = "macos")]
    pub(crate) monitor_x: i32,
    #[cfg(target_os = "macos")]
    pub(crate) monitor_y: i32,
    #[cfg(target_os = "macos")]
    pub(crate) monitor_scale: f64,
}

pub(crate) struct AppState {
    pub(crate) settings: RwLock<settings::AppSettings>,
    pub(crate) settings_store: RwLock<Option<settings::store::SettingsStore>>,
    pub(crate) settings_security_state: RwLock<settings::SecurityState>,
    pub(crate) security_error: RwLock<Option<String>>,
    pub(crate) config_dir: PathBuf,
    pub(crate) service: String,
    pub(crate) storage: storage::StorageManager,
    pub(crate) shortcut_warnings: RwLock<Vec<String>>,
    pub(crate) system_audio: Mutex<Option<audio::system::SystemAudioRecorder>>,
    pub(crate) region_capture: Mutex<Option<RegionCaptureSession>>,
    pub(crate) cancelled_requests: Mutex<HashSet<String>>,
}
