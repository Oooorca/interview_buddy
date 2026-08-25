use serde::Serialize;
use tauri::{Emitter, State};

use crate::app_state::AppState;

pub const MAX_STREAM_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnswerDelta {
    request_id: String,
    delta: String,
}

pub(super) fn request_cancelled(
    state: &State<'_, AppState>,
    request_id: &str,
) -> Result<bool, String> {
    state
        .cancelled_requests
        .lock()
        .map(|requests| requests.contains(request_id))
        .map_err(|error| error.to_string())
}

pub(super) async fn read_answer_stream(
    mut response: reqwest::Response,
    request_id: &str,
    window: &tauri::WebviewWindow,
    state: &State<'_, AppState>,
) -> Result<(String, bool), String> {
    let mut pending = Vec::<u8>::new();
    let mut full_text = String::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("读取流式回答失败：{error}"))?
    {
        if request_cancelled(state, request_id)? {
            return Ok((full_text, true));
        }
        pending.extend_from_slice(&chunk);
        if pending.len() > MAX_STREAM_LINE_BYTES {
            return Err("LLM 流式响应单行超过 1 MiB 安全上限".into());
        }
        while let Some(index) = pending.iter().position(|byte| *byte == b'\n') {
            let line = pending.drain(..=index).collect::<Vec<_>>();
            process_sse_line(&line, request_id, window, &mut full_text)?;
        }
    }
    if !pending.is_empty() {
        process_sse_line(&pending, request_id, window, &mut full_text)?;
    }
    if request_cancelled(state, request_id)? {
        return Ok((full_text, true));
    }
    if full_text.trim().is_empty() {
        return Err("LLM 流式响应中没有文本内容".into());
    }
    Ok((full_text, false))
}

fn process_sse_line(
    bytes: &[u8],
    request_id: &str,
    window: &tauri::WebviewWindow,
    full_text: &mut String,
) -> Result<(), String> {
    let line = std::str::from_utf8(bytes)
        .map_err(|error| format!("LLM 流包含无效 UTF-8：{error}"))?
        .trim();
    let Some(data) = line.strip_prefix("data:").map(str::trim) else {
        return Ok(());
    };
    if data.is_empty() || data == "[DONE]" {
        return Ok(());
    }
    if let Some(delta) = parse_sse_delta(data)? {
        full_text.push_str(&delta);
        window
            .emit(
                "answer-stream-delta",
                AnswerDelta {
                    request_id: request_id.to_string(),
                    delta,
                },
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn parse_sse_delta(data: &str) -> Result<Option<String>, String> {
    let value: serde_json::Value = serde_json::from_str(data)
        .map_err(|error| format!("无法解析流式回答：{error}；正文：{}", preview(data)))?;
    if let Some(message) = value
        .pointer("/error/message")
        .and_then(|item| item.as_str())
    {
        return Err(message.to_string());
    }
    Ok(value
        .pointer("/choices/0/delta/content")
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned))
}

fn preview(body: &str) -> String {
    let mut output: String = body.chars().take(240).collect();
    if body.chars().count() > 240 {
        output.push('…');
    }
    output.replace(['\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_compatible_sse_delta_is_extracted() {
        let delta =
            parse_sse_delta(r#"{"choices":[{"delta":{"content":"你好"},"finish_reason":null}]}"#)
                .expect("valid chunk");
        assert_eq!(delta.as_deref(), Some("你好"));
        assert_eq!(
            parse_sse_delta(r#"{"choices":[{"delta":{}}]}"#).unwrap(),
            None
        );
    }
}
