import { describe, expect, it } from "vitest";
import { smoothedPositionMs } from "./clock";

describe("smoothedPositionMs", () => {
  it("holds position when paused", () => {
    expect(smoothedPositionMs(90_000, false, 5_000)).toBe(90_000);
  });

  it("advances while playing", () => {
    expect(smoothedPositionMs(90_000, true, 1_500)).toBe(91_500);
  });
});