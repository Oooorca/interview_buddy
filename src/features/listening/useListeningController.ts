import { useEffect, useRef, useState, type MutableRefObject } from "react";
import { useTranslation } from "react-i18next";
import { backend, errorMessage } from "../../services/backend";
import type { AppSettings, AppStatus, CaptureResult, SecurityIssue, TranscriptEntry } from "../../shared/types";
import { useAutoAnswer } from "../answer/useAutoAnswer";
import { useAudioCapture } from "./useAudioCapture";
import { useTranscriptionQueue } from "./useTranscriptionQueue";
import { advanceVad, freshVadState, VAD_POLL_MS, type VadState } from "./vad";

type ListeningInterviewPort = {
  capturesRef: MutableRefObject<CaptureResult[]>;
  transcriptEpochRef: MutableRefObject<number>;
  appendTranscripts: (entries: Omit<TranscriptEntry, "id" | "pinned">[]) => void;
  composePrompt: (currentText: string, source: "manual" | "auto") => string;
};

type ListeningAnswerPort = {
  busyRef: MutableRefObject<boolean>;
  generate: (
    prompt: string,
    images: CaptureResult[],
    source: "manual" | "auto",
    historyPrompt?: string,
  ) => Promise<string | null>;
};

type UseListeningControllerOptions = {
  settingsRef: MutableRefObject<AppSettings>;
  apiKeyConfigured: boolean;
  securityIssue: SecurityIssue | null;
  interview: ListeningInterviewPort;
  answer: ListeningAnswerPort;
  setSettingsOpen: (open: boolean) => void;
  setStatus: (status: AppStatus) => void;
  setNotice: (notice: string) => void;
  onAnswerIdleRef: MutableRefObject<() => void>;
};

