import type { PromptMode } from "../../shared/types";

type PromptEditorProps = {
  title: string;
  description: string;
  mode: PromptMode;
  value: string | null;
  defaultValue: string;
  rows: number;
  onModeChange: (mode: PromptMode) => void;
  onValueChange: (value: string) => void;
};

export function PromptEditor({
  title,
  description,
  mode,
  value,
  defaultValue,
  rows,
  onModeChange,
  onValueChange,
}: PromptEditorProps) {
  const visibleValue = mode === "default" ? defaultValue : value || "";
  return <section className={`prompt-card ${mode}`}>
    <header className="prompt-card-heading">
      <div><b>{title}</b><span>{description}</span></div>
      <select aria-label={`${title}模式`} value={mode}
        onChange={(event) => onModeChange(event.target.value as PromptMode)}>
        <option value="default">推荐默认</option>
        <option value="custom">自定义</option>
        <option value="disabled">禁用</option>
      </select>
    </header>
    {mode === "disabled" ? <div className="prompt-disabled-warning">
      已明确禁用。请求将不携带此 Prompt，回答质量和格式稳定性可能下降。
    </div> : <>
      <textarea rows={rows} value={visibleValue} readOnly={mode === "default"}
        onChange={(event) => onValueChange(event.target.value)} />
      <footer className="prompt-card-footer">
        <span>{mode === "default" ? "随应用升级自动使用最新内置版本" : `${visibleValue.length} 个字符`}</span>
        {mode === "default"
          ? <button onClick={() => onModeChange("custom")}>复制为自定义</button>
          : <button onClick={() => onModeChange("default")}>恢复推荐默认</button>}
      </footer>
    </>}
  </section>;
}
