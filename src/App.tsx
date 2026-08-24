import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { AnswerView } from "./AnswerView";
import { backend } from "./tauri";
import {
  defaultSettings,
  type AnswerDelta,
  type AnswerHistoryEntry,
  type AppSettings,
  type AudioOutputDevice,
  type CaptureResult,
  type ConversationMessage,
  type StorageInfo,
} from "./types";

type Status = "ready" | "working" | "error";
type MicMonitor = {
  audioContext: AudioContext;
  analyser: AnalyserNode;
  samples: Float32Array<ArrayBuffer>;
};
type MicSession = {
  recorder: MediaRecorder;
  stream: MediaStream;
  chunks: Blob[];
  mimeType: string;
  monitor: MicMonitor;
};
type PendingAutoAnswer = { question: string; sessionId: number; readyAt: number };
type TranscriptEntry = {
  id: string;
  speaker: "me" | "them";
  text: string;
  pinned: boolean;
};
type SettingsPage = "api" | "audio" | "storage";
type VadAction = "flush" | "discard" | null;
type VadState = {
  initialized: boolean;
  speaking: boolean;
  speechMs: number;
  silenceMs: number;
  utteranceMs: number;
  idleMs: number;
  noiseFloor: number;
  lastAt: number;
};
const VAD_POLL_MS = 100;
const VAD_ENERGY_FLOOR = 0.0025;
const VAD_SILENCE_MS = 800;
const VAD_MIN_SPEECH_MS = 350;
const VAD_MAX_UTTERANCE_MS = 25_000;
const VAD_IDLE_RESET_MS = 8_000;
const MAX_ANSWER_HISTORY = 30;
const MAX_TRANSCRIPT_ENTRIES = 80;
const TRANSCRIPT_CONTEXT_ENTRIES = 16;
const TRANSCRIPT_CONTEXT_CHARS = 8_000;
const AUTO_ANSWER_SETTLE_MS = 700;
const AUTO_ANSWER_DEDUPE_MS = 20_000;
const IS_MAC = navigator.userAgent.includes("Mac");
const MOD = IS_MAC ? "⌘⇧" : "Ctrl+Shift+";
const LANGUAGE_OPTIONS = [
  ["auto", "自动检测"], ["zh", "中文"], ["en", "English"], ["ja", "日本語"],
  ["ko", "한국어"], ["de", "Deutsch"], ["fr", "Français"], ["es", "Español"],
] as const;

function freshVadState(): VadState {
  return {
    initialized: false,
    speaking: false,
    speechMs: 0,
    silenceMs: 0,
    utteranceMs: 0,
    idleMs: 0,
    noiseFloor: VAD_ENERGY_FLOOR,
    lastAt: performance.now(),
  };
}

function advanceVad(state: VadState, level: number, now: number): VadAction {
  const elapsed = Math.max(40, Math.min(500, now - state.lastAt));
  state.lastAt = now;
  if (!state.initialized) {
    state.noiseFloor = Math.min(level, VAD_ENERGY_FLOOR);
    state.initialized = true;
  }
  const enterThreshold = Math.max(VAD_ENERGY_FLOOR, state.noiseFloor * 2.6);
  const exitThreshold = Math.max(VAD_ENERGY_FLOOR * 0.7, state.noiseFloor * 1.6);
  const voiced = level >= (state.speaking ? exitThreshold : enterThreshold);

  if (!state.speaking) {
    if (voiced) {
      state.speaking = true;
      state.speechMs = elapsed;
      state.silenceMs = 0;
      state.utteranceMs = elapsed;
      state.idleMs = 0;
    } else {
      state.noiseFloor = state.noiseFloor * 0.95 + level * 0.05;
      state.idleMs += elapsed;
      if (state.idleMs >= VAD_IDLE_RESET_MS) {
        Object.assign(state, freshVadState());
        return "discard";
      }
    }
    return null;
  }

  state.utteranceMs += elapsed;
  if (voiced) {
    state.speechMs += elapsed;
    state.silenceMs = 0;
  } else {
    state.silenceMs += elapsed;
  }
  const naturalPause = state.silenceMs >= VAD_SILENCE_MS;
  const hasSpeech = state.speechMs >= VAD_MIN_SPEECH_MS;
  if ((naturalPause && hasSpeech) || state.utteranceMs >= VAD_MAX_UTTERANCE_MS) {
    Object.assign(state, freshVadState());
    return "flush";
  }
  if (naturalPause && !hasSpeech) {
    Object.assign(state, freshVadState());
    return "discard";
  }
  return null;
}

function describeMicrophoneError(error: unknown): string {
  const detail = String(error);
  if (IS_MAC && /NotAllowedError|PermissionDenied|permission denied|not allowed/i.test(detail)) {
    return "麦克风：macOS 拒绝了当前构建。请在“隐私与安全性 → 麦克风”中重新开关 Interview Buddy，并彻底退出后重启应用；本地临时签名在重新构建后可能需要重新授权。";
  }
  return `麦克风：${detail}`;
}

