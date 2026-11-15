import { describe, expect, it } from "vitest";
import {
  mediaClockMs,
  smoothedPositionMs,
  timelineAnchorAgeMs,
} from "./clock";

describe("smoothedPositionMs", () => {
  it("holds position when paused", () => {
    expect(smoothedPositionMs(90_000, false, 5_000)).toBe(90_000);
  });

  it("advances while playing at rate 1", () => {
    expect(smoothedPositionMs(90_000, true, 1_500)).toBe(91_500);
  });

  it("extrapolates through a stale GSMTC update", () => {
    expect(smoothedPositionMs(30_000, true, 3_000)).toBe(33_000);
  });

  it("respects playback rate", () => {
    expect(smoothedPositionMs(10_000, true, 2_000, 1.25)).toBe(12_500);
  });
});

describe("mediaClockMs dual anchor", () => {
  const base = {
    positionReportedMs: 30_000,
    positionReportedAtMs: 1_000_000,
    positionExtrapolationWallMs: 996_000,
    playbackStatus: "playing",
    playbackRate: 1,
  };

  it("uses max of GSMTC and wall extrapolation (Spotify quirk)", () => {
    const now = 1_000_000;
    expect(mediaClockMs(base, now)).toBe(34_000);
  });

  it("clamps future GSMTC anchor", () => {
    const now = 5_000;
    const clock = mediaClockMs(
      {
        ...base,
        positionReportedAtMs: 9_000,
        positionExtrapolationWallMs: 9_000,
        positionReportedMs: 1_000,
      },
      now,
    );
    expect(clock).toBe(1_000);
  });

  it("seeks forward when anchor jumps", () => {
    const jumped = mediaClockMs(
      {
        positionReportedMs: 120_000,
        positionReportedAtMs: 1_000_050,
        positionExtrapolationWallMs: 1_000_050,
        playbackStatus: "playing",
        playbackRate: 1,
      },
      1_000_100,
    );
    expect(jumped).toBe(120_050);
  });

  it("freezes on pause", () => {
    const paused = mediaClockMs(
      { ...base, playbackStatus: "paused" },
      1_005_000,
    );
    expect(paused).toBe(30_000);
  });
});

describe("timelineAnchorAgeMs", () => {
  it("returns elapsed ms since anchor", () => {
    expect(timelineAnchorAgeMs(1_000, 4_500)).toBe(3_500);
  });

  it("clamps future anchor to zero age contribution", () => {
    expect(timelineAnchorAgeMs(5_000, 4_000)).toBe(0);
  });
});