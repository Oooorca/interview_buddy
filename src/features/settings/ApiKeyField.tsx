type ApiKeyFieldProps = {
  configured: boolean;
  draft: string;
  pendingClear: boolean;
  onDraftChange: (value: string) => void;
  onClear: () => void;
};

export function ApiKeyField({ configured, draft, pendingClear, onDraftChange, onClear }: ApiKeyFieldProps) {
  return <label>API Key<div className="api-key-field">
    <input type="password" autoComplete="off" value={draft}
      placeholder={configured && !pendingClear ? "已保存，不会回显" : "输入 API Key"}
      onChange={(event) => onDraftChange(event.target.value)} />
    <button type="button" disabled={!configured && !draft}
      onClick={onClear}>
      {pendingClear ? "待清除" : "清除"}
    </button>
  </div></label>;
}
