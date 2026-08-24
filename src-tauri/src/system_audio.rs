//! Windows WASAPI loopback recorder.
//!
//! Adapted from Wisp's MIT-licensed `wisp-loopback` crate:
//! https://github.com/ppXD/Wisp/tree/main/crates/wisp-loopback

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::audio::pcm_wav;

use serde::Serialize;
use windows::core::PCWSTR;
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_LOOPBACK, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::StructuredStorage::PropVariantToString;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};

const REFTIMES_PER_SEC: i64 = 10_000_000;
const POLL: Duration = Duration::from_millis(10);
const MAX_SECONDS: usize = 60 * 30;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list_output_devices() -> Result<Vec<AudioOutputDevice>, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|error| format!("创建音频设备枚举器失败：{error}"))?;
        let default_id = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .ok()
            .and_then(|device| device_id(&device).ok());
        let collection = enumerator
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .map_err(|error| format!("枚举系统输出设备失败：{error}"))?;
        let count = collection.GetCount().map_err(|error| error.to_string())?;
        let mut devices = Vec::with_capacity(count as usize);
        for index in 0..count {
            let device = collection.Item(index).map_err(|error| error.to_string())?;
            let id = device_id(&device)?;
            let name = friendly_name(&device).unwrap_or_else(|_| id.clone());
            devices.push(AudioOutputDevice {
                is_default: default_id.as_deref() == Some(id.as_str()),
                id,
                name,
            });
        }
        devices.sort_by_key(|device| !device.is_default);
        Ok(devices)
    }
}

unsafe fn device_id(device: &IMMDevice) -> Result<String, String> {
    let raw = device.GetId().map_err(|error| error.to_string())?;
    let id = raw.to_string().map_err(|error| error.to_string())?;
    CoTaskMemFree(Some(raw.0.cast()));
    Ok(id)
}

unsafe fn friendly_name(device: &IMMDevice) -> Result<String, String> {
    let store = device
        .OpenPropertyStore(STGM_READ)
        .map_err(|error| error.to_string())?;
    let value = store
        .GetValue(&PKEY_Device_FriendlyName)
        .map_err(|error| error.to_string())?;
    let mut buffer = [0u16; 512];
    PropVariantToString(&value, &mut buffer).map_err(|error| error.to_string())?;
    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..length]))
}

pub struct SystemAudioRecorder {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: Arc<Mutex<u32>>,
    activity: Arc<Mutex<(f32, Instant)>>,
}

impl SystemAudioRecorder {
    pub fn start(device_id: Option<String>) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let sample_rate = Arc::new(Mutex::new(48_000));
        let activity = Arc::new(Mutex::new((0.0, Instant::now())));
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread_stop = Arc::clone(&stop);
        let thread_samples = Arc::clone(&samples);
        let thread_rate = Arc::clone(&sample_rate);
        let thread_activity = Arc::clone(&activity);
        let handle = thread::Builder::new()
            .name("wasapi-system-audio".into())
            .spawn(move || {
                let result = unsafe {
                    run_capture(
                        &thread_stop,
                        &thread_samples,
                        &thread_rate,
                        &thread_activity,
                        &ready_tx,
                        device_id.as_deref(),
                    )
                };
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
                activity,
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
        self.take_chunk()
    }

    pub fn take_chunk(&self) -> Result<Vec<u8>, String> {
        let sample_rate = *self.sample_rate.lock().map_err(|error| error.to_string())?;
        let mut samples = self.samples.lock().map_err(|error| error.to_string())?;
        let chunk = std::mem::take(&mut *samples);
        if !has_audio_activity(&chunk) {
            return Ok(Vec::new());
        }
        Ok(pcm_wav(&chunk, sample_rate))
    }

    pub fn clear_chunk(&self) -> Result<(), String> {
        self.samples
            .lock()
            .map_err(|error| error.to_string())?
            .clear();
        Ok(())
    }

    pub fn activity_level(&self) -> Result<f32, String> {
        let (level, updated_at) = *self.activity.lock().map_err(|error| error.to_string())?;
        Ok(if updated_at.elapsed() <= Duration::from_millis(350) {
            level
        } else {
            0.0
        })
    }
}

fn has_audio_activity(samples: &[f32]) -> bool {
    samples.len() >= 1_600 && samples.iter().any(|sample| sample.abs() >= 0.0015)
}

unsafe fn run_capture(
    stop: &AtomicBool,
    samples: &Mutex<Vec<f32>>,
    output_rate: &Mutex<u32>,
    activity: &Mutex<(f32, Instant)>,
    ready: &mpsc::Sender<Result<(), String>>,
    device_id: Option<&str>,
) -> Result<(), String> {
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|error| format!("创建音频设备枚举器失败：{error}"))?;
    let device = if let Some(device_id) = device_id.filter(|id| !id.is_empty()) {
        let wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
        enumerator
            .GetDevice(PCWSTR::from_raw(wide.as_ptr()))
            .map_err(|error| format!("打开指定输出设备失败：{error}"))?
    } else {
        enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|error| format!("打开默认输出设备失败：{error}"))?
    };
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
                let rms = rms_level(&mono);
                *activity.lock().map_err(|error| error.to_string())? = (rms, Instant::now());
                let mut output = samples.lock().map_err(|error| error.to_string())?;
                let remaining = sample_rate as usize * MAX_SECONDS - output.len();
                output.extend(mono.into_iter().take(remaining));
            } else {
                *activity.lock().map_err(|error| error.to_string())? = (0.0, Instant::now());
            }
            capture
                .ReleaseBuffer(frames)
                .map_err(|error| error.to_string())?;
        }
    }
    let _ = client.Stop();
    Ok(())
}

fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
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
        let recorder = SystemAudioRecorder::start(None).expect("WASAPI loopback should start");
        std::thread::sleep(Duration::from_millis(200));
        let _ = recorder.stop();
    }

    #[test]
    fn activity_filter_rejects_silence_and_keeps_signal() {
        assert!(!has_audio_activity(&vec![0.0; 1_600]));
        let mut signal = vec![0.0; 1_600];
        signal[800] = 0.01;
        assert!(has_audio_activity(&signal));
    }

    #[test]
    fn rms_reports_signal_energy() {
        assert_eq!(rms_level(&[0.0, 0.0]), 0.0);
        assert!((rms_level(&[0.5, -0.5]) - 0.5).abs() < 0.0001);
    }
}
