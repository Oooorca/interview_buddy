import type { AppPlatform } from "./types";

export const windowsPlatform: AppPlatform = {
  kind: "windows",
  rootClass: "platform-windows",
  shortcutModifier: "Ctrl+Shift+",
  quitShortcut: "Ctrl+Q",
  isMicrophonePermissionError: () => false,
};