function App() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("waiting...");
  const [captures, setCaptures] = useState<CaptureResult[]>([]);
  const [status, setStatus] = useState<Status>("ready");
  const [listening, setListening] = useState(false);
  const [autoAnswer, setAutoAnswer] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsPage, setSettingsPage] = useState<SettingsPage>("api");
  const [notice, setNotice] = useState("共享不可见");
  const [pendingTranscriptions, setPendingTranscriptions] = useState(0);
  const [answering, setAnswering] = useState(false);
  const [voiceIssue, setVoiceIssue] = useState("");
  const [microphoneDevices, setMicrophoneDevices] = useState<MediaDeviceInfo[]>([]);
  const [outputDevices, setOutputDevices] = useState<AudioOutputDevice[]>([]);
  const [devicesLoading, setDevicesLoading] = useState(false);
  const [deviceIssue, setDeviceIssue] = useState("");
  const [storageInfo, setStorageInfo] = useState<StorageInfo | null>(null);
  const [storageLoading, setStorageLoading] = useState(false);
  const [storageIssue, setStorageIssue] = useState("");
  const [answerHistory, setAnswerHistory] = useState<AnswerHistoryEntry[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [transcripts, setTranscripts] = useState<TranscriptEntry[]>([]);
  const [backgroundOpen, setBackgroundOpen] = useState(false);
  const [shortcutIssue, setShortcutIssue] = useState("");

  const micRef = useRef<MicSession | null>(null);
  const systemActiveRef = useRef(false);
  const listeningRef = useRef(false);
  const autoAnswerRef = useRef(false);
  const startingRef = useRef(false);
  const vadTimerRef = useRef<number | null>(null);
  const vadBusyRef = useRef(false);
  const micVadRef = useRef<VadState>(freshVadState());
  const systemVadRef = useRef<VadState>(freshVadState());
  const voiceSessionRef = useRef(0);
  const transcriptEpochRef = useRef(0);
  const transcriptionQueueRef = useRef<Promise<void>>(Promise.resolve());
  const pendingAutoAnswerRef = useRef<PendingAutoAnswer | null>(null);
  const autoAnswerTimerRef = useRef<number | null>(null);
  const lastAutoQuestionRef = useRef({ text: "", at: 0 });
  const busyRef = useRef(false);
  const captureBusyRef = useRef(false);
  const inputRef = useRef(input);
  const capturesRef = useRef(captures);
  const settingsRef = useRef(settings);
  const dispatchRef = useRef<(action: string) => void>(() => undefined);
  const activeRequestIdRef = useRef<string | null>(null);
  const streamStartedRef = useRef(false);
  const answerHistoryRef = useRef(answerHistory);
  const transcriptsRef = useRef(transcripts);
  const transcriptListRef = useRef<HTMLDivElement | null>(null);
  const transcriptCountRef = useRef(0);

  inputRef.current = input;
  capturesRef.current = captures;
  settingsRef.current = settings;
  listeningRef.current = listening;
  autoAnswerRef.current = autoAnswer;
  answerHistoryRef.current = answerHistory;
  transcriptsRef.current = transcripts;

  useEffect(() => {
    backend.loadSettings().then(setSettings).catch(() => undefined);
    backend.shortcutWarnings().then((warnings) => {
      if (!warnings.length) return;
      const issue = warnings.join("；");
      setShortcutIssue(issue);
      setNotice(`快捷键冲突：${warnings.map((warning) => warning.split("：")[0]).join("、")}`);
    }).catch(() => undefined);
  }, []);

  useEffect(() => {
    if (transcripts.length <= transcriptCountRef.current) {
      transcriptCountRef.current = transcripts.length;
      return;
    }
    transcriptCountRef.current = transcripts.length;
    requestAnimationFrame(() => {
      const list = transcriptListRef.current;
      if (list) list.scrollTop = list.scrollHeight;
    });
  }, [transcripts.length]);

  useEffect(() => {
    if (settingsOpen && settingsPage === "audio") void refreshAudioDevices(false);
    if (settingsOpen && settingsPage === "storage") void refreshStorageInfo();
  }, [settingsOpen, settingsPage]);

  useEffect(() => {
    const mediaDevices = navigator.mediaDevices;
    if (!mediaDevices?.addEventListener) return;
    const refresh = () => { if (settingsOpen && settingsPage === "audio") void refreshAudioDevices(false); };
    mediaDevices.addEventListener("devicechange", refresh);
    return () => mediaDevices.removeEventListener("devicechange", refresh);
  }, [settingsOpen, settingsPage]);

  useEffect(() => {
    let active = true;
    const disposers: Array<() => void> = [];
    Promise.all([
      listen<string>("shortcut-action", ({ payload }) => dispatchRef.current(payload)),
      listen<CaptureResult>("region-captured", ({ payload }) => {
        captureBusyRef.current = false;
        addCapture(payload);
      }),
      listen<string>("region-capture-error", ({ payload }) => {
        captureBusyRef.current = false;
        showError("区域截图失败", payload);
      }),
      listen("region-capture-cancelled", () => {
        captureBusyRef.current = false;
        setStatus("ready");
        setNotice("已取消区域截图");
      }),
      listen<AnswerDelta>("answer-stream-delta", ({ payload }) => {
        if (payload.requestId !== activeRequestIdRef.current) return;
        setOutput((current) => {
          if (!streamStartedRef.current) {
            streamStartedRef.current = true;
            return payload.delta;
          }
          return current + payload.delta;
        });
      }),
    ]).then((items) => {
      if (active) disposers.push(...items);
      else items.forEach((dispose) => dispose());
    });
    return () => {
      active = false;
      disposers.forEach((dispose) => dispose());
    };
  }, []);

  async function refreshAudioDevices(requestMicrophonePermission: boolean) {
    if (devicesLoading) return;
    setDevicesLoading(true);
    setDeviceIssue("");
    const errors: string[] = [];
    if (requestMicrophonePermission) {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        stream.getTracks().forEach((track) => track.stop());
      } catch (error) {
        errors.push(`麦克风授权：${String(error)}`);
      }
    }
    const [browserDevices, nativeOutputs] = await Promise.allSettled([
      navigator.mediaDevices.enumerateDevices(),
      backend.listSystemAudioDevices(),
    ]);
    if (browserDevices.status === "fulfilled") {
      setMicrophoneDevices(browserDevices.value.filter((device) => device.kind === "audioinput"));
    } else {
      errors.push(`输入设备：${String(browserDevices.reason)}`);
    }
    if (nativeOutputs.status === "fulfilled") setOutputDevices(nativeOutputs.value);
    else errors.push(`输出设备：${String(nativeOutputs.reason)}`);
    setDeviceIssue(errors.join("；"));
    setDevicesLoading(false);
  }

  async function refreshStorageInfo() {
    setStorageLoading(true);
    setStorageIssue("");
    try { setStorageInfo(await backend.storageInfo()); }
    catch (error) { setStorageIssue(String(error)); }
    finally { setStorageLoading(false); }
  }

  async function chooseStorageRoot() {
    setStorageIssue("");
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择 Interview Buddy 数据与缓存目录",
        defaultPath: storageInfo?.dataRoot,
      });
      if (typeof selected !== "string") return;
      await applyStorageRoot(selected);
    } catch (error) { setStorageIssue(String(error)); }
    finally { setStorageLoading(false); }
  }

  async function applyStorageRoot(path: string) {
    setStorageLoading(true);
    await backend.saveSettings(settingsRef.current);
    const info = await backend.setStorageRoot(path);
    setStorageInfo(info);
    setNotice(info.restartRequired ? "存储目录已修改，重启后生效" : "存储目录未改变");
  }

  async function restoreDefaultStorageRoot() {
    if (!storageInfo) return;
    setStorageIssue("");
    try { await applyStorageRoot(storageInfo.defaultDataRoot); }
    catch (error) { setStorageIssue(String(error)); }
    finally { setStorageLoading(false); }
  }

  async function scheduleStorageCleanup() {
    setStorageLoading(true);
    setStorageIssue("");
    try {
      setStorageInfo(await backend.scheduleSafeCleanup());
      setNotice("已安排下次启动前安全清理缓存");
    } catch (error) { setStorageIssue(String(error)); }
    finally { setStorageLoading(false); }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function appendToCurrentInput(text: string) {
    const addition = text.trim();
    if (!addition) return;
    const current = inputRef.current.trim();
    const next = current ? `${current}\n${addition}` : addition;
    inputRef.current = next;
    setInput(next);
    setNotice("已加入本轮输入");
  }

  function appendTranscripts(entries: Omit<TranscriptEntry, "id" | "pinned">[]) {
    if (!entries.length) return;
    setTranscripts((current) => {
      const additions = entries.map((entry) => ({ ...entry, id: crypto.randomUUID(), pinned: false }));
      const next = [...current, ...additions].slice(-MAX_TRANSCRIPT_ENTRIES);
      transcriptsRef.current = next;
      return next;
    });
  }

  function updateTranscript(id: string, change: Partial<Pick<TranscriptEntry, "text" | "pinned">>) {
    setTranscripts((current) => {
      const next = current.map((entry) => entry.id === id ? { ...entry, ...change } : entry);
      transcriptsRef.current = next;
      return next;
    });
  }

  function removeTranscript(id: string) {
    setTranscripts((current) => {
      const next = current.filter((entry) => entry.id !== id);
      transcriptsRef.current = next;
      return next;
    });
  }

  function transcriptContext(): string {
    const entries = transcriptsRef.current.filter((entry) => entry.text.trim());
    const pinned = entries.filter((entry) => entry.pinned);
    const selectedIds = new Set(pinned.map((entry) => entry.id));
    let used = pinned.reduce((total, entry) => total + entry.text.trim().length + 3, 0);
    const recent = entries.filter((entry) => !entry.pinned).slice(-TRANSCRIPT_CONTEXT_ENTRIES).reverse();
    for (const entry of recent) {
      const size = entry.text.trim().length + 3;
      if (used + size > TRANSCRIPT_CONTEXT_CHARS && selectedIds.size) break;
      selectedIds.add(entry.id);
      used += size;
    }
    return entries
      .filter((entry) => selectedIds.has(entry.id))
      .map((entry) => `${entry.speaker === "me" ? "我" : "对方"}：${entry.text.trim()}`)
      .join("\n");
  }

  function composePrompt(currentText: string, source: "manual" | "auto"): string {
    const sections: string[] = [];
    const fixed = settingsRef.current.fixedContext.trim();
    const transcript = transcriptContext();
    const draft = inputRef.current.trim();
    if (fixed) sections.push(`【固定背景】\n${fixed}`);
    if (transcript) sections.push(`【近期对话】\n${transcript}`);
    if (source === "auto") {
      sections.push(`【当前需要回答的问题】\n${currentText.trim()}`);
      if (draft) sections.push(`【本轮补充】\n${draft}`);
    } else {
      sections.push(`【本轮输入】\n${currentText.trim()}`);
    }
    return sections.join("\n\n");
  }

  function addCapture(capture: CaptureResult) {
    const imageNumber = capturesRef.current.length + 1;
    setCaptures((current) => {
      const next = [...current, capture];
      capturesRef.current = next;
      return next;
    });
    setStatus("ready");
    setNotice(`已加入 image_${imageNumber}`);
  }

  function removeCapture(index: number) {
    setCaptures((current) => {
      const next = current.filter((__, itemIndex) => itemIndex !== index);
      capturesRef.current = next;
      return next;
    });
  }

  function conversationMessages(): ConversationMessage[] {
    return answerHistoryRef.current.flatMap((entry) => [
      { role: "user" as const, content: entry.prompt },
      { role: "assistant" as const, content: entry.answer },
    ]);
  }

  function rememberAnswer(
    prompt: string,
    answer: string,
  ) {
    const entry: AnswerHistoryEntry = {
      prompt,
      answer,
    };
    setAnswerHistory((current) => {
      const next = [...current, entry].slice(-MAX_ANSWER_HISTORY);
      answerHistoryRef.current = next;
      setHistoryIndex(next.length - 1);
      return next;
    });
  }

  async function generateAnswer(
    prompt: string,
    images: CaptureResult[],
    source: "manual" | "auto",
    historyPrompt = prompt,
  ): Promise<string | null> {
    if (busyRef.current) {
      setNotice("已有回答正在生成");
      return null;
    }
    const requestId = crypto.randomUUID();
    const previousOutput = output;
    activeRequestIdRef.current = requestId;
    streamStartedRef.current = false;
    busyRef.current = true;
    setAnswering(true);
    setStatus("working");
    setNotice(source === "auto" ? "检测到问题，正在生成回答…" : "正在生成回答…");
    try {
      const result = await backend.ask(
        requestId,
        prompt,
        images.map((image) => image.dataUrl),
        conversationMessages(),
      );
      if (activeRequestIdRef.current !== requestId) return null;
      const answer = result.text.trim();
      if (result.cancelled) {
        if (!streamStartedRef.current) setOutput(previousOutput);
        setStatus("ready");
        setNotice("已停止生成，未写入会话历史");
        return null;
      }
      if (!answer) throw new Error("模型没有返回文本内容");
      setOutput(answer);
      rememberAnswer(historyPrompt, answer);
      setStatus("ready");
      setNotice(source === "auto" ? "自动回答已更新" : "回答完成");
      return answer;
    } catch (error) {
      setStatus("error");
      setNotice(`请求失败：${String(error)}`);
      if (!streamStartedRef.current) setOutput(previousOutput);
      return null;
    } finally {
      if (activeRequestIdRef.current === requestId) activeRequestIdRef.current = null;
      busyRef.current = false;
      setAnswering(false);
      void pumpAutoAnswer();
    }
  }

  async function stopGenerating() {
    const requestId = activeRequestIdRef.current;
    if (!requestId) return;
    setNotice("正在停止生成…");
    try { await backend.cancelAnswer(requestId); }
    catch (error) { setNotice(`停止生成失败：${String(error)}`); }
  }

  function showHistoryEntry(index: number) {
    const entry = answerHistoryRef.current[index];
    if (!entry || answering) return;
    setHistoryIndex(index);
    setOutput(entry.answer);
    setStatus("ready");
    setNotice(`会话回答 ${index + 1}/${answerHistoryRef.current.length}`);
  }

  function startNewSession() {
    if (answering) {
      setNotice("请先停止当前回答");
      return;
    }
    pendingAutoAnswerRef.current = null;
    transcriptEpochRef.current += 1;
    if (autoAnswerTimerRef.current) globalThis.clearTimeout(autoAnswerTimerRef.current);
    autoAnswerTimerRef.current = null;
    lastAutoQuestionRef.current = { text: "", at: 0 };
    answerHistoryRef.current = [];
    transcriptsRef.current = [];
    inputRef.current = "";
    capturesRef.current = [];
    setAnswerHistory([]);
    setTranscripts([]);
    setInput("");
    setCaptures([]);
    setHistoryIndex(-1);
    setOutput("waiting...");
    setStatus("ready");
    setNotice("已开始新会话；固定背景已保留");
  }

  async function takeRegionScreenshot() {
    if (captureBusyRef.current) return;
    captureBusyRef.current = true;
    setStatus("working");
    setNotice("拖拽选择截图区域，Esc 取消");
    try { await backend.openRegionSelector(); }
    catch (error) {
      captureBusyRef.current = false;
      showError("打开区域截图失败", error);
    }
  }

  async function sendRequest() {
    const text = inputRef.current.trim();
    const images = capturesRef.current;
    if (!text && !images.length) { setNotice("本轮输入为空"); return; }
    if (!settingsRef.current.apiKey.trim()) {
      setSettingsOpen(true);
      setOutput("请先在设置中填写 API Key。");
      return;
    }
    const currentText = text || settingsRef.current.codingPrompt;
    const prompt = composePrompt(currentText, "manual");
    const answer = await generateAnswer(prompt, images, "manual", currentText);
    if (!answer) return;
    let preservedNewInput = false;
    if (inputRef.current.trim() === text) {
      inputRef.current = "";
      setInput("");
    } else preservedNewInput = true;
    const capturesUnchanged = capturesRef.current.length === images.length
      && capturesRef.current.every((capture, index) => capture.dataUrl === images[index]?.dataUrl);
    if (capturesUnchanged) {
      capturesRef.current = [];
      setCaptures([]);
    } else preservedNewInput = true;
    setNotice(preservedNewInput ? "回答完成；生成期间的新输入已保留" : "回答完成；本轮输入已清空");
  }

  function createMicMonitor(stream: MediaStream): MicMonitor {
    const audioContext = new AudioContext();
    const analyser = audioContext.createAnalyser();
    analyser.fftSize = 1024;
    analyser.smoothingTimeConstant = 0.15;
    audioContext.createMediaStreamSource(stream).connect(analyser);
    void audioContext.resume();
    return {
      audioContext,
      analyser,
      samples: new Float32Array(analyser.fftSize),
    };
  }

  function createMicSession(stream: MediaStream, mimeType: string, monitor?: MicMonitor): MicSession {
    const recorder = new MediaRecorder(stream, { mimeType });
    const session: MicSession = {
      recorder,
      stream,
      chunks: [],
      mimeType,
      monitor: monitor || createMicMonitor(stream),
    };
    recorder.ondataavailable = (event) => { if (event.data.size) session.chunks.push(event.data); };
    recorder.start(250);
    return session;
  }

  function microphoneLevel(): number {
    const monitor = micRef.current?.monitor;
    if (!monitor) return 0;
    monitor.analyser.getFloatTimeDomainData(monitor.samples);
    let squares = 0;
    for (const sample of monitor.samples) squares += sample * sample;
    return Math.sqrt(squares / monitor.samples.length);
  }

  async function startDevices(): Promise<void> {
    const failures: string[] = [];
    const current = settingsRef.current;
    if (!current.captureMicrophone && !current.captureSystemAudio) {
      throw new Error("请先在设置中启用麦克风或系统音频");
    }
    if (current.captureMicrophone) {
      let stream: MediaStream | null = null;
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          audio: {
            echoCancellation: true,
            noiseSuppression: true,
            autoGainControl: true,
            ...(current.microphoneDeviceId
              ? { deviceId: { exact: current.microphoneDeviceId } }
              : {}),
          },
        });
        const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
          ? "audio/webm;codecs=opus" : "audio/webm";
        micRef.current = createMicSession(stream, mimeType);
      } catch (error) {
        stream?.getTracks().forEach((track) => track.stop());
        failures.push(describeMicrophoneError(error));
      }
    }
    if (current.captureSystemAudio) {
      try {
        await backend.startSystemAudio();
        systemActiveRef.current = true;
      } catch (error) { failures.push(`系统音频：${String(error)}`); }
    }
    if (!micRef.current && !systemActiveRef.current) throw new Error(failures.join("；"));
    setVoiceIssue(failures.join("；"));
  }

  async function transcribeMic(session: MicSession, stopTracks: boolean): Promise<string> {
    try {
      if (session.recorder.state !== "inactive") {
        await new Promise<void>((resolve) => {
          session.recorder.addEventListener("stop", () => resolve(), { once: true });
          session.recorder.stop();
        });
      }
    } finally {
      if (stopTracks) {
        session.stream.getTracks().forEach((track) => track.stop());
        void session.monitor.audioContext.close();
      }
    }
    const blob = new Blob(session.chunks, { type: session.mimeType });
    if (blob.size < 256) return "";
    const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
    return backend.transcribe(bytes, session.mimeType);
  }

  function rotateMicAndTranscribe(): Promise<string> {
    const current = micRef.current;
    if (!current) return Promise.resolve("");
    const next = createMicSession(current.stream, current.mimeType, current.monitor);
    micRef.current = next;
    return transcribeMic(current, false);
  }

  async function rotateMicAndDiscard(): Promise<void> {
    const current = micRef.current;
    if (!current) return;
    micRef.current = createMicSession(current.stream, current.mimeType, current.monitor);
    if (current.recorder.state !== "inactive") {
      await new Promise<void>((resolve) => {
        current.recorder.addEventListener("stop", () => resolve(), { once: true });
        current.recorder.stop();
      });
    }
  }

  function looksLikeInterviewPrompt(text: string): boolean {
    const compact = text.replace(/\s+/g, " ").trim();
    if (compact.length < 4) return false;
    if (/[?？]$/.test(compact)) return true;
    return /(请问|怎么|如何|为什么|什么|哪些|是否|能否|可以|讲讲|介绍一下|说说|解释|区别|优缺点|复杂度|实现|设计)/.test(compact)
      || /\b(what|why|how|when|where|who|which|can|could|would|do|does|did|is|are|tell me|explain|describe|compare|implement|design|complexity)\b/i.test(compact)
      || /(どのよう|なぜ|何|説明|教えて|어떻게|왜|무엇|설명|comment|pourquoi|quoi|explique|cómo|por qué|qué|explica|warum|was|wie|erkläre)/i.test(compact);
  }

  function normalizedQuestion(text: string): string {
    return text.toLocaleLowerCase().replace(/[\s?？。,.!！:：;；'"“”‘’]/g, "");
  }

  function queueAutoAnswer(question: string, sessionId: number) {
    const normalized = normalizedQuestion(question);
    const now = Date.now();
    const last = lastAutoQuestionRef.current;
    if (normalized && normalized === last.text && now - last.at < AUTO_ANSWER_DEDUPE_MS) {
      setNotice("已忽略重复问题，继续听写");
      return;
    }
    pendingAutoAnswerRef.current = {
      question,
      sessionId,
      readyAt: now + AUTO_ANSWER_SETTLE_MS,
    };
    if (autoAnswerTimerRef.current) globalThis.clearTimeout(autoAnswerTimerRef.current);
    autoAnswerTimerRef.current = globalThis.setTimeout(() => {
      autoAnswerTimerRef.current = null;
      void pumpAutoAnswer();
    }, AUTO_ANSWER_SETTLE_MS);
    setNotice("检测到问题，正在合并当前语句…");
  }

  function handleAutoAnswerChunk(text: string, sessionId: number) {
    const pending = pendingAutoAnswerRef.current;
    if (pending?.sessionId === sessionId) {
      queueAutoAnswer(`${pending.question} ${text}`.trim(), sessionId);
      return;
    }
    if (looksLikeInterviewPrompt(text)) queueAutoAnswer(text, sessionId);
  }

  async function pumpAutoAnswer(): Promise<void> {
    if (busyRef.current) return;
    const pending = pendingAutoAnswerRef.current;
    if (!pending) return;
    const waitMs = pending.readyAt - Date.now();
    if (waitMs > 0) {
      if (autoAnswerTimerRef.current) globalThis.clearTimeout(autoAnswerTimerRef.current);
      autoAnswerTimerRef.current = globalThis.setTimeout(() => {
        autoAnswerTimerRef.current = null;
        void pumpAutoAnswer();
      }, waitMs);
      return;
    }
    pendingAutoAnswerRef.current = null;
    if (pending.sessionId !== voiceSessionRef.current || !autoAnswerRef.current) {
      void pumpAutoAnswer();
      return;
    }
    if (!settingsRef.current.apiKey.trim()) {
      setVoiceIssue("自动回答需要先填写 API Key");
      return;
    }
    lastAutoQuestionRef.current = { text: normalizedQuestion(pending.question), at: Date.now() };
    const prompt = composePrompt(pending.question, "auto");
    const answer = await generateAnswer(prompt, capturesRef.current, "auto", pending.question);
    if (!answer && pending.sessionId === voiceSessionRef.current && autoAnswerRef.current) {
      lastAutoQuestionRef.current = { text: "", at: 0 };
      setVoiceIssue("自动回答未完成");
    }
  }

  function handleVoiceResults(
    mine: PromiseSettledResult<string>,
    theirs: PromiseSettledResult<string>,
    autoAnswerForChunk: boolean,
    sessionId: number,
  ) {
    const entries: Omit<TranscriptEntry, "id" | "pinned">[] = [];
    const errors: string[] = [];
    const myText = mine.status === "fulfilled" ? mine.value.trim() : "";
    const theirText = theirs.status === "fulfilled" ? theirs.value.trim() : "";
    if (myText) entries.push({ speaker: "me", text: myText });
    if (mine.status === "rejected") errors.push(`我的语音：${String(mine.reason)}`);
    if (theirText) entries.push({ speaker: "them", text: theirText });
    if (theirs.status === "rejected") errors.push(`对方语音：${String(theirs.reason)}`);
    appendTranscripts(entries);
    setVoiceIssue(errors.join("；"));
    if (errors.length) setNotice("部分音源转写失败，原回答已保留");
    else if (entries.length) setNotice("已加入实时转写");
    else if (listeningRef.current) setNotice(`${autoAnswerRef.current ? "自动回答" : "听写"}中，等待语音…`);
    if (autoAnswerForChunk && theirText) handleAutoAnswerChunk(theirText, sessionId);
  }

  function queueVoiceChunk(
    mine: Promise<string>,
    theirs: Promise<string>,
    autoAnswerForChunk: boolean,
    sessionId: number,
  ): Promise<void> {
    const transcriptEpoch = transcriptEpochRef.current;
    setPendingTranscriptions((count) => count + 1);
    const results = Promise.allSettled([mine, theirs]);
    const task = transcriptionQueueRef.current
      .then(async () => {
        const [myResult, theirResult] = await results;
        if (transcriptEpoch !== transcriptEpochRef.current) return;
        handleVoiceResults(myResult, theirResult, autoAnswerForChunk, sessionId);
      })
      .catch((error) => {
        setVoiceIssue(`转写任务失败：${String(error)}`);
        setNotice("转写失败，原回答已保留");
      });
    transcriptionQueueRef.current = task;
    return task.finally(() => setPendingTranscriptions((count) => Math.max(0, count - 1)));
  }

  async function runVadTick(sessionId: number) {
    if (vadBusyRef.current || !listeningRef.current || voiceSessionRef.current !== sessionId) return;
    vadBusyRef.current = true;
    try {
      const systemLevel = systemActiveRef.current
        ? await backend.systemAudioLevel().catch(() => 0)
        : 0;
      if (!listeningRef.current || voiceSessionRef.current !== sessionId) return;
      const now = performance.now();
      const micAction = micRef.current ? advanceVad(micVadRef.current, microphoneLevel(), now) : null;
      const systemAction = systemActiveRef.current
        ? advanceVad(systemVadRef.current, systemLevel, now)
        : null;

      if (micAction === "discard") await rotateMicAndDiscard();
      if (systemAction === "discard") await backend.discardSystemAudioChunk();
      if (micAction === "flush" || systemAction === "flush") {
        const mine = micAction === "flush" ? rotateMicAndTranscribe() : Promise.resolve("");
        const theirs = systemAction === "flush"
          ? backend.transcribeSystemAudioChunk()
          : Promise.resolve("");
        void queueVoiceChunk(mine, theirs, autoAnswerRef.current, sessionId);
      }
    } catch (error) {
      setVoiceIssue(`语音分段失败：${String(error)}`);
      setNotice("语音分段异常，录音仍在继续");
    } finally {
      vadBusyRef.current = false;
    }
  }

  function startVadMonitor(sessionId: number) {
    if (vadTimerRef.current) globalThis.clearInterval(vadTimerRef.current);
    micVadRef.current = freshVadState();
    systemVadRef.current = freshVadState();
    vadTimerRef.current = globalThis.setInterval(() => {
      void runVadTick(sessionId);
    }, VAD_POLL_MS);
  }

  async function startListening(enableAutoAnswer = false) {
    if (listeningRef.current || startingRef.current) return;
    startingRef.current = true;
    try {
      await startDevices();
      const sessionId = voiceSessionRef.current + 1;
      voiceSessionRef.current = sessionId;
      listeningRef.current = true;
      autoAnswerRef.current = enableAutoAnswer;
      setListening(true);
      setAutoAnswer(enableAutoAnswer);
      setStatus("ready");
      setNotice(`${enableAutoAnswer ? "自动回答" : "听写"}已开启（按自然停顿转写）`);
      startVadMonitor(sessionId);
    } catch (error) {
      autoAnswerRef.current = false;
      setAutoAnswer(false);
      setVoiceIssue(String(error));
      setStatus("error");
      setNotice("无法启动语音识别，原回答已保留");
    } finally {
      startingRef.current = false;
    }
  }

  async function stopListening() {
    if (!listeningRef.current) { setNotice("听写尚未开始"); return; }
    const sessionId = voiceSessionRef.current;
    listeningRef.current = false;
    autoAnswerRef.current = false;
    pendingAutoAnswerRef.current = null;
    if (autoAnswerTimerRef.current) globalThis.clearTimeout(autoAnswerTimerRef.current);
    autoAnswerTimerRef.current = null;
    setListening(false);
    setAutoAnswer(false);
    if (vadTimerRef.current) globalThis.clearInterval(vadTimerRef.current);
    vadTimerRef.current = null;
    const mic = micRef.current;
    const hadSystem = systemActiveRef.current;
    micRef.current = null;
    systemActiveRef.current = false;
    setNotice("正在完成最后一段转写…");
    await queueVoiceChunk(
      mic ? transcribeMic(mic, true) : Promise.resolve(""),
      hadSystem ? backend.stopSystemAudio() : Promise.resolve(""),
      false,
      sessionId,
    );
    setNotice("听写已停止");
  }

  async function toggleAutoAnswer() {
    if (autoAnswerRef.current) {
      autoAnswerRef.current = false;
      pendingAutoAnswerRef.current = null;
      if (autoAnswerTimerRef.current) globalThis.clearTimeout(autoAnswerTimerRef.current);
      autoAnswerTimerRef.current = null;
      setAutoAnswer(false);
      setNotice("自动回答已关闭，继续听写");
      return;
    }
    if (!listeningRef.current) {
      await startListening(true);
      return;
    }
    autoAnswerRef.current = true;
    setAutoAnswer(true);
    setNotice("自动回答已开启，听写继续运行");
  }

  function clearCurrentInput() {
    inputRef.current = "";
    capturesRef.current = [];
    setInput("");
    setCaptures([]);
    setNotice("已清空本轮输入和截图");
  }

  function clearTranscripts() {
    transcriptEpochRef.current += 1;
    transcriptsRef.current = [];
    setTranscripts([]);
    pendingAutoAnswerRef.current = null;
    if (autoAnswerTimerRef.current) globalThis.clearTimeout(autoAnswerTimerRef.current);
    autoAnswerTimerRef.current = null;
    setNotice("已清空实时转写");
  }

  async function saveFixedContext() {
    try {
      await backend.saveSettings(settingsRef.current);
      setNotice("固定背景已保存");
    } catch (error) {
      setNotice(`固定背景保存失败：${String(error)}`);
    }
  }

  function updateFixedContext(value: string) {
    setSettings((current) => {
      const next = { ...current, fixedContext: value };
      settingsRef.current = next;
      return next;
    });
  }

  function showError(label: string, error: unknown) {
    setOutput(`${label}：${String(error)}`); setStatus("error"); setNotice(label);
  }

  async function saveSettings() {
    try { await backend.saveSettings(settings); setSettingsOpen(false); setNotice("设置已保存"); }
    catch (error) { showError("保存设置失败", error); }
  }

  dispatchRef.current = (action) => {
    switch (action) {
      case "capture-region": void takeRegionScreenshot(); break;
      case "clear": clearCurrentInput(); break;
      case "listening-toggle": void (listeningRef.current ? stopListening() : startListening()); break;
      case "answer-toggle": void toggleAutoAnswer(); break;
      case "send": void sendRequest(); break;
    }
  };

  return (
    <main className={`app-shell ${IS_MAC ? "platform-mac" : "platform-windows"}`}>
      <nav className="command-bar" data-tauri-drag-region onMouseDown={(event) => {
        if (event.button === 0 && !(event.target as HTMLElement).closest("button")) void getCurrentWindow().startDragging();
      }}>
        <div className={`status-dot ${status} ${listening ? "live" : ""}`} />
        <button className="action-button" title={`区域截图 ${MOD}S`} onClick={() => void takeRegionScreenshot()}><span>⌗ 截图</span><kbd>S</kbd></button>
        <span className="divider" />
        <button className={`action-button ${listening ? "active" : ""}`} title={`${MOD}L 开始/停止听写`}
          onClick={() => void (listening ? stopListening() : startListening())}><span>{listening ? "■ 停止" : "◉ 听写"}</span><kbd>L</kbd></button>
        <button className={`action-button answer-mode ${autoAnswer ? "active" : ""}`} title={`${MOD}A 开启/关闭自动回答`}
          onClick={() => void toggleAutoAnswer()}><span>⚡ 自动答</span><kbd>A</kbd></button>
        <span className="bar-spacer" data-tauri-drag-region />
        <span className="notice" title={shortcutIssue || notice}>{notice}</span>
        <span className="window-controls-divider" />
        <div className="window-controls">
          <button className={`window-control ${settingsOpen ? "active" : ""}`} aria-label="设置" title="设置"
            onClick={() => setSettingsOpen((value) => !value)}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h10m4 0h2M4 17h2m4 0h10" /><circle cx="16" cy="7" r="2" /><circle cx="8" cy="17" r="2" /></svg>
          </button>
          <button className="window-control" aria-label="隐藏" title={`隐藏 ${MOD}Space`} onClick={() => void getCurrentWindow().hide()}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
          </button>
          <button className="window-control close-button" aria-label="关闭应用" title={`关闭应用 ${IS_MAC ? "⌘Q" : "Ctrl+Q"}`} onClick={() => void backend.quitApp()}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
          </button>
        </div>
      </nav>

      {settingsOpen ? (
        <section className="settings-panel">
          <div className="settings-title"><strong>设置</strong><button aria-label="关闭设置" onClick={() => setSettingsOpen(false)}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
          </button></div>
          <nav className="settings-tabs">
            <button className={settingsPage === "api" ? "active" : ""} onClick={() => setSettingsPage("api")}>API 与 Prompt</button>
            <button className={settingsPage === "audio" ? "active" : ""} onClick={() => setSettingsPage("audio")}>音频</button>
            <button className={settingsPage === "storage" ? "active" : ""} onClick={() => setSettingsPage("storage")}>存储与清理</button>
          </nav>
          <div className="settings-content">
            {settingsPage === "api" ? <div className="settings-page api-page">
              <label>API Base URL<input value={settings.baseUrl} onChange={(e) => setSettings({ ...settings, baseUrl: e.target.value })} /></label>
              <label>API Key<input type="password" value={settings.apiKey} onChange={(e) => setSettings({ ...settings, apiKey: e.target.value })} /></label>
              <div className="settings-grid">
                <label>文本模型<input value={settings.model} onChange={(e) => setSettings({ ...settings, model: e.target.value })} /></label>
                <label>视觉模型<input value={settings.visionModel} onChange={(e) => setSettings({ ...settings, visionModel: e.target.value })} /></label>
              </div>
              <label>转写模型<input value={settings.transcriptionModel} onChange={(e) => setSettings({ ...settings, transcriptionModel: e.target.value })} /></label>
              <label>系统 Prompt<textarea rows={3} value={settings.systemPrompt} onChange={(e) => setSettings({ ...settings, systemPrompt: e.target.value })} /></label>
              <label>纯截图 Prompt<textarea rows={4} value={settings.codingPrompt} onChange={(e) => setSettings({ ...settings, codingPrompt: e.target.value })} /></label>
            </div> : settingsPage === "audio" ? <div className="settings-page audio-page">
              <div className="audio-page-heading">
                <div><strong>音频输入与输出</strong><span>设备修改将在下次开始听写时生效</span></div>
                <button className="refresh-devices" disabled={devicesLoading} onClick={() => void refreshAudioDevices(true)}>
                  {devicesLoading ? "读取中…" : "授权并刷新设备"}
                </button>
              </div>
              {deviceIssue && <div className="device-issue">{deviceIssue}</div>}
              <section className="audio-channel-card">
                <header><div><b>我的声音</b><span>麦克风输入</span></div>
                  <label className="toggle-setting"><input type="checkbox" checked={settings.captureMicrophone}
                    onChange={(e) => setSettings({ ...settings, captureMicrophone: e.target.checked })} /><i />启用</label></header>
                <div className="audio-channel-fields">
                  <label>输入设备<select disabled={!settings.captureMicrophone} value={settings.microphoneDeviceId}
                    onChange={(e) => setSettings({ ...settings, microphoneDeviceId: e.target.value })}>
                    <option value="">系统默认麦克风</option>
                    {settings.microphoneDeviceId && !microphoneDevices.some((device) => device.deviceId === settings.microphoneDeviceId)
                      && <option value={settings.microphoneDeviceId}>已选择的设备当前不可用</option>}
                    {microphoneDevices.filter((device) => device.deviceId !== "default").map((device, index) =>
                      <option key={device.deviceId} value={device.deviceId}>{device.label || `麦克风 ${index + 1}`}</option>)}
                  </select></label>
                  <label>我的语言<select disabled={!settings.captureMicrophone} value={settings.myTranscriptionLanguage}
                    onChange={(e) => setSettings({ ...settings, myTranscriptionLanguage: e.target.value })}>
                    {LANGUAGE_OPTIONS.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                  </select></label>
                </div>
              </section>
              <section className="audio-channel-card counterpart">
                <header><div><b>对方声音</b><span>系统输出回环</span></div>
                  <label className="toggle-setting"><input type="checkbox" checked={settings.captureSystemAudio}
                    onChange={(e) => setSettings({ ...settings, captureSystemAudio: e.target.checked })} /><i />启用</label></header>
                <div className="audio-channel-fields">
                  <label>输出设备<select disabled={!settings.captureSystemAudio} value={settings.systemAudioDeviceId}
                    onChange={(e) => setSettings({ ...settings, systemAudioDeviceId: e.target.value })}>
                    <option value="">系统默认输出设备</option>
                    {settings.systemAudioDeviceId && !outputDevices.some((device) => device.id === settings.systemAudioDeviceId)
                      && <option value={settings.systemAudioDeviceId}>已选择的设备当前不可用</option>}
                    {outputDevices.filter((device) => device.id).map((device) => <option key={device.id} value={device.id}>
                      {device.name}{device.isDefault ? "（当前默认）" : ""}
                    </option>)}
                  </select></label>
                  <label>对方语言<select disabled={!settings.captureSystemAudio} value={settings.theirTranscriptionLanguage}
                    onChange={(e) => setSettings({ ...settings, theirTranscriptionLanguage: e.target.value })}>
                    {LANGUAGE_OPTIONS.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                  </select></label>
                </div>
              </section>
              <p className="audio-note">“自动检测”适合中英混合；明确知道语种时固定语言通常更准确。系统输出设备决定捕获哪一路会议声音。</p>
            </div> : <div className="settings-page storage-page">
              <div className="storage-page-heading">
                <div><strong>数据与缓存目录</strong><span>设置、WebView2 数据及以后产生的持久化内容统一保存在这里</span></div>
                <button className="refresh-devices" disabled={storageLoading} onClick={() => void refreshStorageInfo()}>
                  {storageLoading ? "读取中…" : "刷新容量"}
                </button>
              </div>
              {storageIssue && <div className="device-issue">{storageIssue}</div>}
              {storageInfo && <>
                <section className="storage-location-card">
                  <header><div><b>当前存储目录</b><span>{storageInfo.isDefault ? "默认：系统应用数据目录" : "自定义位置"}</span></div>
                    <i className={storageInfo.isDefault ? "default" : "custom"}>{storageInfo.isDefault ? "DEFAULT" : "CUSTOM"}</i></header>
                  <div className="storage-path-row">
                    <input readOnly value={storageInfo.dataRoot} title={storageInfo.dataRoot} />
                    <div><button disabled={storageLoading || storageInfo.restartRequired} onClick={() => void chooseStorageRoot()}>选择目录</button>
                      <button disabled={storageLoading || storageInfo.restartRequired || storageInfo.isDefault}
                        onClick={() => void restoreDefaultStorageRoot()}>恢复默认</button></div>
                  </div>
                  <div className="storage-subpath"><span>WebView2</span><code>{storageInfo.webviewDataRoot}</code></div>
                  {storageInfo.restartRequired && <div className="storage-restart">目录已更新。请关闭并重新打开应用，WebView2 和旧数据迁移后才会使用新位置。</div>}
                </section>
                <div className="storage-metrics">
                  <div><span>总占用</span><strong>{formatBytes(storageInfo.totalBytes)}</strong></div>
                  <div><span>可安全清理</span><strong>{formatBytes(storageInfo.safeCacheBytes)}</strong></div>
                </div>
                <section className="storage-cleanup-card">
                  <div><b>自动安全清理</b><span>每次启动、创建 WebView2 前清理普通缓存、GPU/Shader 缓存和崩溃报告</span></div>
                  <label className="toggle-setting"><input type="checkbox" checked={settings.autoSafeCleanup}
                    onChange={(e) => setSettings({ ...settings, autoSafeCleanup: e.target.checked })} /><i />{settings.autoSafeCleanup ? "已开启" : "已关闭"}</label>
                </section>
                <div className="storage-cleanup-actions">
                  <button disabled={storageLoading || storageInfo.cleanupPending} onClick={() => void scheduleStorageCleanup()}>
                    {storageInfo.cleanupPending ? "已安排下次启动清理" : "下次启动时安全清理一次"}
                  </button>
                  <p>清理不会删除 settings.json、API Key、Prompt、设备选择或麦克风身份数据。</p>
                </div>
              </>}
              {!storageInfo && !storageIssue && <div className="storage-loading">正在读取存储信息…</div>}
            </div>}
          </div>
          <div className="settings-actions"><button className="primary" onClick={() => void saveSettings()}>保存设置</button></div>
        </section>
      ) : (
        <section className="workspace">
          <div className="pane input-pane">
            <header><span>INTERVIEW INPUT</span><div className="voice-meta">
              {listening && <i className="voice-live">● {autoAnswer ? "自动回答" : "听写中"}</i>}
              {pendingTranscriptions > 0 && <i className="voice-pending">转写 {pendingTranscriptions}</i>}
              {voiceIssue && <i className="voice-error" title={voiceIssue}>音频异常</i>}
            </div></header>
            <div className="context-stack">
              <section className={`context-section fixed-context ${backgroundOpen ? "expanded" : ""}`}>
                <button className="context-section-heading collapsible" onClick={() => setBackgroundOpen((openNow) => !openNow)}>
                  <span><b>固定背景</b><em>{settings.fixedContext.trim() ? "已保存" : "可选"}</em></span>
                  <i>{backgroundOpen ? "−" : "+"}</i>
                </button>
                {backgroundOpen && <textarea className="fixed-context-input" rows={4}
                  value={settings.fixedContext}
                  onChange={(event) => updateFixedContext(event.target.value)}
                  onBlur={() => void saveFixedContext()}
                  placeholder="简历、岗位要求、项目背景等。自动保存，并在每次回答时使用。" />}
              </section>

              <section className="context-section transcript-section">
                <div className="context-section-heading">
                  <span><b>实时转写</b><em>{transcripts.length ? `${transcripts.length} 条` : "等待语音"}</em></span>
                  <div className="section-tools">
                    <button disabled={!transcripts.length} onClick={clearTranscripts} title="仅清空实时转写">清空转写</button>
                    <button disabled={answering} onClick={startNewSession} title="清空转写、本轮输入、截图和会话回答；保留固定背景">新会话</button>
                  </div>
                </div>
                <div className="transcript-list" ref={transcriptListRef}>
                  {!transcripts.length && <div className="transcript-empty">开启听写后，我和对方的语音会分开显示在这里</div>}
                  {transcripts.map((entry) => (
                    <article className={`transcript-entry ${entry.speaker} ${entry.pinned ? "pinned" : ""}`} key={entry.id}>
                      <span className="speaker-badge">{entry.speaker === "me" ? "我" : "对方"}</span>
                      <textarea rows={2} value={entry.text}
                        aria-label={`${entry.speaker === "me" ? "我" : "对方"}的转写内容`}
                        onChange={(event) => updateTranscript(entry.id, { text: event.target.value })} />
                      <div className="transcript-tools">
                        <button className={entry.pinned ? "active" : ""} title={entry.pinned ? "取消固定" : "固定到回答上下文"}
                          onClick={() => updateTranscript(entry.id, { pinned: !entry.pinned })}>⌖</button>
                        <button title="加入本轮输入" onClick={() => appendToCurrentInput(`${entry.speaker === "me" ? "我" : "对方"}：${entry.text}`)}>＋</button>
                        <button title="删除这条转写" onClick={() => removeTranscript(entry.id)}>×</button>
                      </div>
                    </article>
                  ))}
                </div>
              </section>

              <section className="context-section draft-section">
                <div className="context-section-heading">
                  <span><b>本轮输入</b><em>发送成功后自动清空</em></span>
                </div>
                <div className="attachments">
                  {captures.map((capture, index) => (
                    <button className="attachment-card" key={index} onClick={() => removeCapture(index)} title="点击移除">
                      <img src={capture.dataUrl} alt={`image_${index + 1}`} />
                      <span className="attachment-label">image_{index + 1}</span>
                      <span className="attachment-remove">×</span>
                    </button>
                  ))}
                </div>
                <textarea className="draft-input" value={input} onChange={(event) => {
                  inputRef.current = event.target.value;
                  setInput(event.target.value);
                }} placeholder="输入当前问题、补充要求或临时笔记…" />
                <div className="context-actions">
                  <button className="action-button context-clear" title={`清空本轮输入和截图 ${MOD}C`}
                    disabled={status === "working" || (!input.trim() && captures.length === 0)}
                    onClick={clearCurrentInput}><span>⌫ 清空</span><kbd>C</kbd></button>
                  <button className="action-button context-send" title={`发送 ${MOD}I`}
                    disabled={status === "working" || answering || (!input.trim() && captures.length === 0)}
                    onClick={() => void sendRequest()}><span>{answering ? "回答中…" : "发送"}</span><kbd>I</kbd></button>
                </div>
              </section>
            </div>
          </div>
          <div className="pane output-pane">
            <header className="response-header"><span>RESPONSE</span><div className="response-tools">
              {answering ? <button className="stop-answer" onClick={() => void stopGenerating()}>■ 停止</button> : <>
                <button disabled={historyIndex <= 0} title="上一条回答" onClick={() => showHistoryEntry(historyIndex - 1)}>‹</button>
                <span>{answerHistory.length ? `${historyIndex + 1}/${answerHistory.length}` : "0/0"}</span>
                <button disabled={historyIndex < 0 || historyIndex >= answerHistory.length - 1} title="下一条回答"
                  onClick={() => showHistoryEntry(historyIndex + 1)}>›</button>
                <button disabled={!answerHistory.length && !transcripts.length && !input.trim() && !captures.length}
                  title="开始新会话（保留固定背景）" onClick={startNewSession}>↺</button>
              </>}
              <i className={answering ? "working" : status}>{answering ? "STREAMING" : status.toUpperCase()}</i>
            </div></header>
            <AnswerView content={output} />
          </div>
        </section>
      )}
      <footer className="shortcut-hint">
        <span>⇧S 区域截图</span><span>⇧L 听写</span><span>⇧A 自动答</span><span>⇧I 发送</span><span>⇧C 清空</span><span>{IS_MAC ? "⌘Q" : "Ctrl+Q"} 退出</span>
      </footer>
    </main>
  );
}

export default App;
