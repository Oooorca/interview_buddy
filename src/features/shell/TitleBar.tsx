import { getCurrentWindow } from "@tauri-apps/api/window";
import { backend } from "../../services/backend";

type TitleBarProps = {
  status: "ready" | "working" | "error";
  listening: boolean;
  autoAnswer: boolean;
  settingsOpen: boolean;
  securityLocked: boolean;
  notice: string;
  shortcutIssue: string;
  modifier: string;
  isMac: boolean;
  onCapture: () => void;
  onListeningToggle: () => void;
  onAutoAnswerToggle: () => void;
  onSettingsToggle: () => void;
};

export function TitleBar({
  status, listening, autoAnswer, settingsOpen, securityLocked, notice, shortcutIssue, modifier, isMac,
  onCapture, onListeningToggle, onAutoAnswerToggle, onSettingsToggle,
}: TitleBarProps) {
  return <nav className="command-bar" data-tauri-drag-region onMouseDown={(event) => {
    if (event.button === 0 && !(event.target as HTMLElement).closest("button")) {
      void getCurrentWindow().startDragging();
    }
  }}>
    <div className={`status-dot ${status} ${listening ? "live" : ""}`} />
    <button className="action-button" title={`区域截图 ${modifier}S`} onClick={onCapture}><span>⌗ 截图</span><kbd>S</kbd></button>
    <span className="divider" />
    <button className={`action-button ${listening ? "active" : ""}`} title={`${modifier}L 开始/停止听写`}
      disabled={securityLocked} onClick={onListeningToggle}><span>{listening ? "■ 停止" : "◉ 听写"}</span><kbd>L</kbd></button>
    <button className={`action-button answer-mode ${autoAnswer ? "active" : ""}`} title={`${modifier}A 开启/关闭自动回答`}
      disabled={securityLocked} onClick={onAutoAnswerToggle}><span>⚡ 自动答</span><kbd>A</kbd></button>
    <span className="bar-spacer" data-tauri-drag-region />
    <span className="notice" title={shortcutIssue || notice}>{notice}</span>
    <span className="window-controls-divider" />
    <div className="window-controls">
      <button className={`window-control ${settingsOpen ? "active" : ""}`} aria-label="设置" title="设置"
        onClick={onSettingsToggle}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h10m4 0h2M4 17h2m4 0h10" /><circle cx="16" cy="7" r="2" /><circle cx="8" cy="17" r="2" /></svg>
      </button>
      <button className="window-control" aria-label="隐藏" title={`隐藏 ${modifier}Space`} onClick={() => void getCurrentWindow().hide()}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
      </button>
      <button className="window-control close-button" aria-label="关闭应用" title={`关闭应用 ${isMac ? "⌘Q" : "Ctrl+Q"}`} onClick={() => void backend.quitApp()}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
      </button>
    </div>
  </nav>;
}
