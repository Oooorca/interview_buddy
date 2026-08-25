import type { RefObject } from "react";
import { useTranslation } from "react-i18next";
import type { TranscriptEntry } from "../../shared/types";

type TranscriptPanelProps = {
  entries: TranscriptEntry[];
  answering: boolean;
  listRef: RefObject<HTMLDivElement | null>;
  onClear: () => void;
  onNewSession: () => void;
  onUpdate: (id: string, update: Partial<Pick<TranscriptEntry, "text" | "pinned">>) => void;
  onAppend: (text: string) => void;
  onRemove: (id: string) => void;
};

export function TranscriptPanel(props: TranscriptPanelProps) {
  const { t } = useTranslation();
  const speakerName = (speaker: TranscriptEntry["speaker"]) => t(speaker === "me" ? "transcript.me" : "transcript.other");
  return <section className="context-section transcript-section">
    <div className="context-section-heading">
      <span><b>{t("transcript.title")}</b><em>{props.entries.length ? t("transcript.count", { count: props.entries.length }) : t("transcript.waiting")}</em></span>
      <div className="section-tools">
        <button disabled={!props.entries.length} onClick={props.onClear} title={t("transcript.clearTitle")}>{t("transcript.clear")}</button>
        <button disabled={props.answering} onClick={props.onNewSession} title={t("transcript.newSessionTitle")}>{t("transcript.newSession")}</button>
      </div>
    </div>
    <div className="transcript-list" ref={props.listRef}>
      {!props.entries.length && <div className="transcript-empty">{t("transcript.empty")}</div>}
      {props.entries.map((entry) => (
        <article className={`transcript-entry ${entry.speaker} ${entry.pinned ? "pinned" : ""}`} key={entry.id}>
          <span className="speaker-badge">{speakerName(entry.speaker)}</span>
          <textarea rows={2} value={entry.text} aria-label={t("transcript.contentLabel", { speaker: speakerName(entry.speaker) })}
            onChange={(event) => props.onUpdate(entry.id, { text: event.target.value })} />
          <div className="transcript-tools">
            <button className={entry.pinned ? "active" : ""} title={entry.pinned ? t("transcript.unpin") : t("transcript.pin")}
              onClick={() => props.onUpdate(entry.id, { pinned: !entry.pinned })}>⌖</button>
            <button title={t("transcript.append")} onClick={() => props.onAppend(`${speakerName(entry.speaker)}: ${entry.text}`)}>＋</button>
            <button title={t("transcript.delete")} onClick={() => props.onRemove(entry.id)}>×</button>
          </div>
        </article>
      ))}
    </div>
  </section>;
}
