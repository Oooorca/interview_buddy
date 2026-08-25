import { browserPlatform } from "./browser";
import { macosPlatform } from "./macos";
import type { AppPlatform } from "./types";
import { windowsPlatform } from "./windows";

export function detectPlatform(userAgent: string): AppPlatform {
  if (/Macintosh|Mac OS X/i.test(userAgent)) return macosPlatform;
  if (/Windows/i.test(userAgent)) return windowsPlatform;
  return browserPlatform;
}

export const appPlatform = detectPlatform(navigator.userAgent);
export const isTauriRuntime = "__TAURI_INTERNALS__" in window;

export type { AppPlatform, PlatformKind } from "./types";
