import { useCallback } from "react";

export function useListeningPlatform(isMac: boolean) {
  const describeMicrophoneError = useCallback((error: unknown): string => {
    const detail = String(error);
    if (isMac && /NotAllowedError|PermissionDenied|permission denied|not allowed/i.test(detail)) {
      return "麦克风：macOS 拒绝了当前构建。请在“隐私与安全性 → 麦克风”中重新开关 Interview Buddy，并彻底退出后重启应用；本地临时签名在重新构建后可能需要重新授权。";
    }
    return `麦克风：${detail}`;
  }, [isMac]);
  return { describeMicrophoneError };
}
