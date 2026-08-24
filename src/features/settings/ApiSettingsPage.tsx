import { useTranslation } from "react-i18next";
import type { AppSettings } from "../../shared/types";
import { ApiKeyField } from "./ApiKeyField";

type ApiSettingsPageProps = {
  settings: AppSettings;
  apiKeyConfigured: boolean;
  apiKeyDraft: string;
  apiKeyPendingClear: boolean;
  onSettingsChange: (settings: AppSettings) => void;
  onApiKeyDraftChange: (value: string) => void;
  onApiKeyClear: () => void;
};

export function ApiSettingsPage(props: ApiSettingsPageProps) {
  const { t } = useTranslation();
  const { settings, onSettingsChange } = props;
  return <div className="settings-page api-page">
    <label>API Base URL<input value={settings.baseUrl}
      onChange={(event) => onSettingsChange({ ...settings, baseUrl: event.target.value })} /></label>
    <ApiKeyField configured={props.apiKeyConfigured} draft={props.apiKeyDraft}
      pendingClear={props.apiKeyPendingClear} onDraftChange={props.onApiKeyDraftChange}
      onClear={props.onApiKeyClear} />
    <div className="settings-grid">
      <label>{t("api.textModel")}<input value={settings.model}
        onChange={(event) => onSettingsChange({ ...settings, model: event.target.value })} /></label>
      <label>{t("api.visionModel")}<input value={settings.visionModel}
        onChange={(event) => onSettingsChange({ ...settings, visionModel: event.target.value })} /></label>
    </div>
    <label>{t("api.transcriptionModel")}<input value={settings.transcriptionModel}
      onChange={(event) => onSettingsChange({ ...settings, transcriptionModel: event.target.value })} /></label>
  </div>;
}
