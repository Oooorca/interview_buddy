use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Mutex, RwLock},
};

use crate::{audio, capture::RegionCaptureSession, settings, storage};

pub(crate) struct AppState {
    pub(crate) settings: RwLock<settings::AppSettings>,
    pub(crate) settings_store: RwLock<Option<settings::store::SettingsStore>>,
    pub(crate) settings_security_state: RwLock<settings::SecurityState>,
    pub(crate) security_error: RwLock<Option<String>>,
    pub(crate) config_dir: PathBuf,
    pub(crate) service: String,
    pub(crate) storage: storage::StorageManager,
    pub(crate) shortcut_warnings: RwLock<Vec<String>>,
    pub(crate) system_audio: Mutex<Option<audio::SystemAudioRecorder>>,
    pub(crate) region_capture: Mutex<Option<RegionCaptureSession>>,
    pub(crate) cancelled_requests: Mutex<HashSet<String>>,
}
