import { describe, expect, it } from "vitest";
import { plainScrollOffsetPx, textLooksHebrew } from "./plain-scroll";

describe("plain-scroll", () => {
  it("detects Hebrew script", () => {
    expect(textLooksHebrew("Hello")).toBe(false);
    expect(textLooksHebrew("שלום")).toBe(true);
  });

  it("eases scroll offset with playback progress", () => {
    const h = 400;
    const v = 120;
    const start = plainScrollOffsetPx(0, 1000, h, v);
    const mid = plainScrollOffsetPx(500, 1000, h, v);
    const end = plainScrollOffsetPx(1000, 1000, h, v);
    expect(start).toBe(0);
    expect(mid).toBeGreaterThan(50);
    expect(end).toBeCloseTo(h - v, 0);
  });
});