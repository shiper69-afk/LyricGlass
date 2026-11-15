import { describe, expect, it } from "vitest";
import { parseLrc } from "./lrc";
import { scrollStartTimeMs } from "./lyrics-scroll";
import { lineVocalEndMs } from "./line-vocal-end";

describe("lineVocalEndMs", () => {
  it("does not treat a held last token as vocal through instrumental", () => {
    const { lines } = parseLrc(
      `[00:00.00] <00:00.00>One <00:00.40>two <00:00.80>three <00:01.10>end\n[00:03.00] Next`,
    );
    const vocal = lineVocalEndMs(lines[0], lines[1].timeMs);
    expect(vocal).toBeLessThan(1200);
    const scrollAt = scrollStartTimeMs(
      lines[0].timeMs,
      lines[1].timeMs,
      vocal,
    );
    expect(scrollAt).toBeLessThan(lines[1].timeMs - 200);
  });
});