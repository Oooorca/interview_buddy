import { getCurrentWindow } from "@tauri-apps/api/window";
import { appPlatform } from "../../platform";
import { backend } from "../../services/backend";
import { useTranslation } from "react-i18next";

type TitleBarProps = {
  status: "ready" | "working" | "error";
  listening: boolean;
  autoAnswer: boolean;
  settingsOpen: boolean;
  securityLocked: boolean;
  notice: string;
  shortcutIssue: string;
  onCapture: () => void;
  onListeningToggle: () => void;
  onAutoAnswerToggle: () => void;
  onSettingsToggle: () => void;
};

export function TitleBar({
  status, listening, autoAnswer, settingsOpen, securityLocked, notice, shortcutIssue,
  onCapture, onListeningToggle, onAutoAnswerToggle, onSettingsToggle,
}: TitleBarProps) {
  const { t } = useTranslation();
  return <nav className="command-bar" data-tauri-drag-region onMouseDown={(event) => {
    if (event.button === 0 && !(event.target as HTMLElement).closest("button")) {
      void getCurrentWindow().startDragging();
    }
  }}>
    <div className={`status-dot ${status} ${listening ? "live" : ""}`} />
    <button className="action-button" title={`${t("shell.captureRegion")} ${appPlatform.shortcutModifier}S`} onClick={onCapture}><span>⌗ {t("shell.capture")}</span><kbd>S</kbd></button>
    <span className="divider" />
    <button className={`action-button ${listening ? "active" : ""}`} title={`${appPlatform.shortcutModifier}L ${t("shell.startStopListening")}`}
      disabled={securityLocked} onClick={onListeningToggle}><span>{listening ? `■ ${t("shell.stop")}` : `◉ ${t("shell.listen")}`}</span><kbd>L</kbd></button>
    <button className={`action-button answer-mode ${autoAnswer ? "active" : ""}`} title={`${appPlatform.shortcutModifier}A ${t("shell.toggleAutoAnswer")}`}
      disabled={securityLocked} onClick={onAutoAnswerToggle}><span>⚡ {t("shell.autoAnswer")}</span><kbd>A</kbd></button>
    <span className="bar-spacer" data-tauri-drag-region />
    <span className="notice" title={shortcutIssue || notice}>{notice}</span>
    <span className="window-controls-divider" />
    <div className="window-controls">
      <button className={`window-control ${settingsOpen ? "active" : ""}`} aria-label={t("shell.settings")} title={t("shell.settings")}
        onClick={onSettingsToggle}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h10m4 0h2M4 17h2m4 0h10" /><circle cx="16" cy="7" r="2" /><circle cx="8" cy="17" r="2" /></svg>
      </button>
      <button className="window-control" aria-label={t("shell.hide")} title={`${t("shell.hide")} ${appPlatform.shortcutModifier}Space`} onClick={() => void getCurrentWindow().hide()}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
      </button>
      <button className="window-control close-button" aria-label={t("shell.closeApp")} title={`${t("shell.closeApp")} ${appPlatform.quitShortcut}`} onClick={() => void backend.quitApp()}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
      </button>
    </div>
  </nav>;
}
