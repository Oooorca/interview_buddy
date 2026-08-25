type SecurityRecoveryProps = {
  message: string;
  resetting: boolean;
  onReset: () => void;
};

export function SecurityRecovery({ message, resetting, onReset }: SecurityRecoveryProps) {
  const { t } = useTranslation();
  return <section className="security-recovery" role="alert">
    <div className="security-recovery-icon">!</div>
    <div>
      <strong>{t("security.locked")}</strong>
      <p>{message}</p>
      <p>{t("security.preserved")}</p>
      <button className="danger" disabled={resetting} onClick={onReset}>
        {resetting ? t("security.resetting") : t("security.reset")}
      </button>
    </div>
  </section>;
}
import { useTranslation } from "react-i18next";
