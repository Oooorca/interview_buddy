import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppSettings, CaptureResult } from "./types";

export const backend = {
  loadSettings: () => invoke<AppSettings>("load_settings"),
  saveSettings: (settings: AppSettings) =>
    invoke<void>("save_settings", { settings }),
  capturePrimary: () => invoke<CaptureResult>("capture_primary_monitor"),
  markCaptureOrigin: () => invoke<string>("mark_capture_origin"),
  captureMarkedRegion: () => invoke<CaptureResult>("capture_marked_region"),
  ask: (prompt: string, imageDataUrls: string[] = []) =>
    invoke<string>("ask_llm", {
      request: { prompt, imageDataUrls },
    }),
  transcribe: (bytes: number[], mimeType: string) =>
    invoke<string>("transcribe_audio", { bytes, mimeType }),
  startSystemAudio: () => invoke<void>("start_system_audio"),
  stopSystemAudio: () => invoke<string>("stop_system_audio_and_transcribe"),
};

export async function captureWithoutOverlay(): Promise<CaptureResult> {
  const window = getCurrentWindow();
  const wasVisible = await window.isVisible();
  if (wasVisible) {
    await window.hide();
    await new Promise((resolve) => globalThis.setTimeout(resolve, 140));
  }
  try {
    return await backend.capturePrimary();
  } finally {
    if (wasVisible) await window.show();
  }
}
