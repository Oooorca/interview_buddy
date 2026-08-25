import type { AppPlatform } from "./types";

const PERMISSION_ERROR = /NotAllowedError|PermissionDenied|permission denied|not allowed/i;

export const macosPlatform: AppPlatform = {
  kind: "macos",
  rootClass: "platform-mac",
  shortcutModifier: "⌘⇧",
  quitShortcut: "⌘Q",
  isMicrophonePermissionError: (detail) => PERMISSION_ERROR.test(detail),
};
