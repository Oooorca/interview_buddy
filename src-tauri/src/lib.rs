use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::jpeg::JpegEncoder, ColorType};
#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock, RwLock},
    time::{Duration, Instant},
};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use xcap::Monitor;

mod audio;
#[cfg(target_os = "windows")]
mod system_audio;
#[cfg(target_os = "macos")]
#[path = "system_audio_macos.rs"]
mod system_audio;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[path = "system_audio_unsupported.rs"]
mod system_audio;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    base_url: String,
    api_key: String,
    model: String,
    vision_model: String,
    transcription_model: String,
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
            system_prompt: "你是会议实时 Copilot。根据上下文给出自然、简短、可直接说出口的中文回答；必要时补充要点和反问。不要编造事实。".into(),
            coding_prompt: "你是算法面试助手。识别截图题目，给出核心思路、复杂度、可提交代码和边界情况。".into(),
        }
    }
}

struct AppState {
    settings: RwLock<AppSettings>,
    settings_path: PathBuf,
    system_audio: Mutex<Option<system_audio::SystemAudioRecorder>>,
    capture_origin: Mutex<Option<(i32, i32)>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskRequest {
    prompt: String,
    #[serde(default)]
    image_data_urls: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureResult {
    data_url: String,
    width: u32,
    height: u32,
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
    if let Some(parent) = state.settings_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let serialized = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(&state.settings_path, serialized).map_err(|error| error.to_string())?;
    *state.settings.write().map_err(|error| error.to_string())? = settings;
    Ok(())
}

#[tauri::command]
fn capture_primary_monitor() -> Result<CaptureResult, String> {
    let monitor = Monitor::all()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| "没有找到主显示器".to_string())?;
    let image = monitor.capture_image().map_err(|error| error.to_string())?;
    encode_capture(image)
}

fn encode_capture(image: image::RgbaImage) -> Result<CaptureResult, String> {
    let width = image.width();
    let height = image.height();
    let rgb = image::DynamicImage::ImageRgba8(image).to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 82)
        .encode(rgb.as_raw(), width, height, ColorType::Rgb8.into())
        .map_err(|error| error.to_string())?;
    Ok(CaptureResult {
        data_url: format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg)),
        width,
        height,
    })
}

fn cursor_position() -> Result<(i32, i32), String> {
    use mouse_position::mouse_position::Mouse;

    match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => Ok((x, y)),
        Mouse::Error => Err("读取鼠标位置失败".into()),
    }
}

#[tauri::command]
fn mark_capture_origin(state: State<'_, AppState>) -> Result<String, String> {
    let point = cursor_position()?;
    *state
        .capture_origin
        .lock()
        .map_err(|error| error.to_string())? = Some(point);
    Ok(format!("已标记截图左上角：{}, {}", point.0, point.1))
}

#[tauri::command]
fn capture_marked_region(state: State<'_, AppState>) -> Result<CaptureResult, String> {
    let start = *state
        .capture_origin
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
        .ok_or_else(|| "请先按 Ctrl+Shift+1 标记左上角".to_string())?;
    let end = cursor_position()?;
    let monitor = Monitor::from_point(start.0, start.1).map_err(|error| error.to_string())?;
    let monitor_x = monitor.x().map_err(|error| error.to_string())?;
    let monitor_y = monitor.y().map_err(|error| error.to_string())?;
    // Capture first and crop locally. xcap's region API can otherwise mix DPI
    // coordinate spaces on scaled Windows displays.
    let full = monitor.capture_image().map_err(|error| error.to_string())?;
    let max_x = full.width() as i32;
    let max_y = full.height() as i32;
    let start_x = (start.0 - monitor_x).clamp(0, max_x);
    let start_y = (start.1 - monitor_y).clamp(0, max_y);
    let end_x = (end.0 - monitor_x).clamp(0, max_x);
    let end_y = (end.1 - monitor_y).clamp(0, max_y);
    let left = start_x.min(end_x) as u32;
    let top = start_y.min(end_y) as u32;
    let width = start_x.abs_diff(end_x);
    let height = start_y.abs_diff(end_y);
    if width < 2 || height < 2 {
        return Err(format!(
            "截图区域太小：起点({}, {})，终点({}, {})",
            start.0, start.1, end.0, end.1
        ));
    }
    let cropped = image::imageops::crop_imm(&full, left, top, width, height).to_image();
    *state
        .capture_origin
        .lock()
        .map_err(|error| error.to_string())? = None;
    encode_capture(cropped)
}

