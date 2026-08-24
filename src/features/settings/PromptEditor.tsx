import type { PromptMode } from "../../shared/types";
import { useTranslation } from "react-i18next";

type PromptEditorProps = {
  title: string;
  description: string;
  mode: PromptMode;
  value: string | null;
  defaultValue: string;
  rows: number;
  onModeChange: (mode: PromptMode) => void;
  onValueChange: (value: string) => void;
};

export function PromptEditor({
  title,
  description,
  mode,
  value,
  defaultValue,
  rows,
  onModeChange,
  onValueChange,
}: PromptEditorProps) {
  const { t } = useTranslation();
  const visibleValue = mode === "default" ? defaultValue : value || "";
  return <section className={`prompt-card ${mode}`}>
    <header className="prompt-card-heading">
      <div><b>{title}</b><span>{description}</span></div>
      <select aria-label={t("prompt.modeLabel", { title })} value={mode}
        onChange={(event) => onModeChange(event.target.value as PromptMode)}>
        <option value="default">{t("prompt.recommended")}</option>
        <option value="custom">{t("prompt.custom")}</option>
        <option value="disabled">{t("prompt.disabled")}</option>
      </select>
    </header>
    {mode === "disabled" ? <div className="prompt-disabled-warning">
      {t("prompt.disabledWarning")}
    </div> : <>
      <textarea rows={rows} value={visibleValue} readOnly={mode === "default"}
        onChange={(event) => onValueChange(event.target.value)} />
      <footer className="prompt-card-footer">
        <span>{mode === "default" ? t("prompt.builtinUpgrade") : t("prompt.characterCount", { count: visibleValue.length })}</span>
        {mode === "default"
          ? <button onClick={() => onModeChange("custom")}>{t("prompt.copyCustom")}</button>
          : <button onClick={() => onModeChange("default")}>{t("prompt.restoreDefault")}</button>}
      </footer>
    </>}
  </section>;
}
