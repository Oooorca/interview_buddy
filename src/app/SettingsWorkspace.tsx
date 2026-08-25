import { useEffect } from "react";
import { ApiSettingsPage } from "../features/settings/ApiSettingsPage";
import { GeneralSettingsPage } from "../features/settings/GeneralSettingsPage";
import { PromptSettingsPage } from "../features/settings/PromptSettingsPage";
import { SecurityRecovery } from "../features/settings/SecurityRecovery";
import { SettingsDialog } from "../features/settings/SettingsDialog";
import type { useAppSettings } from "../features/settings/useAppSettings";
import type { useStorageSettings } from "../features/settings/useStorageSettings";
import type { useAudioDevices } from "../features/listening/useAudioDevices";
import type { useWindowSizing } from "../features/window/useWindowSizing";

type SettingsWorkspaceProps = {
  controller: ReturnType<typeof useAppSettings>;
  storage: ReturnType<typeof useStorageSettings>;
  audioDevices: ReturnType<typeof useAudioDevices>;
  windowSizing: ReturnType<typeof useWindowSizing>;
  showError: (label: string, error: unknown) => void;
};

export function SettingsWorkspace({
  controller, storage, audioDevices, windowSizing, showError,
}: SettingsWorkspaceProps) {
  useEffect(() => {
    if (controller.settingsPage !== "general") return;
    void storage.refresh();
    void windowSizing.refresh();
  }, [controller.settingsPage]);

  return <SettingsDialog page={controller.settingsPage} locked={Boolean(controller.securityIssue)}
    onPageChange={controller.setSettingsPage} onClose={() => controller.setSettingsOpen(false)}
    onSave={() => void controller.save(showError)}>
      {controller.securityIssue
        ? <SecurityRecovery message={controller.securityIssue.message}
            resetting={controller.securityResetting}
            onReset={() => void controller.resetSecureSettings(windowSizing.applySettings)} />
        : controller.settingsPage === "general"
          ? <GeneralSettingsPage settings={controller.settings}
              microphoneDevices={audioDevices.microphoneDevices}
              outputDevices={audioDevices.outputDevices}
              devicesLoading={audioDevices.loading} deviceIssue={audioDevices.issue}
              storageInfo={storage.info} storageLoading={storage.loading} storageIssue={storage.issue}
              windowSizeInfo={windowSizing.info} windowSizeLoading={windowSizing.loading}
              windowSizeIssue={windowSizing.issue}
              onSettingsChange={controller.updateGeneral}
              onWindowPresetChange={(preset) => void windowSizing.applyPreset(preset)}
              onRefreshAudio={() => void audioDevices.refresh(true)}
              onRefreshStorage={() => void storage.refresh()}
              onChooseStorageRoot={() => void storage.chooseRoot()}
              onRestoreStorageRoot={() => void storage.restoreDefault()}
              onScheduleCleanup={() => void storage.scheduleCleanup()} />
          : controller.settingsPage === "api"
            ? <ApiSettingsPage settings={controller.settings}
                apiKeyConfigured={controller.apiKeyConfigured}
                apiKeyDraft={controller.apiKeyDraft}
                apiKeyPendingClear={controller.apiKeyUpdate.action === "clear"}
                onSettingsChange={controller.setSettings}
                onApiKeyDraftChange={controller.updateApiKeyDraft}
                onApiKeyClear={controller.clearApiKey} />
            : <PromptSettingsPage settings={controller.settings} issue={controller.promptIssue}
                onModeChange={controller.updatePromptMode}
                onValueChange={controller.updatePromptValue} />}
    </SettingsDialog>;
}
