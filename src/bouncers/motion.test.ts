import { describe, expect, it } from "vitest";
import { DEFAULT_BOUNCER_ID, isBouncerId, normalizeBouncerId } from "./ids";
import {
  computeBouncerMotion,
  eqBarScales,
  squashStretchPhase,
} from "./motion";

describe("bouncer ids", () => {
  it("normalizes unknown to glow-orb", () => {
    expect(normalizeBouncerId("nope")).toBe(DEFAULT_BOUNCER_ID);
    expect(isBouncerId("comet")).toBe(true);
  });
});

describe("bouncer motion", () => {
  it("squash stretch peaks in mid air", () => {
    const mid = squashStretchPhase(0.42);
    expect(mid.ty).toBeLessThan(0);
    const land = squashStretchPhase(1);
    expect(land.ty).toBe(0);
  });

  it("music-note tilts more at apex", () => {
    const apex = computeBouncerMotion({
      id: "music-note",
      hopProgress: 0.42,
      rtl: false,
      heldNote: false,
      wordIndex: 0,
      interWordBlend: 0,
    });
    expect(apex.localTransform).toContain("rotate");
    const land = computeBouncerMotion({
      id: "music-note",
      hopProgress: 0.95,
      rtl: false,
      heldNote: false,
      wordIndex: 0,
      interWordBlend: 0,
    });
    expect(land.localTransform).toContain("rotate");
  });

  it("comet mirrors in rtl on host", () => {
    const motion = computeBouncerMotion({
      id: "comet",
      hopProgress: 0.5,
      rtl: true,
      heldNote: false,
      wordIndex: 0,
      interWordBlend: 0,
    });
    expect(motion.hostTransform).toBe("scaleX(-1)");
  });

  it("paper-plane mirrors in rtl on host", () => {
    const motion = computeBouncerMotion({
      id: "paper-plane",
      hopProgress: 0.4,
      rtl: true,
      heldNote: false,
      wordIndex: 1,
      interWordBlend: 0.2,
    });
    expect(motion.hostTransform).toBe("scaleX(-1)");
  });

  it("eq bars vary heights", () => {
    const a = eqBarScales(0.2, false);
    const b = eqBarScales(0.7, false);
    expect(a[1]).not.toBeCloseTo(b[1], 1);
  });
});