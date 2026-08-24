import { useTranslation } from "react-i18next";
import type { WindowSizeInfo, WindowSizePreset } from "../../shared/types";

type WindowDisplaySettingsProps = {
  preset: WindowSizePreset;
  info: WindowSizeInfo | null;
  loading: boolean;
  issue: string;
  onPresetChange: (preset: WindowSizePreset) => void;
};

const PRESETS: WindowSizePreset[] = ["compact", "standard", "spacious", "custom"];

export function WindowDisplaySettings({
  preset,
  info,
  loading,
  issue,
  onPresetChange,
}: WindowDisplaySettingsProps) {
  const { t } = useTranslation();
  return <section className="window-settings-section">
    <div className="general-page-heading">
      <strong>{t("window.title")}</strong>
      <span>{t("window.description")}</span>
    </div>
    <div className="window-preset-grid" role="radiogroup" aria-label={t("window.sizePreset")}>
      {PRESETS.map((value) => <button key={value} type="button" role="radio"
        aria-checked={preset === value} className={preset === value ? "active" : ""}
        onClick={() => onPresetChange(value)}>
        <span><b>{t(`window.presets.${value}.title`)}</b>
          {value === "standard" && <i>{t("window.recommended")}</i>}</span>
        <small>{t(`window.presets.${value}.description`)}</small>
      </button>)}
    </div>
    <div className="window-size-summary" aria-live="polite">
      <div><span>{t("window.currentSize")}</span>
        <strong>{loading ? t("window.applying") : info ? `${info.width} × ${info.height}` : "—"}</strong></div>
      <div><span>{t("window.monitorWorkspace")}</span>
        <strong>{info ? `${info.monitorWidth} × ${info.monitorHeight}` : "—"}</strong></div>
      <div><span>{t("window.scaleFactor")}</span>
        <strong>{info ? `${Math.round(info.scaleFactor * 100)}%` : "—"}</strong></div>
    </div>
    {preset === "custom" && <p className="window-custom-note">{t("window.customHint")}</p>}
    {issue && <div className="device-issue" role="alert">{issue}</div>}
  </section>;
}
