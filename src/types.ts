export interface AppSettings {
  baseUrl: string;
  apiKey: string;
  model: string;
  visionModel: string;
  transcriptionModel: string;
  systemPrompt: string;
  codingPrompt: string;
}

export interface CaptureResult {
  dataUrl: string;
  width: number;
  height: number;
}

export const defaultSettings: AppSettings = {
  baseUrl: "https://api.openai.com/v1",
  apiKey: "",
  model: "gpt-4.1-mini",
  visionModel: "gpt-4.1",
  transcriptionModel: "gpt-4o-mini-transcribe",
  systemPrompt:
    "你是会议实时 Copilot。根据上下文先理解对方意图，再给出自然、简短、可以直接说出口的中文回答；必要时补充要点和一个合适的反问。不要编造事实。",
  codingPrompt:
    "你是算法面试助手。识别截图中的完整题目，给出：1. 核心思路；2. 时间与空间复杂度；3. 可直接提交的代码；4. 容易出错的边界情况。默认使用 TypeScript；如果截图指定语言则遵从截图。回答紧凑、正确、适合手撕讲解。",
};
