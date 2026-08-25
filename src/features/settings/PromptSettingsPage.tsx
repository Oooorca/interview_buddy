import { useTranslation } from "react-i18next";
import { defaultPromptsFor } from "../../shared/settings";
import type { AppSettings, PromptMode } from "../../shared/types";
import { PromptEditor } from "./PromptEditor";

type PromptSettingsPageProps = {
  settings: AppSettings;
  issue: string;
  onModeChange: (kind: "system" | "coding", mode: PromptMode) => void;
  onValueChange: (kind: "system" | "coding", value: string) => void;
};

export function PromptSettingsPage(props: PromptSettingsPageProps) {
  const { t } = useTranslation();
  const defaults = defaultPromptsFor(props.settings);
  return <div className="settings-page prompt-page">
    <div className="prompt-page-heading">
      <strong>{t("prompt.pageTitle")}</strong>
      <span>{t("prompt.pageDescription")}</span>
    </div>
    {props.issue && <div className="prompt-issue">{props.issue}</div>}
    <PromptEditor title={t("prompt.systemTitle")} description={t("prompt.systemDescription")}
      mode={props.settings.systemPromptMode} value={props.settings.systemPrompt}
      defaultValue={defaults.system} rows={7}
      onModeChange={(mode) => props.onModeChange("system", mode)}
      onValueChange={(value) => props.onValueChange("system", value)} />
    <PromptEditor title={t("prompt.codingTitle")} description={t("prompt.codingDescription")}
      mode={props.settings.codingPromptMode} value={props.settings.codingPrompt}
      defaultValue={defaults.coding} rows={9}
      onModeChange={(mode) => props.onModeChange("coding", mode)}
      onValueChange={(value) => props.onValueChange("coding", value)} />
  </div>;
}
