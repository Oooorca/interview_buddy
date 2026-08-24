export type PromptMode = "default" | "custom" | "disabled";

export interface AppSettings {
  baseUrl: string;
  model: string;
  visionModel: string;
  transcriptionModel: string;
  captureMicrophone: boolean;
  captureSystemAudio: boolean;
  microphoneDeviceId: string;
  systemAudioDeviceId: string;
  myTranscriptionLanguage: string;
  theirTranscriptionLanguage: string;
  autoSafeCleanup: boolean;
  fixedContext: string;
  systemPromptMode: PromptMode;
  codingPromptMode: PromptMode;
  systemPrompt: string | null;
  codingPrompt: string | null;
}

export interface CaptureResult {
  dataUrl: string;
}

export interface AudioOutputDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface StorageInfo {
  dataRoot: string;
  defaultDataRoot: string;
  webviewDataRoot: string;
  totalBytes: number;
  safeCacheBytes: number;
  cleanupPending: boolean;
  restartRequired: boolean;
  isDefault: boolean;
}

export interface ConversationMessage {
  role: "user" | "assistant";
  content: string;
}

export interface AskResult {
  text: string;
  cancelled: boolean;
}

export interface AnswerDelta {
  requestId: string;
  delta: string;
}

export interface AnswerHistoryEntry {
  prompt: string;
  answer: string;
}

export type ApiKeyUpdate =
  | { action: "keep" }
  | { action: "replace"; value: string }
  | { action: "clear" };

export type SecurityState = "ready" | "migrated" | "recovered";

export interface SettingsSnapshot {
  settings: AppSettings;
  apiKeyConfigured: boolean;
  securityState: SecurityState;
}

export type SettingsLoadResult =
  | { state: "ready"; snapshot: SettingsSnapshot }
  | { state: "locked"; reason: string; message: string };

export interface SecurityResetResult {
  snapshot: SettingsSnapshot;
  quarantinePath: string | null;
}
