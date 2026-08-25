import { describe, expect, it, vi } from "vitest";
import { advanceVad, freshVadState } from "./vad";

describe("voice activity detector", () => {
  it("flushes sustained speech after a natural pause", () => {
    vi.spyOn(performance, "now").mockReturnValue(0);
    const state = freshVadState();
    let now = 100;
    for (let index = 0; index < 5; index += 1) {
      expect(advanceVad(state, 0.03, now)).toBeNull();
      now += 100;
    }
    const actions = [];
    for (let index = 0; index < 9; index += 1) {
      actions.push(advanceVad(state, 0, now));
      now += 100;
    }
    expect(actions).toContain("flush");
  });
});
