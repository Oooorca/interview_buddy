import { DEFAULT_CODING_PROMPT, DEFAULT_SYSTEM_PROMPT } from "../../shared/settings";
import type { AppSettings, PromptMode } from "../../shared/types";
import { ApiKeyField } from "./ApiKeyField";
import { PromptEditor } from "./PromptEditor";

type ApiSettingsPageProps = {
  settings: AppSettings;
  apiKeyConfigured: boolean;
  apiKeyDraft: string;
  apiKeyPendingClear: boolean;
  promptIssue: string;
  onSettingsChange: (settings: AppSettings) => void;
  onApiKeyDraftChange: (value: string) => void;
  onApiKeyClear: () => void;
  onPromptModeChange: (kind: "system" | "coding", mode: PromptMode) => void;
  onPromptValueChange: (kind: "system" | "coding", value: string) => void;
};

export function ApiSettingsPage(props: ApiSettingsPageProps) {
  const { settings, onSettingsChange } = props;
  return <div className="settings-page api-page">
    <label>API Base URL<input value={settings.baseUrl}
      onChange={(event) => onSettingsChange({ ...settings, baseUrl: event.target.value })} /></label>
    <ApiKeyField configured={props.apiKeyConfigured} draft={props.apiKeyDraft}
      pendingClear={props.apiKeyPendingClear} onDraftChange={props.onApiKeyDraftChange}
      onClear={props.onApiKeyClear} />
    <div className="settings-grid">
      <label>文本模型<input value={settings.model}
        onChange={(event) => onSettingsChange({ ...settings, model: event.target.value })} /></label>
      <label>视觉模型<input value={settings.visionModel}
        onChange={(event) => onSettingsChange({ ...settings, visionModel: event.target.value })} /></label>
    </div>
    <label>转写模型<input value={settings.transcriptionModel}
      onChange={(event) => onSettingsChange({ ...settings, transcriptionModel: event.target.value })} /></label>
    {props.promptIssue && <div className="prompt-issue">{props.promptIssue}</div>}
    <PromptEditor title="系统 Prompt" description="控制所有回答的身份、语气与事实边界"
      mode={settings.systemPromptMode} value={settings.systemPrompt} defaultValue={DEFAULT_SYSTEM_PROMPT} rows={7}
      onModeChange={(mode) => props.onPromptModeChange("system", mode)}
      onValueChange={(value) => props.onPromptValueChange("system", value)} />
    <PromptEditor title="纯截图 Prompt" description="仅截图、没有本轮文字时使用的解题要求"
      mode={settings.codingPromptMode} value={settings.codingPrompt} defaultValue={DEFAULT_CODING_PROMPT} rows={9}
      onModeChange={(mode) => props.onPromptModeChange("coding", mode)}
      onValueChange={(value) => props.onPromptValueChange("coding", value)} />
  </div>;
}
