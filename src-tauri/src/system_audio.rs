//! Windows WASAPI loopback recorder.
//!
//! Adapted from Wisp's MIT-licensed `wisp-loopback` crate:
//! https://github.com/ppXD/Wisp/tree/main/crates/wisp-loopback

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::audio::pcm_wav;

use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED,
};

const REFTIMES_PER_SEC: i64 = 10_000_000;
const POLL: Duration = Duration::from_millis(10);
const MAX_SECONDS: usize = 60 * 30;

pub struct SystemAudioRecorder {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: Arc<Mutex<u32>>,
}

impl SystemAudioRecorder {
    pub fn start() -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let sample_rate = Arc::new(Mutex::new(48_000));
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread_stop = Arc::clone(&stop);
        let thread_samples = Arc::clone(&samples);
        let thread_rate = Arc::clone(&sample_rate);
        let handle = thread::Builder::new()
            .name("wasapi-system-audio".into())
            .spawn(move || {
                let result =
                    unsafe { run_capture(&thread_stop, &thread_samples, &thread_rate, &ready_tx) };
                if let Err(error) = result {
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|error| error.to_string())?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self {
                stop,
                handle: Some(handle),
                samples,
                sample_rate,
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err("WASAPI 启动超时".into()),
        }
    }

    pub fn stop(mut self) -> Result<Vec<u8>, String> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| "WASAPI 采集线程异常退出".to_string())?;
        }
        let sample_rate = *self.sample_rate.lock().map_err(|error| error.to_string())?;
        let samples = self.samples.lock().map_err(|error| error.to_string())?;
        if samples.is_empty() {
            return Err("没有捕获到系统音频；请确认会议或播放器正在输出声音".into());
        }
        Ok(pcm_wav(&samples, sample_rate))
    }
}

unsafe fn run_capture(
    stop: &AtomicBool,
    samples: &Mutex<Vec<f32>>,
    output_rate: &Mutex<u32>,
    ready: &mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|error| format!("创建音频设备枚举器失败：{error}"))?;
    let device = enumerator
        .GetDefaultAudioEndpoint(eRender, eConsole)
        .map_err(|error| format!("打开默认输出设备失败：{error}"))?;
    let client: IAudioClient = device
        .Activate(CLSCTX_ALL, None)
        .map_err(|error| format!("激活 WASAPI 客户端失败：{error}"))?;
    let format = client
        .GetMixFormat()
        .map_err(|error| format!("读取输出格式失败：{error}"))?;
    let channels = (*format).nChannels as usize;
    let sample_rate = (*format).nSamplesPerSec;
    let bits = (*format).wBitsPerSample;
    *output_rate.lock().map_err(|error| error.to_string())? = sample_rate;
    client
        .Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            REFTIMES_PER_SEC,
            0,
            format,
            None,
        )
        .map_err(|error| format!("初始化 WASAPI loopback 失败：{error}"))?;
    CoTaskMemFree(Some(format as *const _));
    let capture: IAudioCaptureClient = client
        .GetService()
        .map_err(|error| format!("获取 WASAPI 采集接口失败：{error}"))?;
    client
        .Start()
        .map_err(|error| format!("启动系统音频失败：{error}"))?;
    let _ = ready.send(Ok(()));

    while !stop.load(Ordering::Relaxed) {
        thread::sleep(POLL);
        while capture
            .GetNextPacketSize()
            .map_err(|error| error.to_string())?
            > 0
        {
            let mut data = std::ptr::null_mut();
            let mut frames = 0u32;
            let mut flags = 0u32;
            capture
                .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
                .map_err(|error| error.to_string())?;
            let silent = (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;
            if !silent && frames > 0 && channels > 0 && !data.is_null() {
                let mono = downmix(data, frames as usize, channels, bits);
                let mut output = samples.lock().map_err(|error| error.to_string())?;
                let remaining = sample_rate as usize * MAX_SECONDS - output.len();
                output.extend(mono.into_iter().take(remaining));
            }
            capture
                .ReleaseBuffer(frames)
                .map_err(|error| error.to_string())?;
        }
    }
    let _ = client.Stop();
    Ok(())
}

unsafe fn downmix(data: *const u8, frames: usize, channels: usize, bits: u16) -> Vec<f32> {
    let total = frames * channels;
    match bits {
        32 => std::slice::from_raw_parts(data as *const f32, total)
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect(),
        16 => std::slice::from_raw_parts(data as *const i16, total)
            .chunks(channels)
            .map(|frame| {
                frame
                    .iter()
                    .map(|&value| value as f32 / 32_768.0)
                    .sum::<f32>()
                    / channels as f32
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manual hardware smoke test. Run while a speaker/headphone endpoint is active:
    /// `cargo test wasapi_loopback_starts -- --ignored`
    #[test]
    #[ignore]
    fn wasapi_loopback_starts() {
        let recorder = SystemAudioRecorder::start().expect("WASAPI loopback should start");
        std::thread::sleep(Duration::from_millis(200));
        let _ = recorder.stop();
    }
}
