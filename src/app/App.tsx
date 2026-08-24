import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { AnswerView } from "../features/answer/AnswerView";
import { useAnswerSession } from "../features/answer/useAnswerSession";
import { CaptureStrip } from "../features/interview/CaptureStrip";
import { ContextPanel } from "../features/interview/ContextPanel";
import { InputPanel } from "../features/interview/InputPanel";
import { TranscriptPanel, type TranscriptEntry } from "../features/listening/TranscriptPanel";
import { advanceVad, freshVadState, VAD_POLL_MS, type VadState } from "../features/listening/vad";
import { useListeningPlatform } from "../features/listening/useListening";
import { ShortcutFooter } from "../features/shell/ShortcutFooter";
import { TitleBar } from "../features/shell/TitleBar";
import { ApiSettingsPage } from "../features/settings/ApiSettingsPage";
import { AudioSettingsPage } from "../features/settings/AudioSettingsPage";
import { SecurityRecovery } from "../features/settings/SecurityRecovery";
import { SettingsDialog, type SettingsPage } from "../features/settings/SettingsDialog";
import { StorageSettingsPage } from "../features/settings/StorageSettingsPage";
import { normalizedSettingsForSave, useSettings } from "../features/settings/useSettings";
import { backend } from "../services/backend";
import { DEFAULT_CODING_PROMPT, DEFAULT_SYSTEM_PROMPT, defaultSettings } from "../shared/settings";
import {
  type AnswerDelta,
  type AnswerHistoryEntry,
  type ApiKeyUpdate,
  type AppSettings,
  type AudioOutputDevice,
  type CaptureResult,
  type ConversationMessage,
  type PromptMode,
  type StorageInfo,
} from "../shared/types";

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
const MAX_ANSWER_HISTORY = 30;
const MAX_TRANSCRIPT_ENTRIES = 80;
const TRANSCRIPT_CONTEXT_ENTRIES = 16;
const TRANSCRIPT_CONTEXT_CHARS = 8_000;
const AUTO_ANSWER_SETTLE_MS = 700;
const AUTO_ANSWER_DEDUPE_MS = 20_000;
const IS_MAC = navigator.userAgent.includes("Mac");
const IS_TAURI = "__TAURI_INTERNALS__" in window;
const MOD = IS_MAC ? "⌘⇧" : "Ctrl+Shift+";

function resolvedCodingPrompt(settings: AppSettings): string {
  if (settings.codingPromptMode === "default") return DEFAULT_CODING_PROMPT;
  if (settings.codingPromptMode === "custom") return settings.codingPrompt || "";
  return "";
}

