import type { AppSettings, AudioOutputDevice } from "../../shared/types";

const LANGUAGE_OPTIONS = [
  ["auto", "自动检测"], ["zh", "中文"], ["en", "English"], ["ja", "日本語"],
  ["ko", "한국어"], ["de", "Deutsch"], ["fr", "Français"], ["es", "Español"],
] as const;

type AudioSettingsPageProps = {
  settings: AppSettings;
  microphoneDevices: MediaDeviceInfo[];
  outputDevices: AudioOutputDevice[];
  loading: boolean;
  issue: string;
  onSettingsChange: (settings: AppSettings) => void;
  onRefresh: () => void;
};

export function AudioSettingsPage({ settings, microphoneDevices, outputDevices, loading, issue, onSettingsChange, onRefresh }: AudioSettingsPageProps) {
  return <div className="settings-page audio-page">
    <div className="audio-page-heading">
      <div><strong>音频输入与输出</strong><span>设备修改将在下次开始听写时生效</span></div>
      <button className="refresh-devices" disabled={loading} onClick={onRefresh}>{loading ? "读取中…" : "授权并刷新设备"}</button>
    </div>
    {issue && <div className="device-issue">{issue}</div>}
    <section className="audio-channel-card">
      <header><div><b>我的声音</b><span>麦克风输入</span></div>
        <label className="toggle-setting"><input type="checkbox" checked={settings.captureMicrophone}
          onChange={(event) => onSettingsChange({ ...settings, captureMicrophone: event.target.checked })} /><i />启用</label></header>
      <div className="audio-channel-fields">
        <label>输入设备<select disabled={!settings.captureMicrophone} value={settings.microphoneDeviceId}
          onChange={(event) => onSettingsChange({ ...settings, microphoneDeviceId: event.target.value })}>
          <option value="">系统默认麦克风</option>
          {settings.microphoneDeviceId && !microphoneDevices.some((device) => device.deviceId === settings.microphoneDeviceId)
            && <option value={settings.microphoneDeviceId}>已选择的设备当前不可用</option>}
          {microphoneDevices.filter((device) => device.deviceId !== "default").map((device, index) =>
            <option key={device.deviceId} value={device.deviceId}>{device.label || `麦克风 ${index + 1}`}</option>)}
        </select></label>
        <label>我的语言<select disabled={!settings.captureMicrophone} value={settings.myTranscriptionLanguage}
          onChange={(event) => onSettingsChange({ ...settings, myTranscriptionLanguage: event.target.value })}>
          {LANGUAGE_OPTIONS.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
        </select></label>
      </div>
    </section>
    <section className="audio-channel-card counterpart">
      <header><div><b>对方声音</b><span>系统输出回环</span></div>
        <label className="toggle-setting"><input type="checkbox" checked={settings.captureSystemAudio}
          onChange={(event) => onSettingsChange({ ...settings, captureSystemAudio: event.target.checked })} /><i />启用</label></header>
      <div className="audio-channel-fields">
        <label>输出设备<select disabled={!settings.captureSystemAudio} value={settings.systemAudioDeviceId}
          onChange={(event) => onSettingsChange({ ...settings, systemAudioDeviceId: event.target.value })}>
          <option value="">系统默认输出设备</option>
          {settings.systemAudioDeviceId && !outputDevices.some((device) => device.id === settings.systemAudioDeviceId)
            && <option value={settings.systemAudioDeviceId}>已选择的设备当前不可用</option>}
          {outputDevices.filter((device) => device.id).map((device) => <option key={device.id} value={device.id}>
            {device.name}{device.isDefault ? "（当前默认）" : ""}
          </option>)}
        </select></label>
        <label>对方语言<select disabled={!settings.captureSystemAudio} value={settings.theirTranscriptionLanguage}
          onChange={(event) => onSettingsChange({ ...settings, theirTranscriptionLanguage: event.target.value })}>
          {LANGUAGE_OPTIONS.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
        </select></label>
      </div>
    </section>
    <p className="audio-note">“自动检测”适合中英混合；明确知道语种时固定语言通常更准确。系统输出设备决定捕获哪一路会议声音。</p>
  </div>;
}
