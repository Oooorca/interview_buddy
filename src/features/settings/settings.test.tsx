import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ApiKeyField } from "./ApiKeyField";
import { PromptEditor } from "./PromptEditor";
import { SecurityRecovery } from "./SecurityRecovery";
import { GeneralSettingsPage } from "./GeneralSettingsPage";
import { SettingsDialog } from "./SettingsDialog";
import { defaultSettings } from "../../shared/settings";

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

  it("uses full locale identifiers for interface and answer language changes", () => {
    const onSettingsChange = vi.fn();
    render(<GeneralSettingsPage settings={defaultSettings} onSettingsChange={onSettingsChange}
      microphoneDevices={[]} outputDevices={[]} devicesLoading={false} deviceIssue=""
      storageInfo={null} storageLoading={false} storageIssue=""
      windowSizeInfo={{ preset: "standard", width: 800, height: 480, monitorWidth: 1920,
        monitorHeight: 1040, scaleFactor: 1 }} windowSizeLoading={false} windowSizeIssue=""
      onRefreshAudio={vi.fn()} onRefreshStorage={vi.fn()} onChooseStorageRoot={vi.fn()}
      onRestoreStorageRoot={vi.fn()} onScheduleCleanup={vi.fn()} onWindowPresetChange={vi.fn()} />);
    const [uiLanguage, answerLanguage] = screen.getAllByRole("combobox") as HTMLSelectElement[];

    fireEvent.change(uiLanguage, { target: { value: "en-US" } });
    expect(onSettingsChange).toHaveBeenCalledWith(expect.objectContaining({ uiLanguage: "en-US" }));

    fireEvent.change(answerLanguage, { target: { value: "en-US" } });
    expect(onSettingsChange).toHaveBeenCalledWith(expect.objectContaining({ answerLanguage: "en-US" }));
    expect([...uiLanguage.options].map((option) => option.value)).not.toContain("en");
  });

  it("offers responsive window presets and marks Standard as recommended", () => {
    const onWindowPresetChange = vi.fn();
    render(<GeneralSettingsPage settings={defaultSettings} onSettingsChange={vi.fn()}
      microphoneDevices={[]} outputDevices={[]} devicesLoading={false} deviceIssue=""
      storageInfo={null} storageLoading={false} storageIssue=""
      windowSizeInfo={{ preset: "standard", width: 800, height: 480, monitorWidth: 1920,
        monitorHeight: 1040, scaleFactor: 1.25 }} windowSizeLoading={false} windowSizeIssue=""
      onRefreshAudio={vi.fn()} onRefreshStorage={vi.fn()} onChooseStorageRoot={vi.fn()}
      onRestoreStorageRoot={vi.fn()} onScheduleCleanup={vi.fn()}
      onWindowPresetChange={onWindowPresetChange} />);

    expect(screen.getByText("推荐")).toBeInTheDocument();
    expect(screen.getByText("800 × 480")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: /宽敞/ }));
    expect(onWindowPresetChange).toHaveBeenCalledWith("spacious");
  });

  it("uses the consolidated General, API Settings, and Prompt navigation", () => {
    const onPageChange = vi.fn();
    render(<SettingsDialog page="general" locked={false} onPageChange={onPageChange}
      onClose={vi.fn()} onSave={vi.fn()}><div>content</div></SettingsDialog>);

    expect(screen.getByRole("button", { name: "通用" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "API 设置" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Prompt" }));
    expect(onPageChange).toHaveBeenCalledWith("prompt");
    expect(screen.queryByRole("button", { name: "音频" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "存储与清理" })).not.toBeInTheDocument();
  });
});
