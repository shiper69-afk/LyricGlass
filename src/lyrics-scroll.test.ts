import { describe, expect, it } from "vitest";
import {
  scrollDurationMs,
  scrollEase,
  scrollStartTimeMs,
  SCROLL_MS_MAX,
  SCROLL_MS_MIN,
} from "./lyrics-scroll";

describe("lyrics-scroll", () => {
  it("eases within 0..1", () => {
    expect(scrollEase(0)).toBe(0);
    expect(scrollEase(1)).toBe(1);
    expect(scrollEase(0.5)).toBeGreaterThan(0.4);
  });

  it("clamps scroll duration", () => {
    expect(scrollDurationMs(50)).toBe(SCROLL_MS_MIN);
    expect(scrollDurationMs(10_000)).toBe(SCROLL_MS_MAX);
    expect(scrollDurationMs(400)).toBe(400);
  });

  it("delays long-gap hop near next line", () => {
    const start = scrollStartTimeMs(0, 8000, 2500);
    expect(start).toBeGreaterThan(7000);
  });
});