import type { AppSettings, AudioOutputDevice } from "../../shared/types";
import { useTranslation } from "react-i18next";

const LANGUAGE_OPTIONS = [
  ["auto", "languages.auto"], ["zh", "languages.zh"], ["en-US", "languages.en"], ["ja", "languages.ja"],
  ["ko", "languages.ko"], ["de", "languages.de"], ["fr", "languages.fr"], ["es", "languages.es"],
] as const;

export type AudioSettingsPageProps = {
  settings: AppSettings;
  microphoneDevices: MediaDeviceInfo[];
  outputDevices: AudioOutputDevice[];
  loading: boolean;
  issue: string;
  onSettingsChange: (settings: AppSettings) => void;
  onRefresh: () => void;
};

export function AudioSettingsPage({ settings, microphoneDevices, outputDevices, loading, issue, onSettingsChange, onRefresh }: AudioSettingsPageProps) {
  const { t } = useTranslation();
  return <div className="settings-page audio-page">
    <div className="audio-page-heading">
      <div><strong>{t("audio.title")}</strong><span>{t("audio.description")}</span></div>
      <button className="refresh-devices" disabled={loading} onClick={onRefresh}>{loading ? t("audio.loading") : t("audio.authorizeRefresh")}</button>
    </div>
    {issue && <div className="device-issue">{issue}</div>}
    <section className="audio-channel-card">
      <header><div><b>{t("audio.myVoice")}</b><span>{t("audio.microphoneInput")}</span></div>
        <label className="toggle-setting"><input type="checkbox" checked={settings.captureMicrophone}
          onChange={(event) => onSettingsChange({ ...settings, captureMicrophone: event.target.checked })} /><i />{t("audio.enabled")}</label></header>
      <div className="audio-channel-fields">
        <label>{t("audio.inputDevice")}<select disabled={!settings.captureMicrophone} value={settings.microphoneDeviceId}
          onChange={(event) => onSettingsChange({ ...settings, microphoneDeviceId: event.target.value })}>
          <option value="">{t("audio.defaultMicrophone")}</option>
          {settings.microphoneDeviceId && !microphoneDevices.some((device) => device.deviceId === settings.microphoneDeviceId)
            && <option value={settings.microphoneDeviceId}>{t("audio.selectedUnavailable")}</option>}
          {microphoneDevices.filter((device) => device.deviceId !== "default").map((device, index) =>
            <option key={device.deviceId} value={device.deviceId}>{device.label || t("audio.microphoneNumber", { number: index + 1 })}</option>)}
        </select></label>
        <label>{t("audio.myLanguage")}<select disabled={!settings.captureMicrophone} value={settings.myTranscriptionLanguage}
          onChange={(event) => onSettingsChange({ ...settings, myTranscriptionLanguage: event.target.value })}>
          {LANGUAGE_OPTIONS.map(([value, label]) => <option key={value} value={value}>{t(label)}</option>)}
        </select></label>
      </div>
    </section>
    <section className="audio-channel-card counterpart">
      <header><div><b>{t("audio.otherVoice")}</b><span>{t("audio.systemLoopback")}</span></div>
        <label className="toggle-setting"><input type="checkbox" checked={settings.captureSystemAudio}
          onChange={(event) => onSettingsChange({ ...settings, captureSystemAudio: event.target.checked })} /><i />{t("audio.enabled")}</label></header>
      <div className="audio-channel-fields">
        <label>{t("audio.outputDevice")}<select disabled={!settings.captureSystemAudio} value={settings.systemAudioDeviceId}
          onChange={(event) => onSettingsChange({ ...settings, systemAudioDeviceId: event.target.value })}>
          <option value="">{t("audio.defaultOutput")}</option>
          {settings.systemAudioDeviceId && !outputDevices.some((device) => device.id === settings.systemAudioDeviceId)
            && <option value={settings.systemAudioDeviceId}>{t("audio.selectedUnavailable")}</option>}
          {outputDevices.filter((device) => device.id).map((device) => <option key={device.id} value={device.id}>
            {device.name}{device.isDefault ? ` (${t("audio.currentDefault")})` : ""}
          </option>)}
        </select></label>
        <label>{t("audio.otherLanguage")}<select disabled={!settings.captureSystemAudio} value={settings.theirTranscriptionLanguage}
          onChange={(event) => onSettingsChange({ ...settings, theirTranscriptionLanguage: event.target.value })}>
          {LANGUAGE_OPTIONS.map(([value, label]) => <option key={value} value={value}>{t(label)}</option>)}
        </select></label>
      </div>
    </section>
    <p className="audio-note">{t("audio.note")}</p>
  </div>;
}
