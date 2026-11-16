import { describe, expect, it } from "vitest";
import { combinedEnergy, JsBeatDetector } from "./beat-fallback";

describe("JsBeatDetector", () => {
  it("fires when energy jumps above rolling average", () => {
    const d = new JsBeatDetector(16, 1.2, 0.05, 4);
    for (let i = 0; i < 8; i++) expect(d.tick(0.1)).toBe(false);
    expect(d.tick(0.5)).toBe(true);
    expect(d.tick(0.5)).toBe(false);
  });

  it("combinedEnergy blends level and bass", () => {
    const e = combinedEnergy(0.8, [0.2, 0.4]);
    expect(e).toBeGreaterThan(0.25);
    expect(e).toBeLessThan(0.8);
  });
});