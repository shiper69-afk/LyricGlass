import { describe, expect, it } from "vitest";
import { parseLrc } from "./lrc";
import { interLineGapRest, isSongIntroIdle } from "./lyrics-gap";
import { scrollStartTimeMs } from "./lyrics-scroll";
import { lineVocalEndMs } from "./line-vocal-end";

const SHORT_GAP_LRC = `[00:00.00] <00:00.00>One <00:00.40>two <00:00.80>three <00:01.10>end
[00:03.00] <00:03.00>Four <00:03.40>five`;

const LONG_GAP_LRC = `[00:02.00] <00:02.00>Alpha <00:02.50>beta <00:03.00>gamma
[00:10.00] <00:10.00>Delta <00:10.50>echo`;

describe("lyrics-gap", () => {
  it("intro idle only before first line", () => {
    const { lines } = parseLrc(LONG_GAP_LRC);
    expect(isSongIntroIdle(lines, 500)).toBe(true);
    expect(isSongIntroIdle(lines, 2000)).toBe(false);
  });

  it("rests on last word through short and long inter-line gaps", () => {
    const short = parseLrc(SHORT_GAP_LRC).lines;
    const vocal = lineVocalEndMs(short[0], short[1].timeMs);
    const startAt = scrollStartTimeMs(
      short[0].timeMs,
      short[1].timeMs,
      vocal,
    );
    const mid = Math.floor((vocal + 120 + startAt) / 2);
    expect(interLineGapRest(short, 0, mid)).toBe(true);
    expect(interLineGapRest(short, 0, startAt)).toBe(false);

    const long = parseLrc(LONG_GAP_LRC).lines;
    const longVocal = lineVocalEndMs(long[0], long[1].timeMs);
    const longStart = scrollStartTimeMs(
      long[0].timeMs,
      long[1].timeMs,
      longVocal,
    );
    expect(interLineGapRest(long, 0, 5500)).toBe(true);
    expect(interLineGapRest(long, 0, longStart - 10)).toBe(true);
    expect(interLineGapRest(long, 0, longStart)).toBe(false);
  });
});