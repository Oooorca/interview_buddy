import { describe, expect, it } from "vitest";
import { looksLikeInterviewPrompt, normalizedQuestion } from "./autoAnswer";

describe("automatic answer question detection", () => {
  it("recognizes multilingual interview questions without treating short noise as a question", () => {
    expect(looksLikeInterviewPrompt("请介绍一下你负责的项目")).toBe(true);
    expect(looksLikeInterviewPrompt("How would you design this service?")).toBe(true);
    expect(looksLikeInterviewPrompt("嗯")).toBe(false);
  });

  it("normalizes punctuation and casing for duplicate detection", () => {
    expect(normalizedQuestion("How Does It Work?"))
      .toBe(normalizedQuestion("how does it work？"));
  });
});
