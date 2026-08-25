import { useEffect, useRef, useState, type MutableRefObject } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { backend, errorMessage } from "../../services/backend";
import { defaultPromptsFor, effectiveAnswerLanguage } from "../../shared/settings";
import type {
  AnswerDelta,
  AnswerHistoryEntry,
  AppSettings,
  AppStatus,
  CaptureResult,
  ConversationMessage,
  SecurityIssue,
} from "../../shared/types";
import { useAnswerSession } from "./useAnswerSession";

const MAX_ANSWER_HISTORY = 30;

type InterviewAnswerPort = {
  inputRef: MutableRefObject<string>;
  capturesRef: MutableRefObject<CaptureResult[]>;
  composePrompt: (currentText: string, source: "manual" | "auto") => string;
  consumeTurnIfUnchanged: (text: string, captures: CaptureResult[]) => boolean;
};

type UseAnswerControllerOptions = {
  settingsRef: MutableRefObject<AppSettings>;
  apiKeyConfigured: boolean;
  securityIssue: SecurityIssue | null;
  interview: InterviewAnswerPort;
  setSettingsOpen: (open: boolean) => void;
  setStatus: (status: AppStatus) => void;
  setNotice: (notice: string) => void;
  onIdleRef: MutableRefObject<() => void>;
};

function resolvedCodingPrompt(settings: AppSettings): string {
  if (settings.codingPromptMode === "default") return defaultPromptsFor(settings).coding;
  if (settings.codingPromptMode === "custom") return settings.codingPrompt || "";
  return "";
}

export function useAnswerController(options: UseAnswerControllerOptions) {
  const { t } = useTranslation();
  const {
    settingsRef, apiKeyConfigured, securityIssue, interview,
    setSettingsOpen, setStatus, setNotice, onIdleRef,
  } = options;
  const { answerHistory, answerHistoryRef, setAnswerHistory, historyIndex, setHistoryIndex } = useAnswerSession();
  const [output, setOutput] = useState("");
  const [answering, setAnswering] = useState(false);
  const busyRef = useRef(false);
  const activeRequestIdRef = useRef<string | null>(null);
  const streamStartedRef = useRef(false);

  useEffect(() => {
    let active = true;
    let dispose: (() => void) | undefined;
    void listen<AnswerDelta>("answer-stream-delta", ({ payload }) => {
      if (payload.requestId !== activeRequestIdRef.current) return;
      setOutput((current) => {
        if (!streamStartedRef.current) {
          streamStartedRef.current = true;
          return payload.delta;
        }
        return current + payload.delta;
      });
    }).then((unlisten) => {
      if (active) dispose = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      dispose?.();
    };
  }, []);

  function conversationMessages(): ConversationMessage[] {
    return answerHistoryRef.current.flatMap((entry) => [
      { role: "user" as const, content: entry.prompt },
      { role: "assistant" as const, content: entry.answer },
    ]);
  }

  function rememberAnswer(prompt: string, answer: string) {
    const entry: AnswerHistoryEntry = { prompt, answer };
    setAnswerHistory((current) => {
      const next = [...current, entry].slice(-MAX_ANSWER_HISTORY);
      answerHistoryRef.current = next;
      setHistoryIndex(next.length - 1);
      return next;
    });
  }

  async function generate(
    prompt: string,
    images: CaptureResult[],
    source: "manual" | "auto",
    historyPrompt = prompt,
  ): Promise<string | null> {
    if (busyRef.current) {
      setNotice(t("notices.alreadyGenerating"));
      return null;
    }
    const requestId = crypto.randomUUID();
    const previousOutput = output;
    activeRequestIdRef.current = requestId;
    streamStartedRef.current = false;
    busyRef.current = true;
    setAnswering(true);
    setStatus("working");
    setNotice(t(source === "auto" ? "notices.autoGenerating" : "notices.generating"));
    try {
      const result = await backend.ask(
        requestId,
        prompt,
        images.map((image) => image.dataUrl),
        conversationMessages(),
        effectiveAnswerLanguage(settingsRef.current.answerLanguage, settingsRef.current.uiLanguage),
      );
      if (activeRequestIdRef.current !== requestId) return null;
      const answer = result.text.trim();
      if (result.cancelled) {
        if (!streamStartedRef.current) setOutput(previousOutput);
        setStatus("ready");
        setNotice(t("notices.stopped"));
        return null;
      }
      if (!answer) throw new Error(t("notices.emptyModelAnswer"));
      setOutput(answer);
      rememberAnswer(historyPrompt, answer);
      setStatus("ready");
      setNotice(t(source === "auto" ? "notices.autoUpdated" : "notices.answerComplete"));
      return answer;
    } catch (error) {
      setStatus("error");
      setNotice(t("notices.requestFailed", { error: errorMessage(error) }));
      if (!streamStartedRef.current) setOutput(previousOutput);
      return null;
    } finally {
      if (activeRequestIdRef.current === requestId) activeRequestIdRef.current = null;
      busyRef.current = false;
      setAnswering(false);
      onIdleRef.current();
    }
  }

  async function sendCurrentTurn() {
    if (securityIssue) {
      setSettingsOpen(true);
      setNotice(t("notices.recoverFirst"));
      return;
    }
    const text = interview.inputRef.current.trim();
    const images = interview.capturesRef.current;
    if (!text && !images.length) {
      setNotice(t("notices.emptyTurn"));
      return;
    }
    if (!apiKeyConfigured) {
      setSettingsOpen(true);
      setOutput(t("notices.configureKey"));
      return;
    }
    const currentText = text || resolvedCodingPrompt(settingsRef.current);
    const prompt = interview.composePrompt(currentText, "manual");
    const answer = await generate(prompt, images, "manual", currentText);
    if (!answer) return;
    const preserved = interview.consumeTurnIfUnchanged(text, images);
    setNotice(t(preserved ? "notices.completePreserved" : "notices.completeCleared"));
  }

  async function stop() {
    const requestId = activeRequestIdRef.current;
    if (!requestId) return;
    setNotice(t("notices.stopping"));
    try { await backend.cancelAnswer(requestId); }
    catch (error) { setNotice(t("notices.stopFailed", { error: errorMessage(error) })); }
  }

  function showHistoryEntry(index: number) {
    const entry = answerHistoryRef.current[index];
    if (!entry || answering) return;
    setHistoryIndex(index);
    setOutput(entry.answer);
    setStatus("ready");
    setNotice(t("notices.history", { current: index + 1, total: answerHistoryRef.current.length }));
  }

  function reset() {
    answerHistoryRef.current = [];
    setAnswerHistory([]);
    setHistoryIndex(-1);
    setOutput("");
    setStatus("ready");
  }

  return {
    output, setOutput, answering, answerHistory, historyIndex, busyRef,
    generate, sendCurrentTurn, stop, showHistoryEntry, reset,
  };
}
