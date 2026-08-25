import { useState, type MutableRefObject } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { backend, errorMessage } from "../../services/backend";
import type { AppSettings, StorageInfo } from "../../shared/types";
import { normalizedSettingsForSave } from "./useSettings";

type UseStorageSettingsOptions = {
  settingsRef: MutableRefObject<AppSettings>;
  setNotice: (notice: string) => void;
};

export function useStorageSettings({ settingsRef, setNotice }: UseStorageSettingsOptions) {
  const { t } = useTranslation();
  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [issue, setIssue] = useState("");

  async function refresh() {
    setLoading(true);
    setIssue("");
    try { setInfo(await backend.storageInfo()); }
    catch (error) { setIssue(errorMessage(error)); }
    finally { setLoading(false); }
  }

  async function applyRoot(path: string) {
    setLoading(true);
    await backend.saveSettings(normalizedSettingsForSave(settingsRef.current));
    const nextInfo = await backend.setStorageRoot(path);
    setInfo(nextInfo);
    setNotice(t(nextInfo.restartRequired ? "notices.storageRestart" : "notices.storageUnchanged"));
  }

  async function chooseRoot() {
    setIssue("");
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t("storage.pickerTitle"),
        defaultPath: info?.dataRoot,
      });
      if (typeof selected === "string") await applyRoot(selected);
    } catch (error) { setIssue(errorMessage(error)); }
    finally { setLoading(false); }
  }

  async function restoreDefault() {
    if (!info) return;
    setIssue("");
    try { await applyRoot(info.defaultDataRoot); }
    catch (error) { setIssue(errorMessage(error)); }
    finally { setLoading(false); }
  }

  async function scheduleCleanup() {
    setLoading(true);
    setIssue("");
    try {
      setInfo(await backend.scheduleSafeCleanup());
      setNotice(t("notices.cleanupScheduled"));
    } catch (error) { setIssue(errorMessage(error)); }
    finally { setLoading(false); }
  }

  return { info, loading, issue, refresh, chooseRoot, restoreDefault, scheduleCleanup };
}
