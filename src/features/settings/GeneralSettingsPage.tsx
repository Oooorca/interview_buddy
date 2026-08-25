import { useTranslation } from "react-i18next";
import type { AppSettings, AnswerLanguage, AudioOutputDevice, StorageInfo, UiLanguage, WindowSizeInfo, WindowSizePreset } from "../../shared/types";
import { AudioSettingsPage } from "./AudioSettingsPage";
import { StorageSettingsPage } from "./StorageSettingsPage";
import { WindowDisplaySettings } from "./WindowDisplaySettings";

type GeneralSettingsPageProps = {
  settings: AppSettings;
  microphoneDevices: MediaDeviceInfo[];
  outputDevices: AudioOutputDevice[];
  devicesLoading: boolean;
  deviceIssue: string;
  storageInfo: StorageInfo | null;
  storageLoading: boolean;
  storageIssue: string;
  windowSizeInfo: WindowSizeInfo | null;
  windowSizeLoading: boolean;
  windowSizeIssue: string;
  onSettingsChange: (settings: AppSettings) => void;
  onRefreshAudio: () => void;
  onRefreshStorage: () => void;
  onChooseStorageRoot: () => void;
  onRestoreStorageRoot: () => void;
  onScheduleCleanup: () => void;
  onWindowPresetChange: (preset: WindowSizePreset) => void;
};

export function GeneralSettingsPage(props: GeneralSettingsPageProps) {
  const { t } = useTranslation();
  const { settings, onSettingsChange } = props;
  return <div className="settings-page general-page">
    <div className="general-column general-primary-column">
      <section className="language-settings-section">
        <div className="general-page-heading">
          <strong>{t("general.title")}</strong>
          <span>{t("general.description")}</span>
        </div>
        <div className="general-language-card">
          <label>{t("general.uiLanguage")}
            <select value={settings.uiLanguage} onChange={(event) => onSettingsChange({
              ...settings,
              uiLanguage: event.target.value as UiLanguage,
            })}>
              <option value="system">{t("languages.system")}</option>
              <option value="zh-CN">简体中文</option>
              <option value="en-US">English (United States)</option>
            </select>
            <span>{t("general.uiLanguageHint")}</span>
          </label>
          <label>{t("general.answerLanguage")}
            <select value={settings.answerLanguage} onChange={(event) => onSettingsChange({
              ...settings,
              answerLanguage: event.target.value as AnswerLanguage,
            })}>
              <option value="follow-ui">{t("languages.followUi")}</option>
              <option value="zh-CN">简体中文</option>
              <option value="en-US">English (United States)</option>
            </select>
            <span>{t("general.answerLanguageHint")}</span>
          </label>
        </div>
        <p className="general-language-note">{t("general.transcriptionIndependent")}</p>
      </section>
      <AudioSettingsPage settings={settings} microphoneDevices={props.microphoneDevices}
        outputDevices={props.outputDevices} loading={props.devicesLoading} issue={props.deviceIssue}
        onSettingsChange={onSettingsChange} onRefresh={props.onRefreshAudio} />
    </div>
    <div className="general-column general-storage-column">
      <WindowDisplaySettings preset={settings.windowSizePreset} info={props.windowSizeInfo}
        loading={props.windowSizeLoading} issue={props.windowSizeIssue}
        onPresetChange={props.onWindowPresetChange} />
      <StorageSettingsPage settings={settings} info={props.storageInfo}
        loading={props.storageLoading} issue={props.storageIssue} onSettingsChange={onSettingsChange}
        onRefresh={props.onRefreshStorage} onChooseRoot={props.onChooseStorageRoot}
        onRestoreDefault={props.onRestoreStorageRoot} onScheduleCleanup={props.onScheduleCleanup} />
    </div>
  </div>;
}
