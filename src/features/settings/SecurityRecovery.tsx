type SecurityRecoveryProps = {
  message: string;
  resetting: boolean;
  onReset: () => void;
};

export function SecurityRecovery({ message, resetting, onReset }: SecurityRecoveryProps) {
  return <section className="security-recovery" role="alert">
    <div className="security-recovery-icon">!</div>
    <div>
      <strong>加密设置已锁定</strong>
      <p>{message}</p>
      <p>应用不会覆盖现有文件。重置时会先把能够定位的旧设置移入恢复目录。</p>
      <button className="danger" disabled={resetting} onClick={onReset}>
        {resetting ? "正在重置…" : "保留旧文件并重置设置"}
      </button>
    </div>
  </section>;
}
