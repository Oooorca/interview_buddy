import type { ReactNode } from "react";

export type SettingsPage = "api" | "audio" | "storage";

type SettingsDialogProps = {
  page: SettingsPage;
  locked: boolean;
  children: ReactNode;
  onPageChange: (page: SettingsPage) => void;
  onClose: () => void;
  onSave: () => void;
};

export function SettingsDialog({ page, locked, children, onPageChange, onClose, onSave }: SettingsDialogProps) {
  return <section className="settings-panel">
    <div className="settings-title"><strong>设置</strong><button aria-label="关闭设置" onClick={onClose}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
    </button></div>
    <nav className="settings-tabs">
      <button className={page === "api" ? "active" : ""} onClick={() => onPageChange("api")}>API 与 Prompt</button>
      <button className={page === "audio" ? "active" : ""} onClick={() => onPageChange("audio")}>音频</button>
      <button className={page === "storage" ? "active" : ""} onClick={() => onPageChange("storage")}>存储与清理</button>
    </nav>
    <div className="settings-content">{children}</div>
    {!locked && <div className="settings-actions"><button className="primary" onClick={onSave}>保存设置</button></div>}
  </section>;
}
