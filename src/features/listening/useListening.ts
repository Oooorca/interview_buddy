import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { errorMessage } from "../../services/backend";

export function useListeningPlatform(isMac: boolean) {
  const { t } = useTranslation();
  const describeMicrophoneError = useCallback((error: unknown): string => {
    const detail = errorMessage(error);
    if (isMac && /NotAllowedError|PermissionDenied|permission denied|not allowed/i.test(detail)) {
      return t("errors.macMicrophone");
    }
    return t("errors.microphone", { error: detail });
  }, [isMac, t]);
  return { describeMicrophoneError };
}
