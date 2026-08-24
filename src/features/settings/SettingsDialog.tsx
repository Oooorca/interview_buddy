import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

export type SettingsPage = "general" | "api" | "prompt";

type SettingsDialogProps = {
  page: SettingsPage;
  locked: boolean;
  children: ReactNode;
  onPageChange: (page: SettingsPage) => void;
  onClose: () => void;
  onSave: () => void;
};

export function SettingsDialog({ page, locked, children, onPageChange, onClose, onSave }: SettingsDialogProps) {
  const { t } = useTranslation();
  return <section className="settings-panel">
    <div className="settings-title"><strong>{t("settings.title")}</strong><button aria-label={t("settings.close")} onClick={onClose}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
    </button></div>
    <nav className="settings-tabs">
      <button className={page === "general" ? "active" : ""} onClick={() => onPageChange("general")}>{t("general.tab")}</button>
      <button className={page === "api" ? "active" : ""} onClick={() => onPageChange("api")}>{t("settings.apiTab")}</button>
      <button className={page === "prompt" ? "active" : ""} onClick={() => onPageChange("prompt")}>{t("settings.promptTab")}</button>
    </nav>
    <div className="settings-content">{children}</div>
    {!locked && <div className="settings-actions"><button className="primary" onClick={onSave}>{t("actions.save")}</button></div>}
  </section>;
}
