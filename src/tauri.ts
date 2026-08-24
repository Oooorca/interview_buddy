import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, AskResult, AudioOutputDevice, ConversationMessage, StorageInfo } from "./types";

export const backend = {
  loadSettings: () => invoke<AppSettings>("load_settings"),
  saveSettings: (settings: AppSettings) =>
    invoke<void>("save_settings", { settings }),
  shortcutWarnings: () => invoke<string[]>("shortcut_warnings"),
  storageInfo: () => invoke<StorageInfo>("storage_info"),
  setStorageRoot: (path: string) => invoke<StorageInfo>("set_storage_root", { path }),
  scheduleSafeCleanup: () => invoke<StorageInfo>("schedule_safe_cleanup"),
  openRegionSelector: () => invoke<void>("open_region_selector"),
  completeRegionSelection: (selection: { x: number; y: number; width: number; height: number }) =>
    invoke<void>("complete_region_selection", { selection }),
  cancelRegionSelection: () => invoke<void>("cancel_region_selection"),
  quitApp: () => invoke<void>("quit_app"),
  ask: (requestId: string, prompt: string, imageDataUrls: string[] = [], history: ConversationMessage[] = []) =>
    invoke<AskResult>("ask_llm", {
      request: { requestId, prompt, imageDataUrls, history },
    }),
  cancelAnswer: (requestId: string) => invoke<void>("cancel_llm", { requestId }),
  transcribe: (bytes: number[], mimeType: string) =>
    invoke<string>("transcribe_audio", { bytes, mimeType }),
  listSystemAudioDevices: () => invoke<AudioOutputDevice[]>("list_system_audio_devices"),
  startSystemAudio: () => invoke<void>("start_system_audio"),
  systemAudioLevel: () => invoke<number>("system_audio_level"),
  discardSystemAudioChunk: () => invoke<void>("discard_system_audio_chunk"),
  transcribeSystemAudioChunk: () => invoke<string>("transcribe_system_audio_chunk"),
  stopSystemAudio: () => invoke<string>("stop_system_audio_and_transcribe"),
};
