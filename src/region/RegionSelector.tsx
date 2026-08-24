import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { backend } from "../services/backend";

type Point = { x: number; y: number };
type Selection = { x: number; y: number; width: number; height: number };

function clampPoint(event: ReactPointerEvent<HTMLElement>): Point {
  return {
    x: Math.max(0, Math.min(window.innerWidth, event.clientX)),
    y: Math.max(0, Math.min(window.innerHeight, event.clientY)),
  };
}

function rectangle(start: Point, end: Point): Selection {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y),
  };
}

export default function RegionSelector() {
  const [start, setStart] = useState<Point | null>(null);
  const [current, setCurrent] = useState<Point | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [hint, setHint] = useState("按住鼠标拖拽选择区域 · Esc 取消");
  const pointerIdRef = useRef<number | null>(null);
  const selection = useMemo(
    () => start && current ? rectangle(start, current) : null,
    [start, current],
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !submitting) {
        setSubmitting(true);
        void backend.cancelRegionSelection().catch(() => setSubmitting(false));
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [submitting]);

  function begin(event: ReactPointerEvent<HTMLDivElement>) {
    if (submitting || event.button !== 0) return;
    const point = clampPoint(event);
    pointerIdRef.current = event.pointerId;
    event.currentTarget.setPointerCapture(event.pointerId);
    setStart(point);
    setCurrent(point);
    setHint("松开鼠标完成截图 · Esc 取消");
  }

  function move(event: ReactPointerEvent<HTMLDivElement>) {
    if (submitting || pointerIdRef.current !== event.pointerId || !start) return;
    setCurrent(clampPoint(event));
  }

  function finish(event: ReactPointerEvent<HTMLDivElement>) {
    if (submitting || pointerIdRef.current !== event.pointerId || !start) return;
    const finalSelection = rectangle(start, clampPoint(event));
    pointerIdRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (finalSelection.width < 3 || finalSelection.height < 3) {
      setStart(null);
      setCurrent(null);
      setHint("区域太小，请重新拖拽 · Esc 取消");
      return;
    }
    setCurrent({
      x: finalSelection.x + finalSelection.width,
      y: finalSelection.y + finalSelection.height,
    });
    setSubmitting(true);
    setHint("正在截取所选区域…");
    void backend.completeRegionSelection(finalSelection).catch((error) => {
      setSubmitting(false);
      setHint(`截图失败：${String(error)}`);
    });
  }

  function cancelWithRightClick(event: ReactMouseEvent<HTMLDivElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    void backend.cancelRegionSelection().catch(() => setSubmitting(false));
  }

  return (
    <main
      className={`region-selector${submitting ? " submitting" : ""}`}
      onPointerDown={begin}
      onPointerMove={move}
      onPointerUp={finish}
      onPointerCancel={() => {
        pointerIdRef.current = null;
        setStart(null);
        setCurrent(null);
      }}
      onContextMenu={cancelWithRightClick}
    >
      <div className="region-selector-hint">{hint}</div>
      {selection && selection.width > 0 && selection.height > 0 && (
        <div
          className="region-selection"
          style={{
            left: selection.x,
            top: selection.y,
            width: selection.width,
            height: selection.height,
          }}
        >
          <span>{Math.round(selection.width)} × {Math.round(selection.height)}</span>
        </div>
      )}
    </main>
  );
}
