import promptsEnUs from "../../resources/default-prompts/en-US.json";
import promptsZhCn from "../../resources/default-prompts/zh-CN.json";
import type { AnswerLanguage, AppSettings, SupportedLanguage, UiLanguage } from "./types";

export const DEFAULT_PROMPTS = {
  "zh-CN": promptsZhCn,
  "en-US": promptsEnUs,
} as const;

export function systemLanguage(): SupportedLanguage {
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
}

export function effectiveUiLanguage(language: UiLanguage): SupportedLanguage {
  return language === "system" ? systemLanguage() : language;
}

export function effectiveAnswerLanguage(
  answerLanguage: AnswerLanguage,
  uiLanguage: UiLanguage,
): SupportedLanguage {
  return answerLanguage === "follow-ui" ? effectiveUiLanguage(uiLanguage) : answerLanguage;
}

export function defaultPromptsFor(settings: Pick<AppSettings, "answerLanguage" | "uiLanguage">) {
  return DEFAULT_PROMPTS[effectiveAnswerLanguage(settings.answerLanguage, settings.uiLanguage)];
}

export const defaultSettings: AppSettings = {
  uiLanguage: "system",
  answerLanguage: "follow-ui",
  windowSizePreset: "standard",
  customWindowWidth: 880,
  customWindowHeight: 540,
  baseUrl: "https://api.openai.com/v1",
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
  systemPromptMode: "default",
  codingPromptMode: "default",
  systemPrompt: null,
  codingPrompt: null,
};