function App() {
  const { settings, settingsRef, setSettings } = useSettings(defaultSettings);
  const { describeMicrophoneError } = useListeningPlatform(IS_MAC);
  const { answerHistory, answerHistoryRef, setAnswerHistory, historyIndex, setHistoryIndex } = useAnswerSession();
  const [apiKeyConfigured, setApiKeyConfigured] = useState(false);
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [apiKeyUpdate, setApiKeyUpdate] = useState<ApiKeyUpdate>({ action: "keep" });
  const [securityIssue, setSecurityIssue] = useState<{ reason: string; message: string } | null>(null);
  const [securityResetting, setSecurityResetting] = useState(false);
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
  const [transcripts, setTranscripts] = useState<TranscriptEntry[]>([]);
  const [backgroundOpen, setBackgroundOpen] = useState(false);
  const [shortcutIssue, setShortcutIssue] = useState("");
  const [promptIssue, setPromptIssue] = useState("");

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
  const dispatchRef = useRef<(action: string) => void>(() => undefined);
  const activeRequestIdRef = useRef<string | null>(null);
  const streamStartedRef = useRef(false);
  const transcriptsRef = useRef(transcripts);
  const transcriptListRef = useRef<HTMLDivElement | null>(null);
  const transcriptCountRef = useRef(0);

  inputRef.current = input;
  capturesRef.current = captures;
  listeningRef.current = listening;
  autoAnswerRef.current = autoAnswer;
  transcriptsRef.current = transcripts;

  useEffect(() => {
    if (!IS_TAURI) return;
    backend.loadSettings().then((result) => {
      if (result.state === "locked") {
        setSecurityIssue({ reason: result.reason, message: result.message });
        setSettingsOpen(true);
        return;
      }
      setSettings(result.snapshot.settings);
      setApiKeyConfigured(result.snapshot.apiKeyConfigured);
      if (result.snapshot.securityState === "migrated") setNotice("旧设置已安全迁移");
      if (result.snapshot.securityState === "recovered") setNotice("已从加密备份恢复设置");
    }).catch((error) => {
      setSecurityIssue({ reason: "load-failed", message: String(error) });
      setSettingsOpen(true);
    });
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
    await backend.saveSettings(normalizedSettingsForSave(settingsRef.current));
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
    if (securityIssue) {
      setSettingsOpen(true);
      setNotice("请先恢复加密设置");
      return;
    }
    const text = inputRef.current.trim();
    const images = capturesRef.current;
    if (!text && !images.length) { setNotice("本轮输入为空"); return; }
    if (!apiKeyConfigured || securityIssue) {
      setSettingsOpen(true);
      setOutput("请先在设置中填写 API Key。");
      return;
    }
    const currentText = text || resolvedCodingPrompt(settingsRef.current);
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
    if (!apiKeyConfigured || securityIssue) {
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
    if (securityIssue) {
      setSettingsOpen(true);
      setNotice("请先恢复加密设置");
      return;
    }
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
    if (securityIssue) {
      setSettingsOpen(true);
      setNotice("请先恢复加密设置");
      return;
    }
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
      await backend.saveSettings(normalizedSettingsForSave(settingsRef.current));
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

  function updatePromptMode(kind: "system" | "coding", mode: PromptMode) {
    setPromptIssue("");
    setSettings((current) => {
      const next = kind === "system"
        ? {
            ...current,
            systemPromptMode: mode,
            systemPrompt: mode === "custom"
              ? current.systemPrompt || DEFAULT_SYSTEM_PROMPT
              : mode === "default" ? null : current.systemPrompt,
          }
        : {
            ...current,
            codingPromptMode: mode,
            codingPrompt: mode === "custom"
              ? current.codingPrompt || DEFAULT_CODING_PROMPT
              : mode === "default" ? null : current.codingPrompt,
          };
      settingsRef.current = next;
      return next;
    });
  }

  function updatePromptValue(kind: "system" | "coding", value: string) {
    setPromptIssue("");
    setSettings((current) => {
      const next = kind === "system"
        ? { ...current, systemPrompt: value }
        : { ...current, codingPrompt: value };
      settingsRef.current = next;
      return next;
    });
  }

  function showError(label: string, error: unknown) {
    setOutput(`${label}：${String(error)}`); setStatus("error"); setNotice(label);
  }

  async function resetSecureSettings() {
    if (!window.confirm("将保留能够定位的损坏文件并重置密钥与全部设置。是否继续？")) return;
    setSecurityResetting(true);
    try {
      const result = await backend.resetSecureSettings();
      setSettings(result.snapshot.settings);
      settingsRef.current = result.snapshot.settings;
      setApiKeyConfigured(result.snapshot.apiKeyConfigured);
      setApiKeyDraft("");
      setApiKeyUpdate({ action: "keep" });
      setSecurityIssue(null);
      setNotice(result.quarantinePath ? `旧文件已保留：${result.quarantinePath}` : "安全设置已重置");
    } catch (error) {
      setNotice(`重置失败：${String(error)}`);
    } finally {
      setSecurityResetting(false);
    }
  }

  async function saveSettings() {
    const emptyCustom = settings.systemPromptMode === "custom" && !settings.systemPrompt?.trim()
      ? "系统 Prompt"
      : settings.codingPromptMode === "custom" && !settings.codingPrompt?.trim()
        ? "纯截图 Prompt"
        : "";
    if (emptyCustom) {
      setSettingsPage("api");
      setPromptIssue(`${emptyCustom}处于自定义模式，但内容为空。请选择推荐默认或明确禁用。`);
      setNotice("Prompt 设置尚未完成");
      return;
    }
    try {
      const normalized = normalizedSettingsForSave(settings);
      const snapshot = await backend.saveSettings(normalized, apiKeyUpdate);
      settingsRef.current = snapshot.settings;
      setSettings(snapshot.settings);
      setApiKeyConfigured(snapshot.apiKeyConfigured);
      setApiKeyDraft("");
      setApiKeyUpdate({ action: "keep" });
      setPromptIssue("");
      setSettingsOpen(false);
      setNotice("设置已保存");
    } catch (error) {
      setPromptIssue(String(error));
      showError("保存设置失败", error);
    }
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
      <TitleBar status={status} listening={listening} autoAnswer={autoAnswer}
        settingsOpen={settingsOpen} securityLocked={Boolean(securityIssue)} notice={notice} shortcutIssue={shortcutIssue}
        modifier={MOD} isMac={IS_MAC} onCapture={() => void takeRegionScreenshot()}
        onListeningToggle={() => void (listening ? stopListening() : startListening())}
        onAutoAnswerToggle={() => void toggleAutoAnswer()}
        onSettingsToggle={() => setSettingsOpen((value) => !value)} />

      {settingsOpen ? (
        <SettingsDialog page={settingsPage} locked={Boolean(securityIssue)}
          onPageChange={setSettingsPage} onClose={() => setSettingsOpen(false)}
          onSave={() => void saveSettings()}>
            {securityIssue ? <SecurityRecovery message={securityIssue.message} resetting={securityResetting}
              onReset={() => void resetSecureSettings()} /> : <>
            {settingsPage === "api" ? <ApiSettingsPage settings={settings}
              apiKeyConfigured={apiKeyConfigured} apiKeyDraft={apiKeyDraft}
              apiKeyPendingClear={apiKeyUpdate.action === "clear"} promptIssue={promptIssue}
              onSettingsChange={setSettings}
              onApiKeyDraftChange={(value) => {
                setApiKeyDraft(value);
                setApiKeyUpdate(value ? { action: "replace", value } : { action: "keep" });
              }}
              onApiKeyClear={() => { setApiKeyDraft(""); setApiKeyUpdate({ action: "clear" }); }}
              onPromptModeChange={updatePromptMode} onPromptValueChange={updatePromptValue} />
            : settingsPage === "audio" ? <AudioSettingsPage settings={settings}
              microphoneDevices={microphoneDevices} outputDevices={outputDevices}
              loading={devicesLoading} issue={deviceIssue} onSettingsChange={setSettings}
              onRefresh={() => void refreshAudioDevices(true)} />
            : <StorageSettingsPage settings={settings} info={storageInfo} loading={storageLoading}
              issue={storageIssue} onSettingsChange={setSettings}
              onRefresh={() => void refreshStorageInfo()} onChooseRoot={() => void chooseStorageRoot()}
              onRestoreDefault={() => void restoreDefaultStorageRoot()}
              onScheduleCleanup={() => void scheduleStorageCleanup()} />}
            </>}
        </SettingsDialog>
      ) : (
        <section className="workspace">
          <InputPanel listening={listening} autoAnswer={autoAnswer}
            pendingTranscriptions={pendingTranscriptions} voiceIssue={voiceIssue}>
              <ContextPanel value={settings.fixedContext} open={backgroundOpen}
                onOpenChange={setBackgroundOpen} onValueChange={updateFixedContext}
                onSave={() => void saveFixedContext()} />

              <TranscriptPanel entries={transcripts} answering={answering} listRef={transcriptListRef}
                onClear={clearTranscripts} onNewSession={startNewSession} onUpdate={updateTranscript}
                onAppend={appendToCurrentInput} onRemove={removeTranscript} />

              <section className="context-section draft-section">
                <div className="context-section-heading">
                  <span><b>本轮输入</b><em>发送成功后自动清空</em></span>
                </div>
                <CaptureStrip captures={captures} onRemove={removeCapture} />
                <textarea className="draft-input" value={input} onChange={(event) => {
                  inputRef.current = event.target.value;
                  setInput(event.target.value);
                }} placeholder="输入当前问题、补充要求或临时笔记…" />
                <div className="context-actions">
                  <button className="action-button context-clear" title={`清空本轮输入和截图 ${MOD}C`}
                    disabled={status === "working" || (!input.trim() && captures.length === 0)}
                    onClick={clearCurrentInput}><span>⌫ 清空</span><kbd>C</kbd></button>
                  <button className="action-button context-send" title={`发送 ${MOD}I`}
                    disabled={Boolean(securityIssue) || status === "working" || answering || (!input.trim() && captures.length === 0)}
                    onClick={() => void sendRequest()}><span>{answering ? "回答中…" : "发送"}</span><kbd>I</kbd></button>
                </div>
              </section>
          </InputPanel>
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
      <ShortcutFooter isMac={IS_MAC} />
    </main>
  );
}

export default App;
