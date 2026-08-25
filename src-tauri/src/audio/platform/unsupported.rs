use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list_output_devices() -> Result<Vec<AudioOutputDevice>, String> {
    Ok(Vec::new())
}

pub struct SystemAudioRecorder;

impl SystemAudioRecorder {
    pub fn start(_device_id: Option<String>) -> Result<Self, String> {
        Err("当前平台暂不支持系统音频采集".into())
    }

    pub fn stop(self) -> Result<Vec<u8>, String> {
        Err("当前平台暂不支持系统音频采集".into())
    }

    pub fn take_chunk(&self) -> Result<Vec<u8>, String> {
        Err("当前平台暂不支持系统音频采集".into())
    }

    pub fn clear_chunk(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn activity_level(&self) -> Result<f32, String> {
        Ok(0.0)
    }
}
