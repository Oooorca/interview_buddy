type ContextPanelProps = {
  value: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onValueChange: (value: string) => void;
  onSave: () => void;
};

export function ContextPanel({ value, open, onOpenChange, onValueChange, onSave }: ContextPanelProps) {
  const { t } = useTranslation();
  return <section className={`context-section fixed-context ${open ? "expanded" : ""}`}>
    <button className="context-section-heading collapsible" onClick={() => onOpenChange(!open)}>
      <span><b>{t("context.fixed")}</b><em>{value.trim() ? t("context.saved") : t("context.optional")}</em></span>
      <i>{open ? "−" : "+"}</i>
    </button>
    {open && <textarea className="fixed-context-input" rows={4} value={value}
      onChange={(event) => onValueChange(event.target.value)} onBlur={onSave}
      placeholder={t("context.placeholder")} />}
  </section>;
}
import { useTranslation } from "react-i18next";
