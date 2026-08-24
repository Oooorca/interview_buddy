//! macOS system-audio recorder backed by ScreenCaptureKit (macOS 13+).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use screencapturekit::prelude::*;

use crate::audio::pcm_wav;

const SAMPLE_RATE: u32 = 16_000;
const MAX_SAMPLES: usize = SAMPLE_RATE as usize * 60 * 30;

pub struct SystemAudioRecorder {
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    handle: Option<JoinHandle<()>>,
}

impl SystemAudioRecorder {
    pub fn start() -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread_stop = Arc::clone(&stop);
        let thread_samples = Arc::clone(&samples);
        let handle = thread::Builder::new()
            .name("screencapturekit-system-audio".into())
            .spawn(move || {
                let result = run_capture(&thread_stop, &thread_samples, &ready_tx);
                if let Err(error) = result {
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|error| error.to_string())?;

        match ready_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => Ok(Self {
                stop,
                samples,
                handle: Some(handle),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(
                "系统音频启动超时。若 macOS 的授权开关已经打开，请先关闭再重新打开 Interview Buddy 的“屏幕与系统音频录制”，然后彻底退出并重启应用。本地临时签名在重新构建后可能需要重新授权。"
                    .into(),
            ),
        }
    }

    pub fn stop(mut self) -> Result<Vec<u8>, String> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| "ScreenCaptureKit 采集线程异常退出".to_string())?;
        }
        let samples = self.samples.lock().map_err(|error| error.to_string())?;
        if samples.is_empty() {
            return Err("没有捕获到系统音频；请确认已授权且会议正在输出声音".into());
        }
        Ok(pcm_wav(&samples, SAMPLE_RATE))
    }
}

fn run_capture(
    stop: &AtomicBool,
    samples: &Arc<Mutex<Vec<f32>>>,
    ready: &mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    let content =
        SCShareableContent::get().map_err(|error| format!("读取屏幕内容失败：{error}"))?;
    let display = content
        .displays()
        .into_iter()
        .next()
        .ok_or_else(|| "没有找到显示器".to_string())?;
    let filter = SCContentFilter::create()
        .with_display(&display)
        .with_excluding_windows(&[])
        .build();
    let config = SCStreamConfiguration::new()
        .with_width(2)
        .with_height(2)
        .with_captures_audio(true)
        .with_excludes_current_process_audio(true)
        .with_sample_rate(SAMPLE_RATE as i32)
        .with_channel_count(1);
    let callback_samples = Arc::clone(samples);
    let mut stream = SCStream::new(&filter, &config);
    stream.add_output_handler(
        move |sample: CMSampleBuffer, _| {
            let Some(buffers) = sample.audio_buffer_list() else {
                return;
            };
            let Ok(mut output) = callback_samples.lock() else {
                return;
            };
            let remaining = MAX_SAMPLES.saturating_sub(output.len());
            output.extend(
                buffers
                    .iter()
                    .flat_map(|buffer| buffer.data().chunks_exact(4))
                    .map(|bytes| f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .take(remaining),
            );
        },
        SCStreamOutputType::Audio,
    );
    stream
        .start_capture()
        .map_err(|error| format!("启动系统音频失败：{error}"))?;
    let _ = ready.send(Ok(()));
    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(50));
    }
    stream
        .stop_capture()
        .map_err(|error| format!("停止系统音频失败：{error}"))
}
