import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

type InputPanelProps = {
  listening: boolean;
  autoAnswer: boolean;
  pendingTranscriptions: number;
  voiceIssue: string;
  children: ReactNode;
};

export function InputPanel({ listening, autoAnswer, pendingTranscriptions, voiceIssue, children }: InputPanelProps) {
  const { t } = useTranslation();
  return <div className="pane input-pane">
    <header><span>INTERVIEW INPUT</span><div className="voice-meta">
      {listening && <i className="voice-live">● {autoAnswer ? t("shell.autoAnswer") : t("shell.listening")}</i>}
      {pendingTranscriptions > 0 && <i className="voice-pending">{t("audio.transcribing", { count: pendingTranscriptions })}</i>}
      {voiceIssue && <i className="voice-error" title={voiceIssue}>{t("audio.abnormal")}</i>}
    </div></header>
    <div className="context-stack">{children}</div>
  </div>;
}
