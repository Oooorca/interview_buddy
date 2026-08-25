use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashSet, sync::Mutex, time::Duration};
use tauri::State;

use super::stream::{read_answer_stream, request_cancelled};
use crate::{
    app_state::AppState,
    error::AppResult,
    settings::{commands::ensure_security_ready, migration::resolved_system_prompt},
};

pub(crate) struct CancellationGuard<'a> {
    requests: &'a Mutex<HashSet<String>>,
    request_id: String,
}

impl<'a> CancellationGuard<'a> {
    pub(crate) fn new(
        requests: &'a Mutex<HashSet<String>>,
        request_id: String,
    ) -> Result<Self, String> {
        requests
            .lock()
            .map_err(|error| error.to_string())?
            .remove(&request_id);
        Ok(Self {
            requests,
            request_id,
        })
    }
}

impl Drop for CancellationGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut requests) = self.requests.lock() {
            requests.remove(&self.request_id);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AskRequest {
    request_id: String,
    prompt: String,
    #[serde(default)]
    image_data_urls: Vec<String>,
    #[serde(default)]
    history: Vec<ConversationMessage>,
    #[serde(default = "default_answer_locale")]
    answer_locale: String,
}

fn default_answer_locale() -> String {
    "zh-CN".into()
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ConversationMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AskResult {
    text: String,
    cancelled: bool,
}

#[tauri::command]
pub(crate) async fn ask_llm(
    request: AskRequest,
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<AskResult> {
    ensure_security_ready(&state)?;
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    if settings.api_key.is_empty() {
        return Err("请先配置 API Key".into());
    }

    let request_id = request.request_id.clone();
    let _cancellation_guard =
        CancellationGuard::new(&state.cancelled_requests, request_id.clone())?;
    let (model, user_content) = if request.image_data_urls.is_empty() {
        (settings.model.clone(), json!(request.prompt))
    } else {
        let mut content = vec![json!({"type": "text", "text": request.prompt})];
        content.extend(
            request
                .image_data_urls
                .into_iter()
                .map(|image| json!({"type": "image_url", "image_url": {"url": image}})),
        );
        (settings.vision_model.clone(), json!(content))
    };
    let mut messages = Vec::new();
    let answer_locale = if request.answer_locale == "en-US" {
        "en-US"
    } else {
        "zh-CN"
    };
    if let Some(system_prompt) = resolved_system_prompt(&settings, answer_locale) {
        messages.push(json!({"role": "system", "content": system_prompt}));
    }
    messages.extend(bounded_history(&request.history));
    messages.push(json!({"role": "user", "content": user_content}));
    let body = json!({
        "model": model,
        "messages": messages,
        "temperature": 0.25,
        "stream": true
    });
    let url = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .post(&url)
        .bearer_auth(settings.api_key.expose())
        .json(&body)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(response_error(response).await.into());
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !content_type.contains("text/event-stream") {
        let body = response.text().await.map_err(|error| error.to_string())?;
        if request_cancelled(&state, &request_id)? {
            return Ok(AskResult {
                text: String::new(),
                cancelled: true,
            });
        }
        let text = parse_chat_response(&body)?;
        return Ok(AskResult {
            text,
            cancelled: false,
        });
    }

    match read_answer_stream(response, &request_id, &window, &state).await {
        Ok((text, cancelled)) => Ok(AskResult { text, cancelled }),
        Err(stream_error) => {
            if request_cancelled(&state, &request_id)? {
                return Ok(AskResult {
                    text: String::new(),
                    cancelled: true,
                });
            }
            let mut fallback_body = body;
            fallback_body["stream"] = json!(false);
            let fallback = client
                .post(&url)
                .bearer_auth(settings.api_key.expose())
                .json(&fallback_body)
                .send()
                .await
                .map_err(|error| {
                    format!("流式回答失败（{stream_error}）；普通请求也失败：{error}")
                })?;
            if request_cancelled(&state, &request_id)? {
                return Ok(AskResult {
                    text: String::new(),
                    cancelled: true,
                });
            }
            if !fallback.status().is_success() {
                return Err(response_error(fallback).await.into());
            }
            let fallback_text = fallback
                .text()
                .await
                .map_err(|error| format!("读取普通回答失败：{error}"))?;
            if request_cancelled(&state, &request_id)? {
                return Ok(AskResult {
                    text: String::new(),
                    cancelled: true,
                });
            }
            let text = parse_chat_response(&fallback_text)?;
            Ok(AskResult {
                text,
                cancelled: false,
            })
        }
    }
}

#[tauri::command]
pub(crate) fn cancel_llm(request_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state
        .cancelled_requests
        .lock()
        .map_err(|error| error.to_string())?
        .insert(request_id);
    Ok(())
}

pub(crate) fn bounded_history(history: &[ConversationMessage]) -> Vec<serde_json::Value> {
    const MAX_MESSAGES: usize = 16;
    const MAX_CHARS: usize = 18_000;
    let mut retained = Vec::new();
    let mut chars = 0usize;
    for message in history.iter().rev() {
        if retained.len() >= MAX_MESSAGES {
            break;
        }
        let role = match message.role.as_str() {
            "user" => "user",
            "assistant" | "model" => "assistant",
            _ => continue,
        };
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }
        let length = content.chars().count();
        if !retained.is_empty() && chars + length > MAX_CHARS {
            break;
        }
        chars += length;
        retained.push(json!({"role": role, "content": content}));
    }
    retained.reverse();
    retained
}

fn parse_chat_response(body: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("LLM 返回了无法解析的响应：{error}；正文：{}", preview(body)))?;
    value
        .pointer("/choices/0/message/content")
        .and_then(|item| item.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            value
                .pointer("/error/message")
                .and_then(|item| item.as_str())
                .unwrap_or("LLM 响应中没有文本内容")
                .to_string()
        })
}

async fn response_error(response: reqwest::Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(|item| item.as_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| format!("LLM 服务返回 {status}：{}", preview(&body)))
}

fn preview(body: &str) -> String {
    let mut output: String = body.chars().take(240).collect();
    if body.chars().count() > 240 {
        output.push('…');
    }
    output.replace(['\r', '\n'], " ")
}
