import { describe, expect, it } from "vitest";
import {
  buildLineWordTimings,
  cappedLineEndMs,
  findActiveWordIndex,
  MAX_LINE_SPREAD_MS,
  syllableWeight,
} from "./word-timing";

describe("cappedLineEndMs", () => {
  it("caps long gaps before next line", () => {
    expect(cappedLineEndMs(10_000, 90_000)).toBe(10_000 + MAX_LINE_SPREAD_MS);
  });

  it("uses full gap when short", () => {
    expect(cappedLineEndMs(1000, 3500)).toBe(3500);
  });
});

describe("syllableWeight", () => {
  it("weights longer English words higher", () => {
    expect(syllableWeight("I")).toBeLessThan(syllableWeight("beautiful"));
  });

  it("handles Hebrew", () => {
    expect(syllableWeight("שלום")).toBeGreaterThan(0);
  });
});

describe("buildLineWordTimings", () => {
  it("distributes by syllable weight", () => {
    const words = buildLineWordTimings(
      "Hello beautiful world",
      "Hello beautiful world",
      0,
      3000,
    );
    expect(words).toHaveLength(3);
    expect(words[0].startMs).toBe(0);
    expect(words[2].endMs).toBe(3000);
    const mid = words[1].endMs - words[1].startMs;
    const last = words[2].endMs - words[2].startMs;
    expect(mid).toBeGreaterThan(last);
  });

  it("uses enhanced tags when present", () => {
    const raw = "<00:10.12>Hello <00:10.80>world";
    const words = buildLineWordTimings(raw, "Hello world", 10_000, 20_000);
    expect(words[0].startMs).toBe(10_120);
    expect(words[1].startMs).toBe(10_800);
  });

  it("does not stretch last word across instrumental gap", () => {
    const words = buildLineWordTimings(
      "Line end",
      "Line end",
      0,
      60_000,
    );
    expect(words[words.length - 1].endMs).toBeLessThanOrEqual(
      MAX_LINE_SPREAD_MS,
    );
  });
});

describe("findActiveWordIndex", () => {
  const words = buildLineWordTimings("a b c", "a b c", 0, 3000);

  it("returns word index for time", () => {
    expect(findActiveWordIndex(words, 500)).toBe(0);
    expect(findActiveWordIndex(words, 2500)).toBe(2);
  });
});