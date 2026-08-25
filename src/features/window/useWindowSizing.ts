import { useEffect, useRef, useState, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { backend, errorMessage } from "../../services/backend";
import type { AppSettings, WindowSizeInfo, WindowSizePreset } from "../../shared/types";

type UseWindowSizingOptions = {
  isTauri: boolean;
  settingsRef: MutableRefObject<AppSettings>;
  setSettings: Dispatch<SetStateAction<AppSettings>>;
};

export function useWindowSizing({ isTauri, settingsRef, setSettings }: UseWindowSizingOptions) {
  const [info, setInfo] = useState<WindowSizeInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [issue, setIssue] = useState("");
  const eventTimerRef = useRef<number | null>(null);
  const programmaticResizeRef = useRef(false);
  const programmaticTimerRef = useRef<number | null>(null);

  function suppressProgrammaticResize() {
    programmaticResizeRef.current = true;
    if (programmaticTimerRef.current) globalThis.clearTimeout(programmaticTimerRef.current);
    programmaticTimerRef.current = globalThis.setTimeout(() => {
      programmaticResizeRef.current = false;
    }, 900);
  }

  useEffect(() => {
    if (!isTauri) return;
    let disposed = false;
    const appWindow = getCurrentWindow();
    const clearEventTimer = () => {
      if (eventTimerRef.current) globalThis.clearTimeout(eventTimerRef.current);
      eventTimerRef.current = null;
    };
    const listeners = Promise.all([
      appWindow.onResized(() => {
        if (programmaticResizeRef.current) return;
        clearEventTimer();
        eventTimerRef.current = globalThis.setTimeout(() => {
          void backend.rememberWindowSize().then((nextInfo) => {
            if (disposed) return;
            setInfo(nextInfo);
            setSettings((current) => ({
              ...current,
              windowSizePreset: "custom",
              customWindowWidth: nextInfo.width,
              customWindowHeight: nextInfo.height,
            }));
          }).catch(() => undefined);
        }, 550);
      }),
      appWindow.onMoved(() => {
        suppressProgrammaticResize();
        clearEventTimer();
        eventTimerRef.current = globalThis.setTimeout(() => {
          const current = settingsRef.current;
          void backend.applyWindowSize(
            current.windowSizePreset,
            current.customWindowWidth,
            current.customWindowHeight,
          ).then((nextInfo) => { if (!disposed) setInfo(nextInfo); })
            .catch(() => undefined);
        }, 450);
      }),
    ]);
    return () => {
      disposed = true;
      clearEventTimer();
      if (programmaticTimerRef.current) globalThis.clearTimeout(programmaticTimerRef.current);
      void listeners.then((unlisten) => unlisten.forEach((dispose) => dispose()));
    };
  }, [isTauri, setSettings, settingsRef]);

  async function refresh() {
    setLoading(true);
    setIssue("");
    try { setInfo(await backend.windowSizeInfo()); }
    catch (error) { setIssue(errorMessage(error)); }
    finally { setLoading(false); }
  }

  async function applyPreset(preset: WindowSizePreset) {
    const previous = settingsRef.current;
    const customWidth = preset === "custom" ? info?.width ?? previous.customWindowWidth : previous.customWindowWidth;
    const customHeight = preset === "custom" ? info?.height ?? previous.customWindowHeight : previous.customWindowHeight;
    setSettings({
      ...previous,
      windowSizePreset: preset,
      customWindowWidth: customWidth,
      customWindowHeight: customHeight,
    });
    setLoading(true);
    setIssue("");
    suppressProgrammaticResize();
    try {
      const nextInfo = await backend.applyWindowSize(preset, customWidth, customHeight);
      setInfo(nextInfo);
      if (preset === "custom") {
        setSettings((current) => ({
          ...current,
          customWindowWidth: nextInfo.width,
          customWindowHeight: nextInfo.height,
        }));
      }
    } catch (error) {
      setSettings(previous);
      setIssue(errorMessage(error));
    } finally {
      setLoading(false);
    }
  }

  function applySettings(settings: AppSettings) {
    suppressProgrammaticResize();
    void backend.applyWindowSize(
      settings.windowSizePreset,
      settings.customWindowWidth,
      settings.customWindowHeight,
    ).then(setInfo).catch(() => undefined);
  }

  return { info, loading, issue, refresh, applyPreset, applySettings };
}
