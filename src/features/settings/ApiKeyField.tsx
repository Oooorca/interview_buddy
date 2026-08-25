type ApiKeyFieldProps = {
  configured: boolean;
  draft: string;
  pendingClear: boolean;
  onDraftChange: (value: string) => void;
  onClear: () => void;
};

export function ApiKeyField({ configured, draft, pendingClear, onDraftChange, onClear }: ApiKeyFieldProps) {
  const { t } = useTranslation();
  return <label>API Key<div className="api-key-field">
    <input type="password" autoComplete="off" value={draft}
      placeholder={configured && !pendingClear ? t("api.savedKey") : t("api.enterKey")}
      onChange={(event) => onDraftChange(event.target.value)} />
    <button type="button" disabled={!configured && !draft}
      onClick={onClear}>
      {pendingClear ? t("api.pendingClear") : t("api.clearKey")}
    </button>
  </div></label>;
}
import { useTranslation } from "react-i18next";
