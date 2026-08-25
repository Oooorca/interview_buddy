import { invoke } from "@tauri-apps/api/core";
import i18n from "../i18n";
import type {
  ApiKeyUpdate,
  AppSettings,
  AskResult,
  AudioOutputDevice,
  ConversationMessage,
  SecurityResetResult,
  SettingsLoadResult,
  SettingsSnapshot,
  StorageInfo,
  SupportedLanguage,
  WindowSizeInfo,
  WindowSizePreset,
} from "../shared/types";

type CommandErrorPayload = {
  code: string;
  detail?: string;
};

function commandErrorPayload(error: unknown): CommandErrorPayload | null {
  if (typeof error === "object" && error !== null && "code" in error) {
    const value = error as Record<string, unknown>;
    return {
      code: String(value.code),
      detail: typeof value.detail === "string" ? value.detail : undefined,
    };
  }
  if (typeof error === "string" && error.startsWith("{")) {
    try {
      return commandErrorPayload(JSON.parse(error));
    } catch {
      return null;
    }
  }
  return null;
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  const payload = commandErrorPayload(error);
  if (!payload) return typeof error === "string" ? error : i18n.t("errors.unknown");
  const message = i18n.t(`backend.${payload.code}`, {
    defaultValue: i18n.t("errors.unknown"),
  });
  return payload.detail ? `${message}: ${payload.detail}` : message;
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (!("__TAURI_INTERNALS__" in window)) {
    throw new Error(i18n.t("backend.appOnly"));
  }
  try {
    return await invoke<T>(name, args);
  } catch (error) {
    throw new Error(errorMessage(error));
  }
}

export const backend = {
  loadSettings: () => command<SettingsLoadResult>("load_settings"),
  saveSettings: (settings: AppSettings, apiKeyUpdate: ApiKeyUpdate = { action: "keep" }) =>
    command<SettingsSnapshot>("save_settings", { request: { settings, apiKeyUpdate } }),
  resetSecureSettings: () => command<SecurityResetResult>("reset_secure_settings"),
  shortcutWarnings: () => command<string[]>("shortcut_warnings"),
  storageInfo: () => command<StorageInfo>("storage_info"),
  setStorageRoot: (path: string) => command<StorageInfo>("set_storage_root", { path }),
  scheduleSafeCleanup: () => command<StorageInfo>("schedule_safe_cleanup"),
  windowSizeInfo: () => command<WindowSizeInfo>("window_size_info"),
  applyWindowSize: (preset: WindowSizePreset, customWidth: number, customHeight: number) =>
    command<WindowSizeInfo>("apply_window_size", {
      request: { preset, customWidth, customHeight },
    }),
  rememberWindowSize: () => command<WindowSizeInfo>("remember_window_size"),
  openRegionSelector: () => command<void>("open_region_selector"),
  completeRegionSelection: (selection: { x: number; y: number; width: number; height: number }) =>
    command<void>("complete_region_selection", { selection }),
  cancelRegionSelection: () => command<void>("cancel_region_selection"),
  quitApp: () => command<void>("quit_app"),
  ask: (requestId: string, prompt: string, imageDataUrls: string[] = [], history: ConversationMessage[] = [], answerLocale: SupportedLanguage = "zh-CN") =>
    command<AskResult>("ask_llm", {
      request: { requestId, prompt, imageDataUrls, history, answerLocale },
    }),
  cancelAnswer: (requestId: string) => command<void>("cancel_llm", { requestId }),
  transcribe: (bytes: number[], mimeType: string) =>
    command<string>("transcribe_audio", { bytes, mimeType }),
  listSystemAudioDevices: () => command<AudioOutputDevice[]>("list_system_audio_devices"),
  startSystemAudio: () => command<void>("start_system_audio"),
  systemAudioLevel: () => command<number>("system_audio_level"),
  discardSystemAudioChunk: () => command<void>("discard_system_audio_chunk"),
  transcribeSystemAudioChunk: () => command<string>("transcribe_system_audio_chunk"),
  stopSystemAudio: () => command<string>("stop_system_audio_and_transcribe"),
};
