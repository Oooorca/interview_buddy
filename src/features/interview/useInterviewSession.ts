import { useEffect, useRef, useState, type MutableRefObject } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import i18n from "../../i18n";
import { backend } from "../../services/backend";
import { effectiveAnswerLanguage } from "../../shared/settings";
import type { AppSettings, AppStatus, CaptureResult, TranscriptEntry } from "../../shared/types";

const MAX_TRANSCRIPT_ENTRIES = 80;
const TRANSCRIPT_CONTEXT_ENTRIES = 16;
const TRANSCRIPT_CONTEXT_CHARS = 8_000;

type UseInterviewSessionOptions = {
  settingsRef: MutableRefObject<AppSettings>;
  setStatus: (status: AppStatus) => void;
  setNotice: (notice: string) => void;
  showErrorRef: MutableRefObject<(label: string, error: unknown) => void>;
};

export function useInterviewSession({ settingsRef, setStatus, setNotice, showErrorRef }: UseInterviewSessionOptions) {
  const { t } = useTranslation();
  const [input, setInput] = useState("");
  const [captures, setCaptures] = useState<CaptureResult[]>([]);
  const [transcripts, setTranscripts] = useState<TranscriptEntry[]>([]);
  const [backgroundOpen, setBackgroundOpen] = useState(false);
  const inputRef = useRef(input);
  const capturesRef = useRef(captures);
  const transcriptsRef = useRef(transcripts);
  const transcriptEpochRef = useRef(0);
  const transcriptListRef = useRef<HTMLDivElement | null>(null);
  const transcriptCountRef = useRef(0);
  const captureBusyRef = useRef(false);

  inputRef.current = input;
  capturesRef.current = captures;
  transcriptsRef.current = transcripts;

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
    let active = true;
    const disposers: Array<() => void> = [];
    Promise.all([
      listen<CaptureResult>("region-captured", ({ payload }) => {
        captureBusyRef.current = false;
        addCapture(payload);
      }),
      listen<string>("region-capture-error", ({ payload }) => {
        captureBusyRef.current = false;
        showErrorRef.current(i18n.t("notices.captureFailed"), payload);
      }),
      listen("region-capture-cancelled", () => {
        captureBusyRef.current = false;
        setStatus("ready");
        setNotice(i18n.t("notices.captureCancelled"));
      }),
    ]).then((items) => {
      if (active) disposers.push(...items);
      else items.forEach((dispose) => dispose());
    });
    return () => {
      active = false;
      disposers.forEach((dispose) => dispose());
    };
  }, [setNotice, setStatus, showErrorRef]);

  function setInputValue(value: string) {
    inputRef.current = value;
    setInput(value);
  }

  function appendToCurrentInput(text: string) {
    const addition = text.trim();
    if (!addition) return;
    setInputValue(inputRef.current.trim() ? `${inputRef.current.trim()}\n${addition}` : addition);
    setNotice(t("notices.addedToTurn"));
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
    const answerT = i18n.getFixedT(effectiveAnswerLanguage(
      settingsRef.current.answerLanguage,
      settingsRef.current.uiLanguage,
    ));
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
      .map((entry) => `${answerT(entry.speaker === "me" ? "promptContext.me" : "promptContext.other")}: ${entry.text.trim()}`)
      .join("\n");
  }

  function composePrompt(currentText: string, source: "manual" | "auto"): string {
    const sections: string[] = [];
    const fixed = settingsRef.current.fixedContext.trim();
    const transcript = transcriptContext();
    const draft = inputRef.current.trim();
    const answerT = i18n.getFixedT(effectiveAnswerLanguage(
      settingsRef.current.answerLanguage,
      settingsRef.current.uiLanguage,
    ));
    if (fixed) sections.push(`[${answerT("promptContext.fixed")}]\n${fixed}`);
    if (transcript) sections.push(`[${answerT("promptContext.recent")}]\n${transcript}`);
    if (source === "auto") {
      sections.push(`[${answerT("promptContext.question")}]\n${currentText.trim()}`);
      if (draft) sections.push(`[${answerT("promptContext.supplement")}]\n${draft}`);
    } else {
      sections.push(`[${answerT("promptContext.turn")}]\n${currentText.trim()}`);
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
    setNotice(t("notices.captureAdded", { number: imageNumber }));
  }

  function removeCapture(index: number) {
    setCaptures((current) => {
      const next = current.filter((__, itemIndex) => itemIndex !== index);
      capturesRef.current = next;
      return next;
    });
  }

  async function takeRegionScreenshot() {
    if (captureBusyRef.current) return;
    captureBusyRef.current = true;
    setStatus("working");
    setNotice(t("notices.selectRegion"));
    try { await backend.openRegionSelector(); }
    catch (error) {
      captureBusyRef.current = false;
      showErrorRef.current(t("notices.openCaptureFailed"), error);
    }
  }

  function clearCurrentInput() {
    setInputValue("");
    capturesRef.current = [];
    setCaptures([]);
    setNotice(t("notices.currentCleared"));
  }

  function clearTranscripts() {
    transcriptEpochRef.current += 1;
    transcriptsRef.current = [];
    setTranscripts([]);
    setNotice(t("notices.transcriptsCleared"));
  }

  function resetSession() {
    transcriptEpochRef.current += 1;
    transcriptsRef.current = [];
    inputRef.current = "";
    capturesRef.current = [];
    setTranscripts([]);
    setInput("");
    setCaptures([]);
  }

  function consumeTurnIfUnchanged(originalText: string, originalCaptures: CaptureResult[]) {
    let preserved = false;
    if (inputRef.current.trim() === originalText) setInputValue("");
    else preserved = true;
    const capturesUnchanged = capturesRef.current.length === originalCaptures.length
      && capturesRef.current.every((capture, index) => capture.dataUrl === originalCaptures[index]?.dataUrl);
    if (capturesUnchanged) {
      capturesRef.current = [];
      setCaptures([]);
    } else preserved = true;
    return preserved;
  }

  return {
    input, captures, transcripts, backgroundOpen, setBackgroundOpen,
    inputRef, capturesRef, transcriptsRef, transcriptEpochRef, transcriptListRef,
    setInputValue, appendToCurrentInput, appendTranscripts, updateTranscript, removeTranscript,
    composePrompt, removeCapture, takeRegionScreenshot,
    clearCurrentInput, clearTranscripts, resetSession, consumeTurnIfUnchanged,
  };
}