export function useListeningController(options: UseListeningControllerOptions) {
  const { t } = useTranslation();
  const [listening, setListening] = useState(false);
  const [autoAnswerEnabled, setAutoAnswerEnabled] = useState(false);
  const [voiceIssue, setVoiceIssue] = useState("");
  const listeningRef = useRef(false);
  const autoAnswerRef = useRef(false);
  const startingRef = useRef(false);
  const voiceSessionRef = useRef(0);
  const vadTimerRef = useRef<number | null>(null);
  const vadBusyRef = useRef(false);
  const micVadRef = useRef<VadState>(freshVadState());
  const systemVadRef = useRef<VadState>(freshVadState());

  listeningRef.current = listening;
  autoAnswerRef.current = autoAnswerEnabled;

  const audio = useAudioCapture({
    settingsRef: options.settingsRef,
    setVoiceIssue,
  });
  const autoAnswer = useAutoAnswer({
    apiKeyConfigured: options.apiKeyConfigured,
    securityIssue: options.securityIssue,
    voiceSessionRef,
    autoAnswerRef,
    capturesRef: options.interview.capturesRef,
    answerBusyRef: options.answer.busyRef,
    composePrompt: options.interview.composePrompt,
    generate: options.answer.generate,
    setVoiceIssue,
    setNotice: options.setNotice,
    onAnswerIdleRef: options.onAnswerIdleRef,
  });
  const transcription = useTranscriptionQueue({
    transcriptEpochRef: options.interview.transcriptEpochRef,
    listeningRef,
    autoAnswerRef,
    appendTranscripts: options.interview.appendTranscripts,
    handleAutoChunk: autoAnswer.handleChunk,
    setVoiceIssue,
    setNotice: options.setNotice,
  });

  useEffect(() => () => {
    if (vadTimerRef.current) globalThis.clearInterval(vadTimerRef.current);
    autoAnswer.clearPending();
  }, []);

  async function runVadTick(sessionId: number) {
    if (vadBusyRef.current || !listeningRef.current || voiceSessionRef.current !== sessionId) return;
    vadBusyRef.current = true;
    try {
      const systemLevel = audio.systemActiveRef.current
        ? await backend.systemAudioLevel().catch(() => 0)
        : 0;
      if (!listeningRef.current || voiceSessionRef.current !== sessionId) return;
      const now = performance.now();
      const micAction = audio.micRef.current
        ? advanceVad(micVadRef.current, audio.microphoneLevel(), now)
        : null;
      const systemAction = audio.systemActiveRef.current
        ? advanceVad(systemVadRef.current, systemLevel, now)
        : null;
      if (micAction === "discard") await audio.rotateAndDiscard();
      if (systemAction === "discard") await backend.discardSystemAudioChunk();
      if (micAction === "flush" || systemAction === "flush") {
        const mine = micAction === "flush" ? audio.rotateAndTranscribe() : Promise.resolve("");
        const theirs = systemAction === "flush"
          ? backend.transcribeSystemAudioChunk()
          : Promise.resolve("");
        void transcription.enqueue(mine, theirs, autoAnswerRef.current, sessionId);
      }
    } catch (error) {
      setVoiceIssue(t("errors.segmentation", { error: errorMessage(error) }));
      options.setNotice(t("notices.segmentationAbnormal"));
    } finally {
      vadBusyRef.current = false;
    }
  }

  function startVadMonitor(sessionId: number) {
    if (vadTimerRef.current) globalThis.clearInterval(vadTimerRef.current);
    micVadRef.current = freshVadState();
    systemVadRef.current = freshVadState();
    vadTimerRef.current = globalThis.setInterval(() => { void runVadTick(sessionId); }, VAD_POLL_MS);
  }

  async function start(enableAutoAnswer = false) {
    if (options.securityIssue) {
      options.setSettingsOpen(true);
      options.setNotice(t("notices.recoverFirst"));
      return;
    }
    if (listeningRef.current || startingRef.current) return;
    startingRef.current = true;
    try {
      await audio.startDevices();
      const sessionId = voiceSessionRef.current + 1;
      voiceSessionRef.current = sessionId;
      listeningRef.current = true;
      autoAnswerRef.current = enableAutoAnswer;
      setListening(true);
      setAutoAnswerEnabled(enableAutoAnswer);
      options.setStatus("ready");
      options.setNotice(t("notices.started", {
        mode: t(enableAutoAnswer ? "shell.autoAnswer" : "shell.listen"),
      }));
      startVadMonitor(sessionId);
    } catch (error) {
      autoAnswerRef.current = false;
      setAutoAnswerEnabled(false);
      setVoiceIssue(errorMessage(error));
      options.setStatus("error");
      options.setNotice(t("notices.startFailed"));
    } finally {
      startingRef.current = false;
    }
  }

  async function stop() {
    if (!listeningRef.current) {
      options.setNotice(t("notices.notListening"));
      return;
    }
    const sessionId = voiceSessionRef.current;
    listeningRef.current = false;
    autoAnswerRef.current = false;
    autoAnswer.clearPending();
    setListening(false);
    setAutoAnswerEnabled(false);
    if (vadTimerRef.current) globalThis.clearInterval(vadTimerRef.current);
    vadTimerRef.current = null;
    const { mic, hadSystem } = audio.detach();
    options.setNotice(t("notices.finishingTranscription"));
    await transcription.enqueue(
      mic ? audio.transcribeMic(mic, true) : Promise.resolve(""),
      hadSystem ? backend.stopSystemAudio() : Promise.resolve(""),
      false,
      sessionId,
    );
    options.setNotice(t("notices.listeningStopped"));
  }

  async function toggleAutoAnswer() {
    if (options.securityIssue) {
      options.setSettingsOpen(true);
      options.setNotice(t("notices.recoverFirst"));
      return;
    }
    if (autoAnswerRef.current) {
      autoAnswerRef.current = false;
      autoAnswer.clearPending();
      setAutoAnswerEnabled(false);
      options.setNotice(t("notices.autoOff"));
    } else if (!listeningRef.current) {
      await start(true);
    } else {
      autoAnswerRef.current = true;
      setAutoAnswerEnabled(true);
      options.setNotice(t("notices.autoOn"));
    }
  }

  return {
    listening,
    autoAnswer: autoAnswerEnabled,
    pendingTranscriptions: transcription.pendingTranscriptions,
    voiceIssue,
    listeningRef,
    start,
    stop,
    toggleAutoAnswer,
    clearPendingAuto: autoAnswer.clearPending,
    resetSession: autoAnswer.reset,
  };
}
