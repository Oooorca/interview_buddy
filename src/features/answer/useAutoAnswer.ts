import { useRef, type MutableRefObject } from "react";
import { useTranslation } from "react-i18next";
import type { CaptureResult, SecurityIssue } from "../../shared/types";
import { looksLikeInterviewPrompt, normalizedQuestion } from "./autoAnswer";

type PendingAutoAnswer = { question: string; sessionId: number; readyAt: number };

type UseAutoAnswerOptions = {
  apiKeyConfigured: boolean;
  securityIssue: SecurityIssue | null;
  voiceSessionRef: MutableRefObject<number>;
  autoAnswerRef: MutableRefObject<boolean>;
  capturesRef: MutableRefObject<CaptureResult[]>;
  answerBusyRef: MutableRefObject<boolean>;
  composePrompt: (text: string, source: "auto") => string;
  generate: (
    prompt: string,
    images: CaptureResult[],
    source: "auto",
    historyPrompt: string,
  ) => Promise<string | null>;
  setVoiceIssue: (issue: string) => void;
  setNotice: (notice: string) => void;
  onAnswerIdleRef: MutableRefObject<() => void>;
};

const SETTLE_MS = 700;
const DEDUPE_MS = 20_000;

export function useAutoAnswer(options: UseAutoAnswerOptions) {
  const { t } = useTranslation();
  const pendingRef = useRef<PendingAutoAnswer | null>(null);
  const timerRef = useRef<number | null>(null);
  const lastQuestionRef = useRef({ text: "", at: 0 });

  function clearPending() {
    pendingRef.current = null;
    if (timerRef.current) globalThis.clearTimeout(timerRef.current);
    timerRef.current = null;
  }

  function queue(question: string, sessionId: number) {
    const normalized = normalizedQuestion(question);
    const now = Date.now();
    const last = lastQuestionRef.current;
    if (normalized && normalized === last.text && now - last.at < DEDUPE_MS) {
      options.setNotice(t("notices.duplicateIgnored"));
      return;
    }
    pendingRef.current = { question, sessionId, readyAt: now + SETTLE_MS };
    if (timerRef.current) globalThis.clearTimeout(timerRef.current);
    timerRef.current = globalThis.setTimeout(() => {
      timerRef.current = null;
      void pump();
    }, SETTLE_MS);
    options.setNotice(t("notices.mergingQuestion"));
  }

  function handleChunk(text: string, sessionId: number) {
    const pending = pendingRef.current;
    if (pending?.sessionId === sessionId) queue(`${pending.question} ${text}`.trim(), sessionId);
    else if (looksLikeInterviewPrompt(text)) queue(text, sessionId);
  }

  async function pump(): Promise<void> {
    if (options.answerBusyRef.current) return;
    const pending = pendingRef.current;
    if (!pending) return;
    const waitMs = pending.readyAt - Date.now();
    if (waitMs > 0) {
      if (timerRef.current) globalThis.clearTimeout(timerRef.current);
      timerRef.current = globalThis.setTimeout(() => {
        timerRef.current = null;
        void pump();
      }, waitMs);
      return;
    }
    pendingRef.current = null;
    if (pending.sessionId !== options.voiceSessionRef.current || !options.autoAnswerRef.current) return;
    if (!options.apiKeyConfigured || options.securityIssue) {
      options.setVoiceIssue(t("notices.autoNeedsKey"));
      return;
    }
    lastQuestionRef.current = { text: normalizedQuestion(pending.question), at: Date.now() };
    const prompt = options.composePrompt(pending.question, "auto");
    const result = await options.generate(
      prompt,
      options.capturesRef.current,
      "auto",
      pending.question,
    );
    if (!result && pending.sessionId === options.voiceSessionRef.current && options.autoAnswerRef.current) {
      lastQuestionRef.current = { text: "", at: 0 };
      options.setVoiceIssue(t("notices.autoIncomplete"));
    }
  }

  function reset() {
    clearPending();
    lastQuestionRef.current = { text: "", at: 0 };
  }

  options.onAnswerIdleRef.current = () => { void pump(); };

  return { handleChunk, clearPending, reset };
}
