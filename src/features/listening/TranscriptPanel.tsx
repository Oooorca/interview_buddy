import type { RefObject } from "react";

export type TranscriptEntry = {
  id: string;
  speaker: "me" | "them";
  text: string;
  pinned: boolean;
};

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
  return <section className="context-section transcript-section">
    <div className="context-section-heading">
      <span><b>实时转写</b><em>{props.entries.length ? `${props.entries.length} 条` : "等待语音"}</em></span>
      <div className="section-tools">
        <button disabled={!props.entries.length} onClick={props.onClear} title="仅清空实时转写">清空转写</button>
        <button disabled={props.answering} onClick={props.onNewSession} title="清空转写、本轮输入、截图和会话回答；保留固定背景">新会话</button>
      </div>
    </div>
    <div className="transcript-list" ref={props.listRef}>
      {!props.entries.length && <div className="transcript-empty">开启听写后，我和对方的语音会分开显示在这里</div>}
      {props.entries.map((entry) => (
        <article className={`transcript-entry ${entry.speaker} ${entry.pinned ? "pinned" : ""}`} key={entry.id}>
          <span className="speaker-badge">{entry.speaker === "me" ? "我" : "对方"}</span>
          <textarea rows={2} value={entry.text} aria-label={`${entry.speaker === "me" ? "我" : "对方"}的转写内容`}
            onChange={(event) => props.onUpdate(entry.id, { text: event.target.value })} />
          <div className="transcript-tools">
            <button className={entry.pinned ? "active" : ""} title={entry.pinned ? "取消固定" : "固定到回答上下文"}
              onClick={() => props.onUpdate(entry.id, { pinned: !entry.pinned })}>⌖</button>
            <button title="加入本轮输入" onClick={() => props.onAppend(`${entry.speaker === "me" ? "我" : "对方"}：${entry.text}`)}>＋</button>
            <button title="删除这条转写" onClick={() => props.onRemove(entry.id)}>×</button>
          </div>
        </article>
      ))}
    </div>
  </section>;
}
