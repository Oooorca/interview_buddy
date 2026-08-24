import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { backend, errorMessage } from "../../services/backend";
import type { AudioOutputDevice } from "../../shared/types";

export function useAudioDevices(active: boolean) {
  const { t } = useTranslation();
  const [microphoneDevices, setMicrophoneDevices] = useState<MediaDeviceInfo[]>([]);
  const [outputDevices, setOutputDevices] = useState<AudioOutputDevice[]>([]);
  const [loading, setLoading] = useState(false);
  const [issue, setIssue] = useState("");
  const loadingRef = useRef(false);

  async function refresh(requestMicrophonePermission: boolean) {
    if (loadingRef.current) return;
    loadingRef.current = true;
    setLoading(true);
    setIssue("");
    const errors: string[] = [];
    if (requestMicrophonePermission) {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        stream.getTracks().forEach((track) => track.stop());
      } catch (error) {
        errors.push(t("errors.microphonePermission", { error: errorMessage(error) }));
      }
    }
    const [browserDevices, nativeOutputs] = await Promise.allSettled([
      navigator.mediaDevices.enumerateDevices(),
      backend.listSystemAudioDevices(),
    ]);
    if (browserDevices.status === "fulfilled") {
      setMicrophoneDevices(browserDevices.value.filter((device) => device.kind === "audioinput"));
    } else {
      errors.push(t("errors.inputDevices", { error: errorMessage(browserDevices.reason) }));
    }
    if (nativeOutputs.status === "fulfilled") setOutputDevices(nativeOutputs.value);
    else errors.push(t("errors.outputDevices", { error: errorMessage(nativeOutputs.reason) }));
    setIssue(errors.join("; "));
    loadingRef.current = false;
    setLoading(false);
  }

  useEffect(() => {
    if (active) void refresh(false);
  }, [active]);

  useEffect(() => {
    const mediaDevices = navigator.mediaDevices;
    if (!mediaDevices?.addEventListener) return;
    const handleDeviceChange = () => { if (active) void refresh(false); };
    mediaDevices.addEventListener("devicechange", handleDeviceChange);
    return () => mediaDevices.removeEventListener("devicechange", handleDeviceChange);
  }, [active]);

  return { microphoneDevices, outputDevices, loading, issue, refresh };
}
