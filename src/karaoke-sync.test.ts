import { describe, expect, it } from "vitest";
import {
  ballAnchorIndices,
  computeKaraokeSync,
  highlightWordIndex,
} from "./karaoke-sync";
import { buildLineWordTimings } from "./word-timing";

const THUNDER_LINE = "Thunder, thunder, thun-, thunder";

describe("karaoke sync", () => {
  const words = buildLineWordTimings(
    THUNDER_LINE,
    THUNDER_LINE,
    0,
    8_000,
  );

  it("tokenizes Thunder line into four distinct words by index", () => {
    expect(words).toHaveLength(4);
    expect(words[0].text.trim()).toMatch(/^Thunder/);
    expect(words[2].text.trim()).toBe("thun-,");
  });

  it("ball logical index matches highlight index across the line", () => {
    for (let t = 0; t <= 8_000; t += 25) {
      const state = computeKaraokeSync(words, t);
      if (!state) continue;
      const hi = highlightWordIndex(state);
      const { from, to, blend } = ballAnchorIndices(state, words.length);
      expect(hi).toBe(state.activeIndex);
      if (blend <= 0) {
        expect(from).toBe(hi);
        expect(to).toBe(hi);
      } else {
        expect(from).toBe(hi);
        expect(to).toBe(hi + 1);
        expect(blend).toBeLessThan(1);
      }
    }
  });

  it("does not target next word before its start time", () => {
    for (let i = 0; i < words.length - 1; i += 1) {
      const mid = Math.floor((words[i].startMs + words[i + 1].startMs) / 2);
      if (mid >= words[i + 1].startMs) continue;
      const state = computeKaraokeSync(words, mid);
      expect(state).not.toBeNull();
      expect(highlightWordIndex(state!)).toBeLessThan(i + 1);
    }
  });

  it("lands on word N when time reaches word N start", () => {
    for (let i = 1; i < words.length; i += 1) {
      const state = computeKaraokeSync(words, words[i].startMs);
      expect(state?.activeIndex).toBe(i);
      expect(highlightWordIndex(state!)).toBe(i);
      const anchors = ballAnchorIndices(state!, words.length);
      expect(anchors.blend).toBe(0);
      expect(anchors.from).toBe(i);
    }
  });
});