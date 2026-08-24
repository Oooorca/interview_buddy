import type { AppSettings, StorageInfo } from "../../shared/types";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

type StorageSettingsPageProps = {
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
  const { settings, info, loading, issue } = props;
  return <div className="settings-page storage-page">
    <div className="storage-page-heading">
      <div><strong>数据与缓存目录</strong><span>设置、WebView2 数据及以后产生的持久化内容统一保存在这里</span></div>
      <button className="refresh-devices" disabled={loading} onClick={props.onRefresh}>{loading ? "读取中…" : "刷新容量"}</button>
    </div>
    {issue && <div className="device-issue">{issue}</div>}
    {info && <>
      <section className="storage-location-card">
        <header><div><b>当前存储目录</b><span>{info.isDefault ? "默认：系统应用数据目录" : "自定义位置"}</span></div>
          <i className={info.isDefault ? "default" : "custom"}>{info.isDefault ? "DEFAULT" : "CUSTOM"}</i></header>
        <div className="storage-path-row">
          <input readOnly value={info.dataRoot} title={info.dataRoot} />
          <div><button disabled={loading || info.restartRequired} onClick={props.onChooseRoot}>选择目录</button>
            <button disabled={loading || info.restartRequired || info.isDefault} onClick={props.onRestoreDefault}>恢复默认</button></div>
        </div>
        <div className="storage-subpath"><span>WebView2</span><code>{info.webviewDataRoot}</code></div>
        {info.restartRequired && <div className="storage-restart">目录已更新。请关闭并重新打开应用，WebView2 和旧数据迁移后才会使用新位置。</div>}
      </section>
      <div className="storage-metrics">
        <div><span>总占用</span><strong>{formatBytes(info.totalBytes)}</strong></div>
        <div><span>可安全清理</span><strong>{formatBytes(info.safeCacheBytes)}</strong></div>
      </div>
      <section className="storage-cleanup-card">
        <div><b>自动安全清理</b><span>每次启动、创建 WebView2 前清理普通缓存、GPU/Shader 缓存和崩溃报告</span></div>
        <label className="toggle-setting"><input type="checkbox" checked={settings.autoSafeCleanup}
          onChange={(event) => props.onSettingsChange({ ...settings, autoSafeCleanup: event.target.checked })} />
          <i />{settings.autoSafeCleanup ? "已开启" : "已关闭"}</label>
      </section>
      <div className="storage-cleanup-actions">
        <button disabled={loading || info.cleanupPending} onClick={props.onScheduleCleanup}>
          {info.cleanupPending ? "已安排下次启动清理" : "下次启动时安全清理一次"}
        </button>
        <p>清理不会删除加密设置、密钥、API Key、Prompt、设备选择或麦克风身份数据。</p>
      </div>
    </>}
    {!info && !issue && <div className="storage-loading">正在读取存储信息…</div>}
  </div>;
}
