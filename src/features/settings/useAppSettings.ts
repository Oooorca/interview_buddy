import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { backend, errorMessage } from "../../services/backend";
import i18n, { applyUiLanguage } from "../../i18n";
import { defaultPromptsFor, defaultSettings } from "../../shared/settings";
import type { ApiKeyUpdate, AppSettings, PromptMode, SecurityIssue } from "../../shared/types";
import type { SettingsPage } from "./SettingsDialog";
import { normalizedSettingsForSave, useSettings } from "./useSettings";

export function useAppSettings(isTauri: boolean) {
  const { t } = useTranslation();
  const { settings, settingsRef, setSettings } = useSettings(defaultSettings);
  const [apiKeyConfigured, setApiKeyConfigured] = useState(false);
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [apiKeyUpdate, setApiKeyUpdate] = useState<ApiKeyUpdate>({ action: "keep" });
  const [securityIssue, setSecurityIssue] = useState<SecurityIssue | null>(null);
  const [securityResetting, setSecurityResetting] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsPage, setSettingsPage] = useState<SettingsPage>("general");
  const [promptIssue, setPromptIssue] = useState("");
  const [notice, setNotice] = useState(() => i18n.t("notices.protected"));
  const [shortcutIssue, setShortcutIssue] = useState("");

  useEffect(() => {
    applyUiLanguage(settings.uiLanguage);
  }, [settings.uiLanguage]);

  useEffect(() => {
    if (!isTauri) return;
    backend.loadSettings().then((result) => {
      if (result.state === "locked") {
        setSecurityIssue({
          reason: result.reason,
          message: errorMessage({ code: result.reason, detail: result.message || undefined }),
        });
        setSettingsOpen(true);
        return;
      }
      setSettings(result.snapshot.settings);
      setApiKeyConfigured(result.snapshot.apiKeyConfigured);
      applyUiLanguage(result.snapshot.settings.uiLanguage);
      if (result.snapshot.securityState === "migrated") setNotice(i18n.t("notices.settingsMigrated"));
      else if (result.snapshot.securityState === "recovered") setNotice(i18n.t("notices.settingsRecovered"));
      else setNotice(i18n.t("notices.protected"));
    }).catch((error) => {
      setSecurityIssue({ reason: "load-failed", message: errorMessage(error) });
      setSettingsOpen(true);
    });
    backend.shortcutWarnings().then((warnings) => {
      if (!warnings.length) return;
      setShortcutIssue(warnings.join("; "));
      setNotice(i18n.t("notices.shortcutConflict", {
        shortcuts: warnings.map((warning) => warning.split(":")[0]).join(", "),
      }));
    }).catch(() => undefined);
  }, [isTauri, setSettings]);

  function updateGeneral(next: AppSettings) {
    const languageChanged = next.uiLanguage !== settingsRef.current.uiLanguage;
    setSettings(next);
    if (languageChanged) {
      applyUiLanguage(next.uiLanguage);
      setNotice(i18n.t("notices.protected"));
    }
  }

  function updateFixedContext(value: string) {
    setSettings((current) => ({ ...current, fixedContext: value }));
  }

  async function saveFixedContext() {
    try {
      await backend.saveSettings(normalizedSettingsForSave(settingsRef.current));
      setNotice(t("notices.backgroundSaved"));
    } catch (error) {
      setNotice(t("notices.backgroundSaveFailed", { error: errorMessage(error) }));
    }
  }

  function updatePromptMode(kind: "system" | "coding", mode: PromptMode) {
    setPromptIssue("");
    setSettings((current) => {
      const defaults = defaultPromptsFor(current);
      return kind === "system"
        ? {
            ...current,
            systemPromptMode: mode,
            systemPrompt: mode === "custom"
              ? current.systemPrompt || defaults.system
              : mode === "default" ? null : current.systemPrompt,
          }
        : {
            ...current,
            codingPromptMode: mode,
            codingPrompt: mode === "custom"
              ? current.codingPrompt || defaults.coding
              : mode === "default" ? null : current.codingPrompt,
          };
    });
  }

  function updatePromptValue(kind: "system" | "coding", value: string) {
    setPromptIssue("");
    setSettings((current) => kind === "system"
      ? { ...current, systemPrompt: value }
      : { ...current, codingPrompt: value });
  }

  function updateApiKeyDraft(value: string) {
    setApiKeyDraft(value);
    setApiKeyUpdate(value ? { action: "replace", value } : { action: "keep" });
  }

  function clearApiKey() {
    setApiKeyDraft("");
    setApiKeyUpdate({ action: "clear" });
  }

  async function resetSecureSettings(onReset: (settings: AppSettings) => void) {
    if (!window.confirm(t("security.confirmReset"))) return;
    setSecurityResetting(true);
    try {
      const result = await backend.resetSecureSettings();
      setSettings(result.snapshot.settings);
      setApiKeyConfigured(result.snapshot.apiKeyConfigured);
      setApiKeyDraft("");
      setApiKeyUpdate({ action: "keep" });
      setSecurityIssue(null);
      onReset(result.snapshot.settings);
      setNotice(result.quarantinePath
        ? t("security.oldFilesPreserved", { path: result.quarantinePath })
        : t("security.resetDone"));
    } catch (error) {
      setNotice(t("security.resetFailed", { error: errorMessage(error) }));
    } finally {
      setSecurityResetting(false);
    }
  }

  async function save(onError: (label: string, error: unknown) => void) {
    const emptyCustom = settings.systemPromptMode === "custom" && !settings.systemPrompt?.trim()
      ? t("prompt.systemTitle")
      : settings.codingPromptMode === "custom" && !settings.codingPrompt?.trim()
        ? t("prompt.codingTitle")
        : "";
    if (emptyCustom) {
      setSettingsPage("prompt");
      setPromptIssue(t("prompt.emptyCustom", { name: emptyCustom }));
      setNotice(t("prompt.incomplete"));
      return;
    }
    try {
      const snapshot = await backend.saveSettings(normalizedSettingsForSave(settings), apiKeyUpdate);
      setSettings(snapshot.settings);
      setApiKeyConfigured(snapshot.apiKeyConfigured);
      setApiKeyDraft("");
      setApiKeyUpdate({ action: "keep" });
      setPromptIssue("");
      setSettingsOpen(false);
      setNotice(t("notices.settingsSaved"));
    } catch (error) {
      setPromptIssue(errorMessage(error));
      onError(t("notices.saveFailed"), error);
    }
  }

  return {
    settings, settingsRef, setSettings,
    apiKeyConfigured, apiKeyDraft, apiKeyUpdate,
    securityIssue, securityResetting,
    settingsOpen, setSettingsOpen, settingsPage, setSettingsPage,
    promptIssue, notice, setNotice, shortcutIssue,
    updateGeneral, updateFixedContext, saveFixedContext,
    updatePromptMode, updatePromptValue,
    updateApiKeyDraft, clearApiKey, resetSecureSettings, save,
  };
}
