use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use std::time::Duration;

use crate::settings::AppSettings;

pub(super) async fn transcribe(
    settings: &AppSettings,
    bytes: Vec<u8>,
    mime_type: &str,
    language: &str,
) -> Result<String, String> {
    if bytes.len() > 10 * 1024 * 1024 {
        return Err("百炼 qwen3-asr-flash 单次音频不能超过 10MB".into());
    }
    let model = if settings.transcription_model.starts_with("qwen3-asr-flash")
        && !settings.transcription_model.contains("realtime")
        && !settings.transcription_model.contains("filetrans")
    {
        settings.transcription_model.as_str()
    } else {
        "qwen3-asr-flash"
    };
    let data_url = format!("data:{mime_type};base64,{}", STANDARD.encode(bytes));
    let mut asr_options = json!({ "enable_itn": true });
    let language = language.trim();
    if !language.is_empty() && language != "auto" {
        asr_options["language"] = json!(language);
    }
    let body = json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "input_audio",
                "input_audio": { "data": data_url }
            }]
        }],
        "stream": false,
        "asr_options": asr_options
    });
    let url = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?
        .post(url)
        .bearer_auth(settings.api_key.expose())
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("连接百炼转写服务失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("读取百炼转写响应失败：{error}"))?;
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("百炼返回无法解析的响应：{error}；正文：{}", preview(&body)))?;
    if !status.is_success() {
        return Err(value
            .pointer("/error/message")
            .and_then(|item| item.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("百炼转写返回 HTTP {status}：{}", preview(&body))));
    }
    value
        .pointer("/choices/0/message/content")
        .and_then(|item| item.as_str())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("百炼转写响应中没有文本：{}", preview(&body)))
}

pub(crate) fn is_dashscope(base_url: &str) -> bool {
    base_url.contains("dashscope.aliyuncs.com")
        || (base_url.contains("maas.aliyuncs.com") && base_url.contains("compatible-mode"))
}

fn preview(body: &str) -> String {
    let mut output: String = body.chars().take(240).collect();
    if body.chars().count() > 240 {
        output.push('…');
    }
    output.replace(['\r', '\n'], " ")
}
