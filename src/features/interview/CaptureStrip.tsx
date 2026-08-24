import type { CaptureResult } from "../../shared/types";

export function CaptureStrip({ captures, onRemove }: { captures: CaptureResult[]; onRemove: (index: number) => void }) {
  return <div className="attachments">
    {captures.map((capture, index) => (
      <button className="attachment-card" key={index} onClick={() => onRemove(index)} title="点击移除">
        <img src={capture.dataUrl} alt={`image_${index + 1}`} />
        <span className="attachment-label">image_{index + 1}</span>
        <span className="attachment-remove">×</span>
      </button>
    ))}
  </div>;
}
