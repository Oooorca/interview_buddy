import { useCallback, useRef, useState, type Dispatch, type SetStateAction } from "react";
import type { AppSettings } from "../../shared/types";

export function normalizedSettingsForSave(settings: AppSettings): AppSettings {
  return {
    ...settings,
    systemPrompt: settings.systemPromptMode === "custom" ? settings.systemPrompt : null,
    codingPrompt: settings.codingPromptMode === "custom" ? settings.codingPrompt : null,
  };
}

export function useSettings(initial: AppSettings) {
  const [settings, setSettingsState] = useState(initial);
  const settingsRef = useRef(settings);
  const setSettings: Dispatch<SetStateAction<AppSettings>> = useCallback((update) => {
    setSettingsState((current) => {
      const next = typeof update === "function" ? update(current) : update;
      settingsRef.current = next;
      return next;
    });
  }, []);
  return { settings, settingsRef, setSettings };
}
