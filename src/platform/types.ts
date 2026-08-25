export type PlatformKind = "windows" | "macos" | "browser";

export type AppPlatform = {
  kind: PlatformKind;
  rootClass: string;
  shortcutModifier: string;
  quitShortcut: string;
  isMicrophonePermissionError: (detail: string) => boolean;
};
