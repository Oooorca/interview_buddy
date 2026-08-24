import { useRef, type MutableRefObject } from "react";
import { useTranslation } from "react-i18next";
import { backend, errorMessage } from "../../services/backend";
import type { AppSettings } from "../../shared/types";
import { useListeningPlatform } from "./useListening";

type MicMonitor = {
  audioContext: AudioContext;
  analyser: AnalyserNode;
  samples: Float32Array<ArrayBuffer>;
};

export type MicSession = {
  recorder: MediaRecorder;
  stream: MediaStream;
  chunks: Blob[];
  mimeType: string;
  monitor: MicMonitor;
};

type UseAudioCaptureOptions = {
  isMac: boolean;
  settingsRef: MutableRefObject<AppSettings>;
  setVoiceIssue: (issue: string) => void;
};

export function useAudioCapture({ isMac, settingsRef, setVoiceIssue }: UseAudioCaptureOptions) {
  const { t } = useTranslation();
  const { describeMicrophoneError } = useListeningPlatform(isMac);
  const micRef = useRef<MicSession | null>(null);
  const systemActiveRef = useRef(false);

  function createMonitor(stream: MediaStream): MicMonitor {
    const audioContext = new AudioContext();
    const analyser = audioContext.createAnalyser();
    analyser.fftSize = 1024;
    analyser.smoothingTimeConstant = 0.15;
    audioContext.createMediaStreamSource(stream).connect(analyser);
    void audioContext.resume();
    return { audioContext, analyser, samples: new Float32Array(analyser.fftSize) };
  }

  function createSession(stream: MediaStream, mimeType: string, monitor?: MicMonitor): MicSession {
    const recorder = new MediaRecorder(stream, { mimeType });
    const session: MicSession = {
      recorder,
      stream,
      chunks: [],
      mimeType,
      monitor: monitor || createMonitor(stream),
    };
    recorder.ondataavailable = (event) => { if (event.data.size) session.chunks.push(event.data); };
    recorder.start(250);
    return session;
  }

  function microphoneLevel(): number {
    const monitor = micRef.current?.monitor;
    if (!monitor) return 0;
    monitor.analyser.getFloatTimeDomainData(monitor.samples);
    let squares = 0;
    for (const sample of monitor.samples) squares += sample * sample;
    return Math.sqrt(squares / monitor.samples.length);
  }

  async function startDevices(): Promise<void> {
    const failures: string[] = [];
    const current = settingsRef.current;
    if (!current.captureMicrophone && !current.captureSystemAudio) throw new Error(t("notices.enableAudio"));
    if (current.captureMicrophone) {
      let stream: MediaStream | null = null;
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          audio: {
            echoCancellation: true,
            noiseSuppression: true,
            autoGainControl: true,
            ...(current.microphoneDeviceId ? { deviceId: { exact: current.microphoneDeviceId } } : {}),
          },
        });
        const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
          ? "audio/webm;codecs=opus" : "audio/webm";
        micRef.current = createSession(stream, mimeType);
      } catch (error) {
        stream?.getTracks().forEach((track) => track.stop());
        failures.push(describeMicrophoneError(error));
      }
    }
    if (current.captureSystemAudio) {
      try {
        await backend.startSystemAudio();
        systemActiveRef.current = true;
      } catch (error) { failures.push(t("errors.systemAudio", { error: errorMessage(error) })); }
    }
    if (!micRef.current && !systemActiveRef.current) throw new Error(failures.join("；"));
    setVoiceIssue(failures.join("；"));
  }

  async function transcribeMic(session: MicSession, stopTracks: boolean): Promise<string> {
    try {
      if (session.recorder.state !== "inactive") {
        await new Promise<void>((resolve) => {
          session.recorder.addEventListener("stop", () => resolve(), { once: true });
          session.recorder.stop();
        });
      }
    } finally {
      if (stopTracks) {
        session.stream.getTracks().forEach((track) => track.stop());
        void session.monitor.audioContext.close();
      }
    }
    const blob = new Blob(session.chunks, { type: session.mimeType });
    if (blob.size < 256) return "";
    return backend.transcribe(Array.from(new Uint8Array(await blob.arrayBuffer())), session.mimeType);
  }

  function rotateAndTranscribe(): Promise<string> {
    const current = micRef.current;
    if (!current) return Promise.resolve("");
    micRef.current = createSession(current.stream, current.mimeType, current.monitor);
    return transcribeMic(current, false);
  }

  async function rotateAndDiscard(): Promise<void> {
    const current = micRef.current;
    if (!current) return;
    micRef.current = createSession(current.stream, current.mimeType, current.monitor);
    if (current.recorder.state !== "inactive") {
      await new Promise<void>((resolve) => {
        current.recorder.addEventListener("stop", () => resolve(), { once: true });
        current.recorder.stop();
      });
    }
  }

  function detach() {
    const mic = micRef.current;
    const hadSystem = systemActiveRef.current;
    micRef.current = null;
    systemActiveRef.current = false;
    return { mic, hadSystem };
  }

  return {
    micRef, systemActiveRef, microphoneLevel, startDevices, transcribeMic,
    rotateAndTranscribe, rotateAndDiscard, detach,
  };
}
