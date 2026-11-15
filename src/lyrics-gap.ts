import type { LrcLine } from "./lrc";
import { lineVocalEndMs } from "./line-vocal-end";
import { scrollStartTimeMs } from "./lyrics-scroll";

/** After the last sung word, hold the ball on that word until cross-line scroll starts. */
export function interLineGapRest(
  lines: LrcLine[],
  idx: number,
  timeMs: number,
): boolean {
  const line = lines[idx];
  const next = lines[idx + 1];
  if (!line || !next) return false;
  const lastEnd = lineVocalEndMs(line, next.timeMs);
  const startAt = scrollStartTimeMs(line.timeMs, next.timeMs, lastEnd);
  if (timeMs < lastEnd + 40) return false;
  if (timeMs >= startAt) return false;
  return true;
}

/** Song intro before the first line timestamp (centered idle bounce only). */
export function isSongIntroIdle(
  lines: LrcLine[],
  timeMs: number,
): boolean {
  return lines.length > 0 && timeMs < lines[0].timeMs;
}