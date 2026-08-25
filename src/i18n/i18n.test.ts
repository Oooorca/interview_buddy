import { describe, expect, it } from "vitest";
import enUs from "./locales/en-US.json";
import zhCn from "./locales/zh-CN.json";
import { DEFAULT_PROMPTS, defaultSettings, effectiveAnswerLanguage } from "../shared/settings";

function leafKeys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) return [prefix];
  return Object.entries(value).flatMap(([key, child]) =>
    leafKeys(child, prefix ? `${prefix}.${key}` : key));
}

describe("application locales", () => {
  it("keeps the zh-CN and en-US translation key sets identical", () => {
    expect(leafKeys(enUs).sort()).toEqual(leafKeys(zhCn).sort());
  });

  it("uses en-US rather than a partial en locale", () => {
    expect(effectiveAnswerLanguage("en-US", "zh-CN")).toBe("en-US");
    expect(defaultSettings.uiLanguage).toBe("system");
    expect(defaultSettings.answerLanguage).toBe("follow-ui");
    expect(Object.keys(DEFAULT_PROMPTS)).toEqual(["zh-CN", "en-US"]);
  });

  it("provides distinct complete default Prompts for both answer languages", () => {
    expect(DEFAULT_PROMPTS["zh-CN"].system).not.toBe(DEFAULT_PROMPTS["en-US"].system);
    expect(DEFAULT_PROMPTS["zh-CN"].coding.length).toBeGreaterThan(40);
    expect(DEFAULT_PROMPTS["en-US"].coding.length).toBeGreaterThan(40);
  });
});
