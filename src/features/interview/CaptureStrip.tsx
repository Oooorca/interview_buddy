import type { CaptureResult } from "../../shared/types";
import { useTranslation } from "react-i18next";

export function CaptureStrip({ captures, onRemove }: { captures: CaptureResult[]; onRemove: (index: number) => void }) {
  const { t } = useTranslation();
  return <div className="attachments">
    {captures.map((capture, index) => (
      <button className="attachment-card" key={index} onClick={() => onRemove(index)} title={t("context.attachmentRemove")}>
        <img src={capture.dataUrl} alt={`image_${index + 1}`} />
        <span className="attachment-label">image_{index + 1}</span>
        <span className="attachment-remove">×</span>
      </button>
    ))}
  </div>;
}
