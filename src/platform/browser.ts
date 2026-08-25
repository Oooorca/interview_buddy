import type { AppPlatform } from "./types";

export const browserPlatform: AppPlatform = {
  kind: "browser",
  rootClass: "platform-browser",
  shortcutModifier: "Ctrl+Shift+",
  quitShortcut: "Ctrl+Q",
  isMicrophonePermissionError: () => false,
};
