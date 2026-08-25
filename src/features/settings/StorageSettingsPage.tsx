import type { AppSettings, StorageInfo } from "../../shared/types";
import { useTranslation } from "react-i18next";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export type StorageSettingsPageProps = {
  settings: AppSettings;
  info: StorageInfo | null;
  loading: boolean;
  issue: string;
  onSettingsChange: (settings: AppSettings) => void;
  onRefresh: () => void;
  onChooseRoot: () => void;
  onRestoreDefault: () => void;
  onScheduleCleanup: () => void;
};

export function StorageSettingsPage(props: StorageSettingsPageProps) {
  const { t } = useTranslation();
  const { settings, info, loading, issue } = props;
  return <div className="settings-page storage-page">
    <div className="storage-page-heading">
      <div><strong>{t("storage.title")}</strong><span>{t("storage.description")}</span></div>
      <button className="refresh-devices" disabled={loading} onClick={props.onRefresh}>{loading ? t("audio.loading") : t("storage.refreshCapacity")}</button>
    </div>
    {issue && <div className="device-issue">{issue}</div>}
    {info && <>
      <section className="storage-location-card">
        <header><div><b>{t("storage.currentRoot")}</b><span>{info.isDefault ? t("storage.defaultLocation") : t("storage.customLocation")}</span></div>
          <i className={info.isDefault ? "default" : "custom"}>{info.isDefault ? t("storage.defaultBadge") : t("storage.customBadge")}</i></header>
        <div className="storage-path-row">
          <input readOnly value={info.dataRoot} title={info.dataRoot} />
          <div><button disabled={loading || info.restartRequired} onClick={props.onChooseRoot}>{t("storage.choose")}</button>
            <button disabled={loading || info.restartRequired || info.isDefault} onClick={props.onRestoreDefault}>{t("storage.restore")}</button></div>
        </div>
        <div className="storage-subpath"><span>WebView2</span><code>{info.webviewDataRoot}</code></div>
        {info.restartRequired && <div className="storage-restart">{t("storage.restart")}</div>}
      </section>
      <div className="storage-metrics">
        <div><span>{t("storage.total")}</span><strong>{formatBytes(info.totalBytes)}</strong></div>
        <div><span>{t("storage.safeToClean")}</span><strong>{formatBytes(info.safeCacheBytes)}</strong></div>
      </div>
      <section className="storage-cleanup-card">
        <div><b>{t("storage.automatic")}</b><span>{t("storage.automaticDescription")}</span></div>
        <label className="toggle-setting"><input type="checkbox" checked={settings.autoSafeCleanup}
          onChange={(event) => props.onSettingsChange({ ...settings, autoSafeCleanup: event.target.checked })} />
          <i />{settings.autoSafeCleanup ? t("storage.enabled") : t("storage.disabled")}</label>
      </section>
      <div className="storage-cleanup-actions">
        <button disabled={loading || info.cleanupPending} onClick={props.onScheduleCleanup}>
          {info.cleanupPending ? t("storage.cleanupPending") : t("storage.cleanupOnce")}
        </button>
        <p>{t("storage.cleanupSafety")}</p>
      </div>
    </>}
    {!info && !issue && <div className="storage-loading">{t("storage.loading")}</div>}
  </div>;
}
