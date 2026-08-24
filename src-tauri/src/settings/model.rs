use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptMode {
    #[default]
    Default,
    Custom,
    Disabled,
}

#[derive(Clone, Default, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(transparent)]
pub struct SecretString(pub(crate) String);

impl SecretString {
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub(crate) base_url: String,
    pub(crate) api_key: SecretString,
    pub(crate) model: String,
    pub(crate) vision_model: String,
    pub(crate) transcription_model: String,
    pub(crate) capture_microphone: bool,
    pub(crate) capture_system_audio: bool,
    pub(crate) microphone_device_id: String,
    pub(crate) system_audio_device_id: String,
    pub(crate) my_transcription_language: String,
    pub(crate) their_transcription_language: String,
    pub(crate) auto_safe_cleanup: bool,
    pub(crate) fixed_context: String,
    pub(crate) transcription_language: String,
    pub(crate) system_prompt_mode: PromptMode,
    pub(crate) coding_prompt_mode: PromptMode,
    pub(crate) system_prompt: Option<String>,
    pub(crate) coding_prompt: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".into(),
            api_key: SecretString::default(),
            model: "gpt-4.1-mini".into(),
            vision_model: "gpt-4.1".into(),
            transcription_model: "gpt-4o-mini-transcribe".into(),
            capture_microphone: true,
            capture_system_audio: true,
            microphone_device_id: String::new(),
            system_audio_device_id: String::new(),
            my_transcription_language: "auto".into(),
            their_transcription_language: "auto".into(),
            auto_safe_cleanup: false,
            fixed_context: String::new(),
            transcription_language: "auto".into(),
            system_prompt_mode: PromptMode::Default,
            coding_prompt_mode: PromptMode::Default,
            system_prompt: None,
            coding_prompt: None,
        }
    }
}

impl std::fmt::Debug for AppSettings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppSettings")
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .field("model", &self.model)
            .field("vision_model", &self.vision_model)
            .field("transcription_model", &self.transcription_model)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PublicSettings {
    pub base_url: String,
    pub model: String,
    pub vision_model: String,
    pub transcription_model: String,
    pub capture_microphone: bool,
    pub capture_system_audio: bool,
    pub microphone_device_id: String,
    pub system_audio_device_id: String,
    pub my_transcription_language: String,
    pub their_transcription_language: String,
    pub auto_safe_cleanup: bool,
    pub fixed_context: String,
    pub system_prompt_mode: PromptMode,
    pub coding_prompt_mode: PromptMode,
    pub system_prompt: Option<String>,
    pub coding_prompt: Option<String>,
}

impl Default for PublicSettings {
    fn default() -> Self {
        Self::from(&AppSettings::default())
    }
}

impl From<&AppSettings> for PublicSettings {
    fn from(settings: &AppSettings) -> Self {
        Self {
            base_url: settings.base_url.clone(),
            model: settings.model.clone(),
            vision_model: settings.vision_model.clone(),
            transcription_model: settings.transcription_model.clone(),
            capture_microphone: settings.capture_microphone,
            capture_system_audio: settings.capture_system_audio,
            microphone_device_id: settings.microphone_device_id.clone(),
            system_audio_device_id: settings.system_audio_device_id.clone(),
            my_transcription_language: settings.my_transcription_language.clone(),
            their_transcription_language: settings.their_transcription_language.clone(),
            auto_safe_cleanup: settings.auto_safe_cleanup,
            fixed_context: settings.fixed_context.clone(),
            system_prompt_mode: settings.system_prompt_mode,
            coding_prompt_mode: settings.coding_prompt_mode,
            system_prompt: settings.system_prompt.clone(),
            coding_prompt: settings.coding_prompt.clone(),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum ApiKeyUpdate {
    Keep,
    Replace { value: SecretString },
    Clear,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsRequest {
    pub settings: PublicSettings,
    pub api_key_update: ApiKeyUpdate,
}

impl AppSettings {
    pub fn apply_update(&self, request: SaveSettingsRequest) -> Result<Self, String> {
        let public = request.settings;
        let api_key = match request.api_key_update {
            ApiKeyUpdate::Keep => self.api_key.clone(),
            ApiKeyUpdate::Replace { value } if !value.is_empty() => value,
            ApiKeyUpdate::Replace { .. } => return Err("新的 API Key 不能为空".into()),
            ApiKeyUpdate::Clear => SecretString::default(),
        };
        Ok(Self {
            base_url: public.base_url,
            api_key,
            model: public.model,
            vision_model: public.vision_model,
            transcription_model: public.transcription_model,
            capture_microphone: public.capture_microphone,
            capture_system_audio: public.capture_system_audio,
            microphone_device_id: public.microphone_device_id,
            system_audio_device_id: public.system_audio_device_id,
            my_transcription_language: public.my_transcription_language,
            their_transcription_language: public.their_transcription_language,
            auto_safe_cleanup: public.auto_safe_cleanup,
            fixed_context: public.fixed_context,
            transcription_language: "auto".into(),
            system_prompt_mode: public.system_prompt_mode,
            coding_prompt_mode: public.coding_prompt_mode,
            system_prompt: public.system_prompt,
            coding_prompt: public.coding_prompt,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecurityState {
    Ready,
    Migrated,
    Recovered,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub settings: PublicSettings,
    pub api_key_configured: bool,
    pub security_state: SecurityState,
}

impl SettingsSnapshot {
    pub fn new(settings: &AppSettings, security_state: SecurityState) -> Self {
        Self {
            settings: PublicSettings::from(settings),
            api_key_configured: !settings.api_key.is_empty(),
            security_state,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SettingsLoadResult {
    Ready { snapshot: Box<SettingsSnapshot> },
    Locked { reason: String, message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityResetResult {
    pub snapshot: SettingsSnapshot,
    pub quarantine_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(settings: &AppSettings, api_key_update: ApiKeyUpdate) -> SaveSettingsRequest {
        SaveSettingsRequest {
            settings: PublicSettings::from(settings),
            api_key_update,
        }
    }

    #[test]
    fn public_snapshot_never_serializes_api_key() {
        let settings = AppSettings {
            api_key: SecretString("synthetic-private-key".into()),
            ..AppSettings::default()
        };
        let encoded =
            serde_json::to_string(&SettingsSnapshot::new(&settings, SecurityState::Ready)).unwrap();
        assert!(!encoded.contains("synthetic-private-key"));
        assert!(!encoded.contains("apiKey\""));
        assert!(encoded.contains("apiKeyConfigured"));
        assert!(!format!("{settings:?}").contains("synthetic-private-key"));
    }

    #[test]
    fn api_key_keep_replace_and_clear_are_explicit() {
        let settings = AppSettings {
            api_key: SecretString("old-key".into()),
            ..AppSettings::default()
        };

        let kept = settings
            .apply_update(request(&settings, ApiKeyUpdate::Keep))
            .unwrap();
        assert_eq!(kept.api_key.expose(), "old-key");

        let replaced = settings
            .apply_update(request(
                &settings,
                ApiKeyUpdate::Replace {
                    value: SecretString("new-key".into()),
                },
            ))
            .unwrap();
        assert_eq!(replaced.api_key.expose(), "new-key");

        let cleared = settings
            .apply_update(request(&settings, ApiKeyUpdate::Clear))
            .unwrap();
        assert!(cleared.api_key.is_empty());
    }
}