#[tauri::command]
async fn ask_llm(request: AskRequest, state: State<'_, AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    ensure_key(&settings)?;

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
    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": settings.system_prompt},
            {"role": "user", "content": user_content}
        ],
        "temperature": 0.25
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
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("读取 LLM 响应失败：{error}"))?;
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|error| {
        format!(
            "LLM 返回了无法解析的响应：{error}；正文：{}",
            preview(&body)
        )
    })?;
    if !status.is_success() {
        return Err(value
            .pointer("/error/message")
            .and_then(|item| item.as_str())
            .unwrap_or("LLM 服务返回错误")
            .to_string());
    }
    value
        .pointer("/choices/0/message/content")
        .and_then(|item| item.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "LLM 响应中没有文本内容".to_string())
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
    transcribe_bytes(&settings, bytes, mime_type).await
}

#[tauri::command]
fn start_system_audio(state: State<'_, AppState>) -> Result<(), String> {
    let mut current = state
        .system_audio
        .lock()
        .map_err(|error| error.to_string())?;
    if current.is_some() {
        return Err("系统音频已经在录制".into());
    }
    *current = Some(system_audio::SystemAudioRecorder::start()?);
    Ok(())
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
    let settings = state
        .settings
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    transcribe_bytes(&settings, wav, "audio/wav".into()).await
}

async fn transcribe_bytes(
    settings: &AppSettings,
    bytes: Vec<u8>,
    mime_type: String,
) -> Result<String, String> {
    ensure_key(settings)?;
    if bytes.is_empty() {
        return Err("录音为空".into());
    }
    if is_dashscope(&settings.base_url) {
        return transcribe_dashscope(settings, bytes, &mime_type).await;
    }
    let extension = audio_extension(&mime_type);
    let part = multipart::Part::bytes(bytes)
        .file_name(format!("speech.{extension}"))
        .mime_str(&mime_type)
        .map_err(|error| error.to_string())?;
    let form = multipart::Form::new()
        .text("model", settings.transcription_model.clone())
        .text("language", "zh")
        .part("file", part);
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
        "asr_options": { "language": "zh", "enable_itn": true }
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

fn shortcut_allowed(action: &'static str) -> bool {
    static LAST_SCREENSHOT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    if action != "capture-full" {
        return true;
    }
    let now = Instant::now();
    let mut last = match LAST_SCREENSHOT.get_or_init(|| Mutex::new(None)).lock() {
        Ok(last) => last,
        Err(_) => return false,
    };
    if last.is_some_and(|previous| now.duration_since(previous) < Duration::from_millis(600)) {
        return false;
    }
    *last = Some(now);
    true
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
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
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
            let mut settings: AppSettings = fs::read_to_string(source_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default();
            let mut persist_settings = source_path != &settings_path;
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
            }
            app.manage(AppState {
                settings: RwLock::new(settings),
                settings_path,
                system_audio: Mutex::new(None),
                capture_origin: Mutex::new(None),
            });
            if let Some(window) = app.get_webview_window("main") {
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
                ("CommandOrControl+Shift+S", "capture-full"),
                ("CommandOrControl+Shift+X", "clear"),
                ("CommandOrControl+Shift+1", "capture-origin"),
                ("CommandOrControl+Shift+2", "capture-region"),
                ("CommandOrControl+Shift+Comma", "manual-start"),
                ("CommandOrControl+Shift+Period", "manual-stop"),
                ("CommandOrControl+Shift+L", "auto-start"),
                ("CommandOrControl+Shift+K", "auto-stop"),
                ("CommandOrControl+Shift+H", "send"),
                ("CommandOrControl+Shift+Space", "toggle-window"),
                ("CommandOrControl+Q", "quit"),
            ];
            for (shortcut, action) in shortcuts {
                app.global_shortcut()
                    .on_shortcut(shortcut, move |app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed && shortcut_allowed(action) {
                            handle_shortcut_action(app, action);
                        }
                    })
                    .map_err(|error| format!("注册快捷键 {shortcut} 失败：{error}"))?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            capture_primary_monitor,
            mark_capture_origin,
            capture_marked_region,
            ask_llm,
            transcribe_audio,
            start_system_audio,
            stop_system_audio_and_transcribe
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Interview Buddy");
}
