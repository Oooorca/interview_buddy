import type { ReactNode } from "react";

type InputPanelProps = {
  listening: boolean;
  autoAnswer: boolean;
  pendingTranscriptions: number;
  voiceIssue: string;
  children: ReactNode;
};

export function InputPanel({ listening, autoAnswer, pendingTranscriptions, voiceIssue, children }: InputPanelProps) {
  return <div className="pane input-pane">
    <header><span>INTERVIEW INPUT</span><div className="voice-meta">
      {listening && <i className="voice-live">● {autoAnswer ? "自动回答" : "听写中"}</i>}
      {pendingTranscriptions > 0 && <i className="voice-pending">转写 {pendingTranscriptions}</i>}
      {voiceIssue && <i className="voice-error" title={voiceIssue}>音频异常</i>}
    </div></header>
    <div className="context-stack">{children}</div>
  </div>;
}
