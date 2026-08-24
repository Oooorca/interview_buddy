type ContextPanelProps = {
  value: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onValueChange: (value: string) => void;
  onSave: () => void;
};

export function ContextPanel({ value, open, onOpenChange, onValueChange, onSave }: ContextPanelProps) {
  return <section className={`context-section fixed-context ${open ? "expanded" : ""}`}>
    <button className="context-section-heading collapsible" onClick={() => onOpenChange(!open)}>
      <span><b>固定背景</b><em>{value.trim() ? "已保存" : "可选"}</em></span>
      <i>{open ? "−" : "+"}</i>
    </button>
    {open && <textarea className="fixed-context-input" rows={4} value={value}
      onChange={(event) => onValueChange(event.target.value)} onBlur={onSave}
      placeholder="简历、岗位要求、项目背景等。自动保存，并在每次回答时使用。" />}
  </section>;
}
