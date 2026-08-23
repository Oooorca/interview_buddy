pub struct SystemAudioRecorder;

impl SystemAudioRecorder {
    pub fn start() -> Result<Self, String> {
        Err("当前平台暂不支持系统音频采集".into())
    }

    pub fn stop(self) -> Result<Vec<u8>, String> {
        Err("当前平台暂不支持系统音频采集".into())
    }
}
