use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::jpeg::JpegEncoder, ColorType};
#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashSet,
    fs,
    sync::{Mutex, RwLock},
    time::Duration,
};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use xcap::Monitor;

mod audio;
mod storage;
#[cfg(target_os = "windows")]
mod system_audio;
#[cfg(target_os = "macos")]
#[path = "system_audio_macos.rs"]
mod system_audio;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[path = "system_audio_unsupported.rs"]
mod system_audio;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AppSettings {
    base_url: String,
    api_key: String,
    model: String,
    vision_model: String,
    transcription_model: String,
    capture_microphone: bool,
    capture_system_audio: bool,
    microphone_device_id: String,
    system_audio_device_id: String,
    my_transcription_language: String,
    their_transcription_language: String,
    auto_safe_cleanup: bool,
    fixed_context: String,
    // Kept for one-way migration from settings written by version 0.2.0.
    transcription_language: String,
    system_prompt: String,
    coding_prompt: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
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
            system_prompt: r#"你是面试实时 Copilot。请结合固定背景、近期对话、本轮输入、截图和历史回答，以面试者的第一人称生成可直接说出口的中文回答。

要求：

- 先直接回答核心问题，再补充最必要的依据、例子或权衡。
- 表达自然、简洁、自信，避免模板化开场、重复题目和冗长铺垫。
- 技术问题优先说明结论、核心原理、实际应用和关键取舍。
- 经历类问题基于已有背景组织为“场景—行动—结果”，不要编造不存在的经历或数据。
- 信息不足时做最小且明确的假设；如果缺失信息会显著影响答案，给出一句简短的澄清问题。
- 严格遵循本轮输入中的具体格式要求；需要代码、步骤或表格时可以使用结构化输出。
- 默认保持精炼，只有问题本身复杂或对方明确要求深入时才展开。"#
                .into(),
            coding_prompt: r#"你是算法面试实时助手。请根据截图还原题目、输入输出、约束和函数签名，并使用 Python 解决。

按以下顺序回答：

1. 题意确认：用一句话概括问题；如果截图信息不完整，明确最小必要假设，不要编造条件。
2. 面试口述：用简洁、自然、可直接说给面试官听的方式说明核心思路、关键数据结构和正确性依据。
3. Python 代码：遵循截图中的函数签名，给出可直接提交的完整实现。代码保持清晰简洁，只在关键逻辑处添加注释。
4. 复杂度：明确时间复杂度和空间复杂度。
5. 边界情况：列出最重要的边界条件，并给出一个简短示例或执行过程用于检查代码。

默认直接提供最优且易于解释的方案。只有当朴素方案有助于推导最优解时，才用一至两句话说明朴素思路及其性能瓶颈，不要为非最优方案重复编写完整代码。

避免冗长背景、重复解释和与解题无关的内容。如果存在多种同等复杂度的方案，优先选择面试中最容易讲清楚、最不容易写错的一种。"#
                .into(),
        }
    }
}

struct AppState {
    settings: RwLock<AppSettings>,
    storage: storage::StorageManager,
    shortcut_warnings: RwLock<Vec<String>>,
    system_audio: Mutex<Option<system_audio::SystemAudioRecorder>>,
    region_capture: Mutex<Option<RegionCaptureSession>>,
    cancelled_requests: Mutex<HashSet<String>>,
}

struct CancellationGuard<'a> {
    requests: &'a Mutex<HashSet<String>>,
    request_id: String,
}

