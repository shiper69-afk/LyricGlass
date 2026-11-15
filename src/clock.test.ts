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
    // Position still 30s at anchor, but anchor was 3s ago while playing.
    expect(smoothedPositionMs(30_000, true, 3_000)).toBe(33_000);
  });

  it("respects playback rate", () => {
    expect(smoothedPositionMs(10_000, true, 2_000, 1.25)).toBe(12_500);
  });

  it("does not advance when rate is zero", () => {
    expect(smoothedPositionMs(10_000, true, 2_000, 0)).toBe(10_000);
  });
});

describe("mediaClockMs", () => {
  const base = {
    positionReportedMs: 60_000,
    positionReportedAtMs: 1_000_000,
    playbackStatus: "playing",
    playbackRate: 1,
  };

  it("seeks forward when anchor jumps", () => {
    const jumped = mediaClockMs(
      { ...base, positionReportedMs: 120_000, positionReportedAtMs: 1_000_050 },
      1_000_100,
    );
    expect(jumped).toBe(120_050);
  });

  it("freezes on pause", () => {
    const paused = mediaClockMs(
      { ...base, playbackStatus: "paused" },
      1_005_000,
    );
    expect(paused).toBe(60_000);
  });

  it("resumes from new anchor on play", () => {
    const resumed = mediaClockMs(
      {
        ...base,
        positionReportedMs: 60_000,
        positionReportedAtMs: 2_000_000,
        playbackStatus: "playing",
      },
      2_001_500,
    );
    expect(resumed).toBe(61_500);
  });

  it("track change uses fresh anchor", () => {
    const nextTrack = mediaClockMs(
      {
        positionReportedMs: 0,
        positionReportedAtMs: 5_000_000,
        playbackStatus: "playing",
        playbackRate: 1,
      },
      5_000_250,
    );
    expect(nextTrack).toBe(250);
  });
});

describe("timelineAnchorAgeMs", () => {
  it("returns elapsed ms since anchor", () => {
    expect(timelineAnchorAgeMs(1_000, 4_500)).toBe(3_500);
  });
});