import { useRef, useState, type MutableRefObject } from "react";
import { useTranslation } from "react-i18next";
import { errorMessage } from "../../services/backend";
import type { TranscriptEntry } from "../../shared/types";

type UseTranscriptionQueueOptions = {
  transcriptEpochRef: MutableRefObject<number>;
  listeningRef: MutableRefObject<boolean>;
  autoAnswerRef: MutableRefObject<boolean>;
  appendTranscripts: (entries: Omit<TranscriptEntry, "id" | "pinned">[]) => void;
  handleAutoChunk: (text: string, sessionId: number) => void;
  setVoiceIssue: (issue: string) => void;
  setNotice: (notice: string) => void;
};

export function useTranscriptionQueue(options: UseTranscriptionQueueOptions) {
  const { t } = useTranslation();
  const [pendingTranscriptions, setPendingTranscriptions] = useState(0);
  const queueRef = useRef<Promise<void>>(Promise.resolve());

  function handleResults(
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
    if (mine.status === "rejected") errors.push(t("errors.myVoice", { error: String(mine.reason) }));
    if (theirText) entries.push({ speaker: "them", text: theirText });
    if (theirs.status === "rejected") errors.push(t("errors.otherVoice", { error: String(theirs.reason) }));
    options.appendTranscripts(entries);
    options.setVoiceIssue(errors.join("；"));
    if (errors.length) options.setNotice(t("notices.partialTranscriptionFailed"));
    else if (entries.length) options.setNotice(t("notices.transcriptAdded"));
    else if (options.listeningRef.current) {
      options.setNotice(t("notices.waitingForSpeech", {
        mode: t(options.autoAnswerRef.current ? "shell.autoAnswer" : "shell.listen"),
      }));
    }
    if (autoAnswerForChunk && theirText) options.handleAutoChunk(theirText, sessionId);
  }

  function enqueue(
    mine: Promise<string>,
    theirs: Promise<string>,
    autoAnswerForChunk: boolean,
    sessionId: number,
  ): Promise<void> {
    const transcriptEpoch = options.transcriptEpochRef.current;
    setPendingTranscriptions((count) => count + 1);
    const results = Promise.allSettled([mine, theirs]);
    const task = queueRef.current
      .then(async () => {
        const [myResult, theirResult] = await results;
        if (transcriptEpoch !== options.transcriptEpochRef.current) return;
        handleResults(myResult, theirResult, autoAnswerForChunk, sessionId);
      })
      .catch((error) => {
        options.setVoiceIssue(t("errors.transcriptionTask", { error: errorMessage(error) }));
        options.setNotice(t("notices.transcriptionFailed"));
      });
    queueRef.current = task;
    return task.finally(() => setPendingTranscriptions((count) => Math.max(0, count - 1)));
  }

  return { pendingTranscriptions, enqueue };
}
