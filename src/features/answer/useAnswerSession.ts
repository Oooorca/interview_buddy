import { useRef, useState } from "react";
import type { AnswerHistoryEntry } from "../../shared/types";

export function useAnswerSession() {
  const [answerHistory, setAnswerHistory] = useState<AnswerHistoryEntry[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const answerHistoryRef = useRef(answerHistory);
  answerHistoryRef.current = answerHistory;
  return { answerHistory, answerHistoryRef, setAnswerHistory, historyIndex, setHistoryIndex };
}
