import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { appPlatform } from "../../platform";
import { errorMessage } from "../../services/backend";

export function useListeningPlatform() {
  const { t } = useTranslation();
  const describeMicrophoneError = useCallback((error: unknown): string => {
    const detail = errorMessage(error);
    if (appPlatform.isMicrophonePermissionError(detail)) {
      return t("errors.macMicrophone");
    }
    return t("errors.microphone", { error: detail });
  }, [t]);
  return { describeMicrophoneError };
}
