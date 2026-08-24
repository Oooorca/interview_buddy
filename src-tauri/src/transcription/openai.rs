use reqwest::multipart;
use std::time::Duration;

use crate::settings::AppSettings;

pub(super) async fn transcribe(
    settings: &AppSettings,
    bytes: Vec<u8>,
    mime_type: &str,
    language: &str,
) -> Result<String, String> {
    let extension = audio_extension(mime_type);
    let part = multipart::Part::bytes(bytes)
        .file_name(format!("speech.{extension}"))
        .mime_str(mime_type)
        .map_err(|error| error.to_string())?;
    let mut form = multipart::Form::new()
        .text("model", settings.transcription_model.clone())
        .part("file", part);
    let language = language.trim();
    if !language.is_empty() && language != "auto" {
        form = form.text("language", language.to_string());
    }
    let url = format!(
        "{}/audio/transcriptions",
        settings.base_url.trim_end_matches('/')
    );
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?
        .post(url)
        .bearer_auth(settings.api_key.expose())
        .multipart(form)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("读取转写响应失败：{error}"))?;
    let parsed = serde_json::from_str::<serde_json::Value>(&body).ok();
    if !status.is_success() {
        return Err(parsed
            .as_ref()
            .and_then(|value| value.pointer("/error/message"))
            .and_then(|item| item.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("转写服务返回 HTTP {status}：{}", preview(&body))));
    }
    if let Some(text) = parsed
        .as_ref()
        .and_then(|value| value.get("text").or_else(|| value.get("output_text")))
        .and_then(|item| item.as_str())
    {
        return Ok(text.trim().to_string());
    }
    let plain = body.trim();
    if !plain.is_empty() && !plain.starts_with('<') {
        return Ok(plain.to_string());
    }
    Err(format!("转写响应中没有可用文本：{}", preview(&body)))
}

fn audio_extension(mime_type: &str) -> &'static str {
    if mime_type.contains("wav") {
        "wav"
    } else if mime_type.contains("ogg") {
        "ogg"
    } else if mime_type.contains("mpeg") || mime_type.contains("mp3") {
        "mp3"
    } else if mime_type.contains("mp4") || mime_type.contains("m4a") {
        "m4a"
    } else {
        "webm"
    }
}

fn preview(body: &str) -> String {
    let mut output: String = body.chars().take(240).collect();
    if body.chars().count() > 240 {
        output.push('…');
    }
    output.replace(['\r', '\n'], " ")
}
