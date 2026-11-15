import { describe, expect, it } from "vitest";
import {
  computeKaraokeSync,
  deriveBallState,
  highlightedWordIndex,
} from "./karaoke-sync";
import { buildLineWordTimings } from "./word-timing";

const THUNDER_LINE = "Thunder, thunder, thun-, thunder";
const CREEP_LINE = "But I'm a creep";

function assertBallMatchesHighlight(words: TimedWord[], lineLabel: string) {
  for (let t = 0; t <= words[words.length - 1].endMs + 200; t += 20) {
    const sync = computeKaraokeSync(words, t);
    if (!sync) continue;
    const ball = deriveBallState(sync, words.length);
    const hi = highlightedWordIndex(ball);
    expect(hi, `${lineLabel} @${t}ms`).toBe(ball.ballWordIndex);
    if (ball.positionBlend > 0 && ball.positionBlend < 1) {
      expect(hi, `${lineLabel} flight @${t}ms`).toBe(ball.positionFrom);
      expect(hi).toBeLessThan(ball.positionTo);
    } else {
      expect(ball.positionTo).toBe(ball.positionFrom);
    }
  }
  for (let i = 0; i < words.length; i += 1) {
    const atStart = computeKaraokeSync(words, words[i].startMs);
    expect(atStart).not.toBeNull();
    const ball = deriveBallState(atStart!, words.length);
    expect(highlightedWordIndex(ball)).toBe(i);
    expect(ball.ballWordIndex).toBe(i);
    expect(ball.positionBlend).toBe(0);
  }
}

type TimedWord = import("./word-timing").TimedWord;

describe("karaoke sync", () => {
  const thunder = buildLineWordTimings(THUNDER_LINE, THUNDER_LINE, 0, 8_000);
  const creep = buildLineWordTimings(CREEP_LINE, CREEP_LINE, 0, 6_000);

  it("tokenizes Creep line into four words", () => {
    expect(creep).toHaveLength(4);
    expect(creep[3].text.trim().toLowerCase()).toContain("creep");
  });

  it("Thunder: ballWordIndex === highlightedIndex at landing and after", () => {
    assertBallMatchesHighlight(thunder, "thunder");
  });

  it("Creep: ballWordIndex === highlightedIndex at landing and after", () => {
    assertBallMatchesHighlight(creep, "creep");
  });

  it("next word stays uncolored until ballWordIndex advances", () => {
    const words = creep;
    const nextWord = 3;
    const prevStart = words[nextWord - 1].startMs;
    const landStart = words[nextWord].startMs;
    const midFlight = Math.floor((prevStart + landStart) / 2);
    if (midFlight < landStart) {
      const sync = computeKaraokeSync(words, midFlight);
      if (sync && sync.interWordT > 0) {
        const ball = deriveBallState(sync, words.length);
        expect(ball.ballWordIndex).toBeLessThan(nextWord);
      }
    }
    const atLand = deriveBallState(
      computeKaraokeSync(words, landStart)!,
      words.length,
    );
    expect(atLand.ballWordIndex).toBe(nextWord);
  });
});