import { useTranslation } from "react-i18next";
import { AnswerView } from "../features/answer/AnswerView";
import type { useAnswerController } from "../features/answer/useAnswerController";
import { CaptureStrip } from "../features/interview/CaptureStrip";
import { ContextPanel } from "../features/interview/ContextPanel";
import { InputPanel } from "../features/interview/InputPanel";
import type { useInterviewSession } from "../features/interview/useInterviewSession";
import { TranscriptPanel } from "../features/listening/TranscriptPanel";
import type { useListeningController } from "../features/listening/useListeningController";
import type { useAppSettings } from "../features/settings/useAppSettings";
import { appPlatform } from "../platform";
import type { AppStatus } from "../shared/types";

type InterviewWorkspaceProps = {
  settings: ReturnType<typeof useAppSettings>;
  interview: ReturnType<typeof useInterviewSession>;
  answer: ReturnType<typeof useAnswerController>;
  listening: ReturnType<typeof useListeningController>;
  status: AppStatus;
  onClearTranscripts: () => void;
  onNewSession: () => void;
};

export function InterviewWorkspace(props: InterviewWorkspaceProps) {
  const { t } = useTranslation();
  const { settings, interview, answer, listening, status } = props;
  return <section className="workspace">
    <InputPanel listening={listening.listening} autoAnswer={listening.autoAnswer}
      pendingTranscriptions={listening.pendingTranscriptions} voiceIssue={listening.voiceIssue}>
        <ContextPanel value={settings.settings.fixedContext} open={interview.backgroundOpen}
          onOpenChange={interview.setBackgroundOpen} onValueChange={settings.updateFixedContext}
          onSave={() => void settings.saveFixedContext()} />

        <TranscriptPanel entries={interview.transcripts} answering={answer.answering}
          listRef={interview.transcriptListRef} onClear={props.onClearTranscripts}
          onNewSession={props.onNewSession} onUpdate={interview.updateTranscript}
          onAppend={interview.appendToCurrentInput} onRemove={interview.removeTranscript} />

        <section className="context-section draft-section">
          <div className="context-section-heading">
            <span><b>{t("context.currentTurn")}</b><em>{t("context.clearsAfterSend")}</em></span>
          </div>
          <CaptureStrip captures={interview.captures} onRemove={interview.removeCapture} />
          <textarea className="draft-input" value={interview.input}
            onChange={(event) => interview.setInputValue(event.target.value)}
            placeholder={t("context.draftPlaceholder")} />
          <div className="context-actions">
            <button className="action-button context-clear"
              title={t("context.clearTitle", { shortcut: `${appPlatform.shortcutModifier}C` })}
              disabled={status === "working" || (!interview.input.trim() && interview.captures.length === 0)}
              onClick={interview.clearCurrentInput}>
              <span>⌫ {t("actions.clear")}</span><kbd>C</kbd>
            </button>
            <button className="action-button context-send"
              title={t("context.sendTitle", { shortcut: `${appPlatform.shortcutModifier}I` })}
              disabled={Boolean(settings.securityIssue) || status === "working" || answer.answering
                || (!interview.input.trim() && interview.captures.length === 0)}
              onClick={() => void answer.sendCurrentTurn()}>
              <span>{answer.answering ? t("shell.sending") : t("shell.send")}</span><kbd>I</kbd>
            </button>
          </div>
        </section>
    </InputPanel>

    <div className="pane output-pane">
      <header className="response-header">
        <span>{t("answer.response").toUpperCase()}</span>
        <div className="response-tools">
          {answer.answering
            ? <button className="stop-answer" onClick={() => void answer.stop()}>■ {t("answer.stop")}</button>
            : <>
                <button disabled={answer.historyIndex <= 0} title={t("answer.previous")}
                  onClick={() => answer.showHistoryEntry(answer.historyIndex - 1)}>‹</button>
                <span>{answer.answerHistory.length
                  ? `${answer.historyIndex + 1}/${answer.answerHistory.length}` : "0/0"}</span>
                <button disabled={answer.historyIndex < 0
                    || answer.historyIndex >= answer.answerHistory.length - 1}
                  title={t("answer.next")}
                  onClick={() => answer.showHistoryEntry(answer.historyIndex + 1)}>›</button>
                <button disabled={!answer.answerHistory.length && !interview.transcripts.length
                    && !interview.input.trim() && !interview.captures.length}
                  title={t("answer.newSession")} onClick={props.onNewSession}>↺</button>
              </>}
          <i className={answer.answering ? "working" : status}>
            {answer.answering ? "STREAMING" : status.toUpperCase()}
          </i>
        </div>
      </header>
      <AnswerView content={answer.output} />
    </div>
  </section>;
}
