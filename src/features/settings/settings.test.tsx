import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ApiKeyField } from "./ApiKeyField";
import { PromptEditor } from "./PromptEditor";
import { SecurityRecovery } from "./SecurityRecovery";

describe("secure settings controls", () => {
  it("shows only the saved state and never receives a stored API key", () => {
    const onDraftChange = vi.fn();
    render(<ApiKeyField configured draft="" pendingClear={false}
      onDraftChange={onDraftChange} onClear={vi.fn()} />);
    const input = screen.getByLabelText("API Key") as HTMLInputElement;
    expect(input).toHaveValue("");
    expect(input).toHaveAttribute("placeholder", "已保存，不会回显");
    fireEvent.change(input, { target: { value: "replacement-secret" } });
    expect(onDraftChange).toHaveBeenCalledWith("replacement-secret");
  });

  it("keeps the recommended prompt read-only and supports explicit custom mode", () => {
    const onModeChange = vi.fn();
    render(<PromptEditor title="系统 Prompt" description="desc" mode="default" value={null}
      defaultValue="recommended" rows={3} onModeChange={onModeChange} onValueChange={vi.fn()} />);
    expect(screen.getByDisplayValue("recommended")).toHaveAttribute("readonly");
    fireEvent.click(screen.getByRole("button", { name: "复制为自定义" }));
    expect(onModeChange).toHaveBeenCalledWith("custom");
  });

  it("requires an explicit recovery action in locked mode", () => {
    const onReset = vi.fn();
    render(<SecurityRecovery message="authentication failed" resetting={false} onReset={onReset} />);
    expect(screen.getByRole("alert")).toHaveTextContent("authentication failed");
    fireEvent.click(screen.getByRole("button", { name: "保留旧文件并重置设置" }));
    expect(onReset).toHaveBeenCalledOnce();
  });
});
