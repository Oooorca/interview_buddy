import prompts from "../../resources/default-prompts.json";
import type { AppSettings } from "./types";

export const DEFAULT_SYSTEM_PROMPT = prompts.system;
export const DEFAULT_CODING_PROMPT = prompts.coding;

export const defaultSettings: AppSettings = {
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
