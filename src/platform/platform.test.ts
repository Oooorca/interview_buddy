import { describe, expect, it } from "vitest";
import { detectPlatform } from "./index";

describe("platform detection", () => {
  it("provides native shortcut conventions for Windows and macOS", () => {
    expect(detectPlatform("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")).toMatchObject({
      kind: "windows",
      shortcutModifier: "Ctrl+Shift+",
      quitShortcut: "Ctrl+Q",
    });
    expect(detectPlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")).toMatchObject({
      kind: "macos",
      shortcutModifier: "⌘⇧",
      quitShortcut: "⌘Q",
    });
  });

  it("uses the browser fallback for unknown hosts", () => {
    expect(detectPlatform("test-browser").kind).toBe("browser");
  });
});
