export interface AppSettings {
  baseUrl: string;
  apiKey: string;
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
  systemPrompt: string;
  codingPrompt: string;
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

export const defaultSettings: AppSettings = {
  baseUrl: "https://api.openai.com/v1",
  apiKey: "",
  model: "gpt-4.1-mini",
  visionModel: "gpt-4.1",
  transcriptionModel: "gpt-4o-mini-transcribe",
  captureMicrophone: true,
  captureSystemAudio: true,
  microphoneDeviceId: "",
  systemAudioDeviceId: "",
  myTranscriptionLanguage: "auto",
  theirTranscriptionLanguage: "auto",
  autoSafeCleanup: false,
  fixedContext: "",
  systemPrompt:
    "你是会议实时 Copilot。根据上下文给出自然、简短、可直接说出口的中文回答；必要时补充要点和反问。",
  codingPrompt:
    "你是算法面试助手。识别截图题目，给出python语言的核心思路、复杂度、可提交代码和边界情况。对于复杂的题目，先给出一个最直观但复杂度较高的解法及代码，再逐步优化到最优解，展现思维过程。",
};