impl<'a> CancellationGuard<'a> {
    fn new(requests: &'a Mutex<HashSet<String>>, request_id: String) -> Result<Self, String> {
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

#[derive(Debug, Clone, Copy)]
struct RegionCaptureSession {
    restore_main_window: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegionSelection {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskRequest {
    request_id: String,
    prompt: String,
    #[serde(default)]
    image_data_urls: Vec<String>,
    #[serde(default)]
    history: Vec<ConversationMessage>,
}

#[derive(Debug, Clone, Deserialize)]
struct ConversationMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnswerDelta {
    request_id: String,
    delta: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AskResult {
    text: String,
    cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureResult {
    data_url: String,
}

#[tauri::command]
fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state
        .settings
        .read()
        .map(|settings| settings.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    let settings_path = state.storage.settings_path()?;
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let serialized = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(settings_path, serialized).map_err(|error| error.to_string())?;
    *state.settings.write().map_err(|error| error.to_string())? = settings;
    Ok(())
}

#[tauri::command]
fn shortcut_warnings(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state
        .shortcut_warnings
        .read()
        .map(|warnings| warnings.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn storage_info(state: State<'_, AppState>) -> Result<storage::StorageInfo, String> {
    state.storage.info()
}

#[tauri::command]
fn set_storage_root(
    path: String,
    state: State<'_, AppState>,
) -> Result<storage::StorageInfo, String> {
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    let serialized = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    state
        .storage
        .configure_root(std::path::Path::new(path.trim()), &serialized)
}

#[tauri::command]
fn schedule_safe_cleanup(state: State<'_, AppState>) -> Result<storage::StorageInfo, String> {
    state.storage.schedule_cleanup()
}

fn encode_capture(image: image::RgbaImage) -> Result<CaptureResult, String> {
    let rgb = image::DynamicImage::ImageRgba8(image).to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 82)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ColorType::Rgb8.into(),
        )
        .map_err(|error| error.to_string())?;
    Ok(CaptureResult {
        data_url: format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg)),
    })
}

fn cursor_position() -> Result<(i32, i32), String> {
    use mouse_position::mouse_position::Mouse;

    match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => Ok((x, y)),
        Mouse::Error => Err("读取鼠标位置失败".into()),
    }
}

fn capture_absolute_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<CaptureResult, String> {
    if width < 2 || height < 2 {
        return Err("截图区域太小".into());
    }
    let monitor = Monitor::from_point(x, y).map_err(|error| error.to_string())?;
    let monitor_x = monitor.x().map_err(|error| error.to_string())?;
    let monitor_y = monitor.y().map_err(|error| error.to_string())?;
    let full = monitor.capture_image().map_err(|error| error.to_string())?;
    let max_x = full.width() as i32;
    let max_y = full.height() as i32;
    let left = (x - monitor_x).clamp(0, max_x);
    let top = (y - monitor_y).clamp(0, max_y);
    let right = (x.saturating_add(width as i32) - monitor_x).clamp(0, max_x);
    let bottom = (y.saturating_add(height as i32) - monitor_y).clamp(0, max_y);
    let cropped_width = left.abs_diff(right);
    let cropped_height = top.abs_diff(bottom);
    if cropped_width < 2 || cropped_height < 2 {
        return Err("截图区域超出当前显示器或尺寸过小".into());
    }
    let cropped = image::imageops::crop_imm(
        &full,
        left.min(right) as u32,
        top.min(bottom) as u32,
        cropped_width,
        cropped_height,
    )
    .to_image();
    encode_capture(cropped)
}

fn take_region_session(state: &AppState) -> Option<RegionCaptureSession> {
    state
        .region_capture
        .lock()
        .ok()
        .and_then(|mut session| session.take())
}

fn restore_main_after_region(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some(session) = take_region_session(&state) else {
        return;
    };
    if session.restore_main_window {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    let _ = app.emit_to("main", "region-capture-cancelled", ());
}

#[tauri::command]
async fn open_region_selector(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if app.get_webview_window("region-selector").is_some() {
        return Err("区域截图选择器已经打开".into());
    }

    let cursor = cursor_position()?;
    let monitor = Monitor::from_point(cursor.0, cursor.1).map_err(|error| error.to_string())?;
    let monitor_x = monitor.x().map_err(|error| error.to_string())?;
    let monitor_y = monitor.y().map_err(|error| error.to_string())?;
    let monitor_width = monitor.width().map_err(|error| error.to_string())?;
    let monitor_height = monitor.height().map_err(|error| error.to_string())?;
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "没有找到主窗口".to_string())?;
    let restore_main_window = main.is_visible().unwrap_or(false);
    if restore_main_window {
        main.hide().map_err(|error| error.to_string())?;
    }
    *state
        .region_capture
        .lock()
        .map_err(|error| error.to_string())? = Some(RegionCaptureSession {
        restore_main_window,
    });

    let selector_builder = tauri::WebviewWindowBuilder::new(
        &app,
        "region-selector",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("选择截图区域")
    .inner_size(640.0, 480.0)
    .visible(false)
    .focused(true)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .content_protected(true);
    #[cfg(target_os = "windows")]
    let selector_builder = selector_builder.data_directory(state.storage.active_webview_path());
    let selector = match selector_builder.build() {
        Ok(selector) => selector,
        Err(error) => {
            restore_main_after_region(&app);
            return Err(format!("无法创建区域截图选择器：{error}"));
        }
    };

    let configured = selector
        .set_position(tauri::PhysicalPosition::new(monitor_x, monitor_y))
        .and_then(|_| selector.set_size(tauri::PhysicalSize::new(monitor_width, monitor_height)))
        .and_then(|_| selector.show())
        .and_then(|_| selector.set_focus());
    if let Err(error) = configured {
        let _ = selector.close();
        restore_main_after_region(&app);
        return Err(format!("无法显示区域截图选择器：{error}"));
    }

    #[cfg(target_os = "windows")]
    match query_display_affinity(&selector) {
        Ok(affinity) => eprintln!("Region selector display affinity: 0x{affinity:X}"),
        Err(error) => eprintln!("Region selector display affinity check failed: {error}"),
    }
    Ok(())
}

#[tauri::command]
async fn complete_region_selection(
    selection: RegionSelection,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !selection.x.is_finite()
        || !selection.y.is_finite()
        || !selection.width.is_finite()
        || !selection.height.is_finite()
        || selection.width < 2.0
        || selection.height < 2.0
    {
        return Err("截图区域无效或尺寸过小".into());
    }
    let selector = app
        .get_webview_window("region-selector")
        .ok_or_else(|| "区域截图选择器已经关闭".to_string())?;
    let position = selector
        .outer_position()
        .map_err(|error| error.to_string())?;
    let scale = selector.scale_factor().map_err(|error| error.to_string())?;
    let x = position
        .x
        .saturating_add((selection.x * scale).round() as i32);
    let y = position
        .y
        .saturating_add((selection.y * scale).round() as i32);
    let right = position
        .x
        .saturating_add(((selection.x + selection.width) * scale).round() as i32);
    let bottom = position
        .y
        .saturating_add(((selection.y + selection.height) * scale).round() as i32);
    let width = x.abs_diff(right);
    let height = y.abs_diff(bottom);
    let session = take_region_session(&state).ok_or_else(|| "区域截图会话已经结束".to_string())?;

    let _ = selector.hide();
    let capture = match tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(180));
        capture_absolute_region(x.min(right), y.min(bottom), width, height)
    })
    .await
    {
        Ok(result) => result,
        Err(error) => Err(format!("区域截图任务失败：{error}")),
    };
    let _ = selector.close();
    if session.restore_main_window {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    match capture {
        Ok(result) => app
            .emit_to("main", "region-captured", result)
            .map_err(|error| error.to_string()),
        Err(error) => {
            let _ = app.emit_to("main", "region-capture-error", error.clone());
            Err(error)
        }
    }
}

#[tauri::command]
fn cancel_region_selection(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session = take_region_session(&state);
    let close_result = app
        .get_webview_window("region-selector")
        .map(|selector| selector.close().map_err(|error| error.to_string()))
        .transpose();
    if session.is_some_and(|item| item.restore_main_window) {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
    let emit_result = app
        .emit_to("main", "region-capture-cancelled", ())
        .map_err(|error| error.to_string());
    close_result?;
    emit_result
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
async fn ask_llm(
    request: AskRequest,
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<AskResult, String> {
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    ensure_key(&settings)?;

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
    let mut messages = vec![json!({"role": "system", "content": settings.system_prompt})];
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
        .bearer_auth(&settings.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(response_error(response).await);
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
                .bearer_auth(&settings.api_key)
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
                return Err(response_error(fallback).await);
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
fn cancel_llm(request_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .cancelled_requests
        .lock()
        .map_err(|error| error.to_string())?
        .insert(request_id);
    Ok(())
}

fn bounded_history(history: &[ConversationMessage]) -> Vec<serde_json::Value> {
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

fn request_cancelled(state: &State<'_, AppState>, request_id: &str) -> Result<bool, String> {
    state
        .cancelled_requests
        .lock()
        .map(|requests| requests.contains(request_id))
        .map_err(|error| error.to_string())
}

async fn read_answer_stream(
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

fn parse_sse_delta(data: &str) -> Result<Option<String>, String> {
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

#[tauri::command]
async fn transcribe_audio(
    bytes: Vec<u8>,
    mime_type: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    transcribe_bytes(
        &settings,
        bytes,
        mime_type,
        &settings.my_transcription_language,
    )
    .await
}

#[tauri::command]
fn start_system_audio(state: State<'_, AppState>) -> Result<(), String> {
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
    *current = Some(system_audio::SystemAudioRecorder::start(
        (!device_id.is_empty()).then_some(device_id),
    )?);
    Ok(())
}

#[tauri::command]
fn list_system_audio_devices() -> Result<Vec<system_audio::AudioOutputDevice>, String> {
    system_audio::list_output_devices()
}

#[tauri::command]
fn system_audio_level(state: State<'_, AppState>) -> Result<f32, String> {
    state
        .system_audio
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
        .ok_or_else(|| "系统音频尚未开始录制".to_string())?
        .activity_level()
}

#[tauri::command]
fn discard_system_audio_chunk(state: State<'_, AppState>) -> Result<(), String> {
    state
        .system_audio
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
        .ok_or_else(|| "系统音频尚未开始录制".to_string())?
        .clear_chunk()
}

#[tauri::command]
async fn stop_system_audio_and_transcribe(state: State<'_, AppState>) -> Result<String, String> {
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
    transcribe_bytes(
        &settings,
        wav,
        "audio/wav".into(),
        &settings.their_transcription_language,
    )
    .await
}

#[tauri::command]
async fn transcribe_system_audio_chunk(state: State<'_, AppState>) -> Result<String, String> {
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
    transcribe_bytes(
        &settings,
        wav,
        "audio/wav".into(),
        &settings.their_transcription_language,
    )
    .await
}

async fn transcribe_bytes(
    settings: &AppSettings,
    bytes: Vec<u8>,
    mime_type: String,
    language: &str,
) -> Result<String, String> {
    ensure_key(settings)?;
    if bytes.is_empty() {
        return Err("录音为空".into());
    }
    if is_dashscope(&settings.base_url) {
        return transcribe_dashscope(settings, bytes, &mime_type, language).await;
    }
    let extension = audio_extension(&mime_type);
    let part = multipart::Part::bytes(bytes)
        .file_name(format!("speech.{extension}"))
        .mime_str(&mime_type)
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
        .bearer_auth(&settings.api_key)
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

async fn transcribe_dashscope(
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
        .bearer_auth(&settings.api_key)
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

fn is_dashscope(base_url: &str) -> bool {
    base_url.contains("dashscope.aliyuncs.com")
        || (base_url.contains("maas.aliyuncs.com") && base_url.contains("compatible-mode"))
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

fn ensure_key(settings: &AppSettings) -> Result<(), String> {
    if settings.api_key.trim().is_empty() {
        Err("请先配置 API Key".into())
    } else {
        Ok(())
    }
}

pub(crate) fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn handle_shortcut_action(app: &tauri::AppHandle, action: &'static str) {
    match action {
        "toggle-window" => toggle_main_window(app),
        "quit" => app.exit(0),
        _ => {
            let _ = app.emit("shortcut-action", action);
        }
    }
}

#[cfg(target_os = "windows")]
fn query_display_affinity(window: &tauri::WebviewWindow) -> Result<u32, String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowDisplayAffinity;
    let handle = window.window_handle().map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return Err("当前窗口不是 Win32 窗口".into());
    };
    let hwnd = HWND(win32.hwnd.get() as *mut std::ffi::c_void);
    let mut affinity = 0u32;
    unsafe { GetWindowDisplayAffinity(hwnd, &mut affinity) }
        .map_err(|error| format!("读取窗口捕获保护失败：{error}"))?;
    Ok(affinity)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut_plugin = tauri_plugin_global_shortcut::Builder::new().build();
    tauri::Builder::default()
        .plugin(shortcut_plugin)
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if window.label() == "region-selector" && matches!(event, tauri::WindowEvent::Destroyed)
            {
                restore_main_after_region(window.app_handle());
            }
        })
        .setup(|app| {
            let old_settings_path = app.path().app_config_dir()?.join("settings.json");
            let old_webview_path = app.path().app_local_data_dir()?.join("EBWebView");
            let storage =
                storage::StorageManager::initialize(&old_settings_path, &old_webview_path)?;
            let settings_path = storage.settings_path()?;
            let legacy_settings_path = app
                .path()
                .config_dir()?
                .join("local.desktop-copilot")
                .join("settings.json");
            let source_path = if settings_path.exists() {
                &settings_path
            } else {
                &legacy_settings_path
            };
            let source_content = fs::read_to_string(source_path).ok();
            let needs_storage_schema = source_content
                .as_deref()
                .is_some_and(|content| !content.contains("\"autoSafeCleanup\""));
            let mut settings: AppSettings = source_content
                .as_deref()
                .and_then(|content| serde_json::from_str(content).ok())
                .unwrap_or_default();
            let mut persist_settings = source_path != &settings_path || needs_storage_schema;
            if settings.transcription_language != "auto"
                && settings.my_transcription_language == "auto"
                && settings.their_transcription_language == "auto"
            {
                settings.my_transcription_language = settings.transcription_language.clone();
                settings.their_transcription_language = settings.transcription_language.clone();
                settings.transcription_language = "auto".into();
                persist_settings = true;
            }
            if is_dashscope(&settings.base_url)
                && (!settings.transcription_model.starts_with("qwen3-asr-flash")
                    || settings.transcription_model.contains("realtime")
                    || settings.transcription_model.contains("filetrans"))
            {
                settings.transcription_model = "qwen3-asr-flash".into();
                persist_settings = true;
            }
            if persist_settings {
                if let Some(parent) = settings_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(serialized) = serde_json::to_string_pretty(&settings) {
                    let _ = fs::write(&settings_path, serialized);
                }
                if source_path != &settings_path && source_path.exists() {
                    let _ = fs::remove_file(source_path);
                }
            }
            if let Err(error) = storage.run_startup_cleanup(settings.auto_safe_cleanup) {
                eprintln!("Interview Buddy safe cleanup deferred after error: {error}");
            }
            let webview_data_path = storage.active_webview_path();
            app.manage(AppState {
                settings: RwLock::new(settings),
                storage,
                shortcut_warnings: RwLock::new(Vec::new()),
                system_audio: Mutex::new(None),
                region_capture: Mutex::new(None),
                cancelled_requests: Mutex::new(HashSet::new()),
            });
            let main_config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or_else(|| "缺少 main 窗口配置".to_string())?;
            let main_builder =
                tauri::WebviewWindowBuilder::from_config(app.handle(), &main_config)?;
            #[cfg(target_os = "windows")]
            let main_builder = main_builder.data_directory(webview_data_path);
            let window = main_builder.build()?;
            window.set_content_protected(true)?;
            window.set_always_on_top(true)?;
            window.show()?;
            #[cfg(target_os = "windows")]
            match query_display_affinity(&window) {
                Ok(affinity) => eprintln!("Interview Buddy display affinity: 0x{affinity:X}"),
                Err(error) => {
                    eprintln!("Interview Buddy display affinity check failed: {error}")
                }
            }
            if let Some(icon) = app.default_window_icon().cloned() {
                TrayIconBuilder::with_id("interview-buddy-tray")
                    .icon(icon)
                    .tooltip("Interview Buddy — 点击显示或隐藏")
                    .show_menu_on_left_click(false)
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            toggle_main_window(tray.app_handle());
                        }
                    })
                    .build(app)?;
            }
            let shortcuts = [
                ("CommandOrControl+Shift+C", "clear"),
                ("CommandOrControl+Shift+S", "capture-region"),
                ("CommandOrControl+Shift+L", "listening-toggle"),
                ("CommandOrControl+Shift+A", "answer-toggle"),
                ("CommandOrControl+Shift+I", "send"),
                ("CommandOrControl+Shift+Space", "toggle-window"),
                ("CommandOrControl+Q", "quit"),
            ];
            for (shortcut, action) in shortcuts {
                if let Err(error) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, move |app, _shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                handle_shortcut_action(app, action);
                            }
                        })
                {
                    let warning = format!("{shortcut}：{error}");
                    eprintln!("Interview Buddy shortcut unavailable: {warning}");
                    app.state::<AppState>()
                        .shortcut_warnings
                        .write()
                        .map_err(|error| error.to_string())?
                        .push(warning);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            shortcut_warnings,
            storage_info,
            set_storage_root,
            schedule_safe_cleanup,
            open_region_selector,
            complete_region_selection,
            cancel_region_selection,
            quit_app,
            ask_llm,
            cancel_llm,
            transcribe_audio,
            list_system_audio_devices,
            start_system_audio,
            system_audio_level,
            discard_system_audio_chunk,
            stop_system_audio_and_transcribe,
            transcribe_system_audio_chunk
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Interview Buddy");
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

    #[test]
    fn history_keeps_only_supported_roles_in_order() {
        let history = vec![
            ConversationMessage {
                role: "system".into(),
                content: "ignore".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "question".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "answer".into(),
            },
        ];
        let bounded = bounded_history(&history);
        assert_eq!(bounded.len(), 2);
        assert_eq!(bounded[0]["role"], "user");
        assert_eq!(bounded[1]["role"], "assistant");
    }

    #[test]
    fn cancellation_guard_clears_stale_and_completed_requests() {
        let requests = Mutex::new(HashSet::from(["request".to_string()]));
        {
            let _guard = CancellationGuard::new(&requests, "request".into()).expect("create guard");
            assert!(!requests.lock().expect("requests").contains("request"));
            requests.lock().expect("requests").insert("request".into());
        }
        assert!(!requests.lock().expect("requests").contains("request"));
    }
}
