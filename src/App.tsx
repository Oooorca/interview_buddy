import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { backend, captureWithoutOverlay } from "./tauri";
import { defaultSettings, type AppSettings, type CaptureResult } from "./types";

type Status = "ready" | "working" | "error";
type ListenMode = "idle" | "manual" | "auto";
type MicSession = { recorder: MediaRecorder; stream: MediaStream; chunks: Blob[]; mimeType: string };
const AUTO_CHUNK_MS = 20_000;
const IS_MAC = navigator.userAgent.includes("Mac");
const MOD = IS_MAC ? "⌘⇧" : "Ctrl+Shift+";

function describeMicrophoneError(error: unknown): string {
  const detail = String(error);
  if (IS_MAC && /NotAllowedError|PermissionDenied|permission denied|not allowed/i.test(detail)) {
    return "麦克风：macOS 拒绝了当前构建。请在“隐私与安全性 → 麦克风”中重新开关 Interview Buddy，并彻底退出后重启应用；本地临时签名在重新构建后可能需要重新授权。";
  }
  return `麦克风：${detail}`;
}

function App() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("waiting...");
  const [captures, setCaptures] = useState<CaptureResult[]>([]);
  const [status, setStatus] = useState<Status>("ready");
  const [mode, setMode] = useState<ListenMode>("idle");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [notice, setNotice] = useState("共享不可见");

  const micRef = useRef<MicSession | null>(null);
  const systemActiveRef = useRef(false);
  const modeRef = useRef<ListenMode>("idle");
  const cycleTimerRef = useRef<number | null>(null);
  const cyclingRef = useRef(false);
  const busyRef = useRef(false);
  const captureBusyRef = useRef(false);
  const inputRef = useRef(input);
  const capturesRef = useRef(captures);
  const settingsRef = useRef(settings);
  const dispatchRef = useRef<(action: string) => void>(() => undefined);

  inputRef.current = input;
  capturesRef.current = captures;
  settingsRef.current = settings;
  modeRef.current = mode;

  useEffect(() => {
    backend.loadSettings().then(setSettings).catch(() => undefined);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<string>("shortcut-action", ({ payload }) => dispatchRef.current(payload))
      .then((dispose) => { unlisten = dispose; });
    return () => unlisten?.();
  }, []);

  function appendInput(lines: string[]) {
    const text = lines.filter(Boolean).join("\n").trim();
    if (!text) return;
    setInput((current) => current.trim() ? `${current.trim()}\n${text}` : text);
  }

  function addCapture(capture: CaptureResult) {
    setCaptures((current) => [...current, capture]);
    setStatus("ready");
    setNotice(`已加入 image_${capturesRef.current.length + 1}`);
  }

  async function takeFullScreenshot() {
    if (busyRef.current || captureBusyRef.current) return;
    captureBusyRef.current = true;
    setStatus("working");
    setNotice("正在截图…");
    try { addCapture(await captureWithoutOverlay()); }
    catch (error) { showError("截图失败", error); }
    finally { captureBusyRef.current = false; }
  }

  async function markCaptureOrigin() {
    try { setNotice(await backend.markCaptureOrigin()); }
    catch (error) { showError("标记截图区域失败", error); }
  }

  async function takeMarkedScreenshot() {
    if (captureBusyRef.current) return;
    captureBusyRef.current = true;
    setStatus("working");
    try { addCapture(await backend.captureMarkedRegion()); }
    catch (error) { showError("矩形截图失败", error); }
    finally { captureBusyRef.current = false; }
  }

  async function sendRequest() {
    if (busyRef.current) return;
    const text = inputRef.current.trim();
    const images = capturesRef.current;
    if (!text && !images.length) { setNotice("左栏为空"); return; }
    if (!settingsRef.current.apiKey.trim()) {
      setSettingsOpen(true);
      setOutput("请先在设置中填写 API Key。");
      return;
    }
    busyRef.current = true;
    setStatus("working");
    setOutput("正在思考…");
    try {
      const prompt = text || settingsRef.current.codingPrompt;
      setOutput(await backend.ask(prompt, images.map((image) => image.dataUrl)));
      setStatus("ready");
      setNotice("回答完成");
    } catch (error) { showError("请求失败", error); }
    finally { busyRef.current = false; }
  }

  async function startDevices(): Promise<void> {
    const failures: string[] = [];
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true },
      });
      const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
        ? "audio/webm;codecs=opus" : "audio/webm";
      const recorder = new MediaRecorder(stream, { mimeType });
      const session: MicSession = { recorder, stream, chunks: [], mimeType };
      recorder.ondataavailable = (event) => { if (event.data.size) session.chunks.push(event.data); };
      recorder.start(500);
      micRef.current = session;
    } catch (error) { failures.push(describeMicrophoneError(error)); }
    try {
      await backend.startSystemAudio();
      systemActiveRef.current = true;
    } catch (error) { failures.push(`系统音频：${String(error)}`); }
    if (!micRef.current && !systemActiveRef.current) throw new Error(failures.join("；"));
    if (failures.length) setNotice(`部分音源不可用：${failures.join("；")}`);
  }

  async function transcribeMic(session: MicSession): Promise<string> {
    if (session.recorder.state !== "inactive") {
      await new Promise<void>((resolve) => {
        session.recorder.addEventListener("stop", () => resolve(), { once: true });
        session.recorder.stop();
      });
    }
    session.stream.getTracks().forEach((track) => track.stop());
    const blob = new Blob(session.chunks, { type: session.mimeType });
    if (!blob.size) return "";
    const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
    return backend.transcribe(bytes, session.mimeType);
  }

  async function stopAndTranscribe(): Promise<void> {
    if (cyclingRef.current) return;
    cyclingRef.current = true;
    const mic = micRef.current;
    const hadSystem = systemActiveRef.current;
    micRef.current = null;
    systemActiveRef.current = false;
    setStatus("working");
    setNotice("正在转写我/对方语音…");
    const [mine, theirs] = await Promise.allSettled([
      mic ? transcribeMic(mic) : Promise.resolve(""),
      hadSystem ? backend.stopSystemAudio() : Promise.resolve(""),
    ]);
    const lines: string[] = [];
    const errors: string[] = [];
    if (mine.status === "fulfilled" && mine.value.trim()) lines.push(`我：${mine.value.trim()}`);
    if (mine.status === "rejected") errors.push(`我的语音：${String(mine.reason)}`);
    if (theirs.status === "fulfilled" && theirs.value.trim()) lines.push(`对方：${theirs.value.trim()}`);
    if (theirs.status === "rejected") errors.push(`对方语音：${String(theirs.reason)}`);
    appendInput(lines);
    if (errors.length) {
      setOutput(`转写部分失败：\n${errors.join("\n")}`);
      setStatus("error");
      setNotice("部分转写失败");
    } else {
      setStatus("ready");
      setNotice(lines.length ? "转写已追加到左栏" : "没有识别到语音");
    }
    cyclingRef.current = false;
  }

  function scheduleAutoCycle() {
    if (cycleTimerRef.current) globalThis.clearTimeout(cycleTimerRef.current);
    cycleTimerRef.current = globalThis.setTimeout(async () => {
      if (modeRef.current !== "auto") return;
      await stopAndTranscribe();
      if (modeRef.current !== "auto") return;
      try { await startDevices(); scheduleAutoCycle(); }
      catch (error) { setMode("idle"); showError("自动监听无法继续", error); }
    }, AUTO_CHUNK_MS);
  }

  async function startListening(nextMode: Exclude<ListenMode, "idle">) {
    if (modeRef.current !== "idle" || cyclingRef.current) { setNotice("已有录音任务正在运行"); return; }
    try {
      await startDevices();
      setMode(nextMode);
      modeRef.current = nextMode;
      setStatus("ready");
      setNotice(nextMode === "auto" ? "自动监听中（每 20 秒转写）" : "正在收取我/对方语音");
      if (nextMode === "auto") scheduleAutoCycle();
    } catch (error) { showError("无法启动语音识别", error); }
  }

  async function stopListening(expected: Exclude<ListenMode, "idle">) {
    if (modeRef.current !== expected) {
      setNotice(expected === "manual" ? "手动语音尚未开始" : "自动监听尚未开始");
      return;
    }
    setMode("idle");
    modeRef.current = "idle";
    if (cycleTimerRef.current) globalThis.clearTimeout(cycleTimerRef.current);
    cycleTimerRef.current = null;
    await stopAndTranscribe();
  }

  function clearContext() {
    setInput(""); setCaptures([]); setOutput(""); setStatus("ready"); setNotice("已清空");
  }

  function showError(label: string, error: unknown) {
    setOutput(`${label}：${String(error)}`); setStatus("error"); setNotice(label);
  }

  async function saveSettings() {
    try { await backend.saveSettings(settings); setSettingsOpen(false); setNotice("设置已保存"); }
    catch (error) { showError("保存设置失败", error); }
  }

  dispatchRef.current = (action) => {
    switch (action) {
      case "capture-full": void takeFullScreenshot(); break;
      case "capture-origin": void markCaptureOrigin(); break;
      case "capture-region": void takeMarkedScreenshot(); break;
      case "clear": clearContext(); break;
      case "manual-start": void startListening("manual"); break;
      case "manual-stop": void stopListening("manual"); break;
      case "auto-start": void startListening("auto"); break;
      case "auto-stop": void stopListening("auto"); break;
      case "send": void sendRequest(); break;
    }
  };

  return (
    <main className={`app-shell ${IS_MAC ? "platform-mac" : "platform-windows"}`}>
      <nav className="command-bar" data-tauri-drag-region onMouseDown={(event) => {
        if (event.button === 0 && !(event.target as HTMLElement).closest("button")) void getCurrentWindow().startDragging();
      }}>
        <div className={`status-dot ${status} ${mode !== "idle" ? "live" : ""}`} />
        <button title={`截图 ${MOD}S`} onClick={() => void takeFullScreenshot()}><span>▣ 截</span><kbd>S</kbd></button>
        <button title={`清空 ${MOD}X`} onClick={clearContext}><span>⌫ 清</span><kbd>X</kbd></button>
        <span className="divider" />
        <button className={mode === "manual" ? "active" : ""} title={`${MOD}, 开始；${MOD}. 结束`}
          onClick={() => void (mode === "manual" ? stopListening("manual") : startListening("manual"))}><span>◉ 听</span><kbd>, / .</kbd></button>
        <button className={mode === "auto" ? "active" : ""} title={`${MOD}L 开启；${MOD}K 关闭`}
          onClick={() => void (mode === "auto" ? stopListening("auto") : startListening("auto"))}><span>∞ 自动</span><kbd>L / K</kbd></button>
        <span className="bar-spacer" data-tauri-drag-region />
        <span className="notice">{notice}</span>
        <button className="icon-button" title="设置" onClick={() => setSettingsOpen((value) => !value)}>⚙</button>
        <button className="icon-button" title={`隐藏 ${MOD}Space`} onClick={() => void getCurrentWindow().hide()}>—</button>
      </nav>

      {settingsOpen ? (
        <section className="settings-panel">
          <div className="settings-title"><strong>设置</strong><button onClick={() => setSettingsOpen(false)}>×</button></div>
          <label>API Base URL<input value={settings.baseUrl} onChange={(e) => setSettings({ ...settings, baseUrl: e.target.value })} /></label>
          <label>API Key<input type="password" value={settings.apiKey} onChange={(e) => setSettings({ ...settings, apiKey: e.target.value })} /></label>
          <div className="settings-grid">
            <label>文本模型<input value={settings.model} onChange={(e) => setSettings({ ...settings, model: e.target.value })} /></label>
            <label>视觉模型<input value={settings.visionModel} onChange={(e) => setSettings({ ...settings, visionModel: e.target.value })} /></label>
          </div>
          <label>转写模型<input value={settings.transcriptionModel} onChange={(e) => setSettings({ ...settings, transcriptionModel: e.target.value })} /></label>
          <label>系统 Prompt<textarea rows={3} value={settings.systemPrompt} onChange={(e) => setSettings({ ...settings, systemPrompt: e.target.value })} /></label>
          <label>纯截图 Prompt<textarea rows={4} value={settings.codingPrompt} onChange={(e) => setSettings({ ...settings, codingPrompt: e.target.value })} /></label>
          <button className="primary" onClick={() => void saveSettings()}>保存</button>
        </section>
      ) : (
        <section className="workspace">
          <div className="pane input-pane">
            <header><span>CONTEXT</span><kbd>{IS_MAC ? "⌘ ⇧ H" : "Ctrl ⇧ H"} 发送</kbd></header>
            <div className="attachments">
              {captures.map((capture, index) => (
                <button className="attachment-card" key={index} onClick={() => setCaptures((items) => items.filter((__, itemIndex) => itemIndex !== index))} title="点击移除">
                  <img src={capture.dataUrl} alt={`image_${index + 1}`} />
                  <span className="attachment-label">image_{index + 1}</span>
                  <span className="attachment-remove">×</span>
                </button>
              ))}
            </div>
            <textarea value={input} onChange={(event) => setInput(event.target.value)}
              placeholder="输入文字，或用截图/语音把上下文追加到这里…" />
          </div>
          <div className="pane output-pane">
            <header><span>RESPONSE</span><i className={status}>{status === "working" ? "THINKING" : status.toUpperCase()}</i></header>
            <article>{output || "waiting..."}</article>
          </div>
        </section>
      )}
      <footer className="shortcut-hint">
        <span>⇧S 全屏截图</span><span>⇧1 / ⇧2 矩形截图</span><span>⇧, / ⇧. 听</span><span>⇧L / ⇧K 自动</span><span>⇧H 发送</span><span>{IS_MAC ? "⌘Q" : "Ctrl+Q"} 退出</span>
      </footer>
    </main>
  );
}

export default App;
