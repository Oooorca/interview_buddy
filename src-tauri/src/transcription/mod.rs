use tauri::State;

use crate::{
    app_state::AppState,
    audio,
    error::AppResult,
    settings::{commands::ensure_security_ready, AppSettings},
};

mod dashscope;
mod openai;

#[tauri::command]
pub(crate) async fn transcribe_audio(
    bytes: Vec<u8>,
    mime_type: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    ensure_security_ready(&state)?;
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    Ok(transcribe_bytes(
        &settings,
        bytes,
        mime_type,
        &settings.my_transcription_language,
    )
    .await?)
}

#[tauri::command]
pub(crate) fn start_system_audio(state: State<'_, AppState>) -> AppResult<()> {
    ensure_security_ready(&state)?;
    let device_id = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .system_audio_device_id
        .trim()
        .to_string();
    let mut current = state
        .system_audio
        .lock()
        .map_err(|error| error.to_string())?;
    if current.is_some() {
        return Err("系统音频已经在录制".into());
    }
    *current = Some(audio::SystemAudioRecorder::start(
        (!device_id.is_empty()).then_some(device_id),
    )?);
    Ok(())
}

#[tauri::command]
pub(crate) fn list_system_audio_devices() -> AppResult<Vec<audio::AudioOutputDevice>> {
    Ok(audio::list_output_devices()?)
}

#[tauri::command]
pub(crate) fn system_audio_level(state: State<'_, AppState>) -> AppResult<f32> {
    Ok(state
        .system_audio
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
        .ok_or_else(|| "系统音频尚未开始录制".to_string())?
        .activity_level()?)
}

#[tauri::command]
pub(crate) fn discard_system_audio_chunk(state: State<'_, AppState>) -> AppResult<()> {
    Ok(state
        .system_audio
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
        .ok_or_else(|| "系统音频尚未开始录制".to_string())?
        .clear_chunk()?)
}

#[tauri::command]
pub(crate) async fn stop_system_audio_and_transcribe(
    state: State<'_, AppState>,
) -> AppResult<String> {
    ensure_security_ready(&state)?;
    let recorder = state
        .system_audio
        .lock()
        .map_err(|error| error.to_string())?
        .take()
        .ok_or_else(|| "系统音频尚未开始录制".to_string())?;
    let wav = recorder.stop()?;
    if wav.is_empty() {
        return Ok(String::new());
    }
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    Ok(transcribe_bytes(
        &settings,
        wav,
        "audio/wav".into(),
        &settings.their_transcription_language,
    )
    .await?)
}

#[tauri::command]
pub(crate) async fn transcribe_system_audio_chunk(state: State<'_, AppState>) -> AppResult<String> {
    ensure_security_ready(&state)?;
    let wav = {
        let current = state
            .system_audio
            .lock()
            .map_err(|error| error.to_string())?;
        let recorder = current
            .as_ref()
            .ok_or_else(|| "系统音频尚未开始录制".to_string())?;
        recorder.take_chunk()?
    };
    if wav.is_empty() {
        return Ok(String::new());
    }
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    Ok(transcribe_bytes(
        &settings,
        wav,
        "audio/wav".into(),
        &settings.their_transcription_language,
    )
    .await?)
}

async fn transcribe_bytes(
    settings: &AppSettings,
    bytes: Vec<u8>,
    mime_type: String,
    language: &str,
) -> Result<String, String> {
    if settings.api_key.is_empty() {
        return Err("请先配置 API Key".into());
    }
    if bytes.is_empty() {
        return Err("录音为空".into());
    }
    let provider_language = if language == "en-US" { "en" } else { language };
    if dashscope::is_dashscope(&settings.base_url) {
        dashscope::transcribe(settings, bytes, &mime_type, provider_language).await
    } else {
        openai::transcribe(settings, bytes, &mime_type, provider_language).await
    }
}

pub(crate) use dashscope::is_dashscope;
