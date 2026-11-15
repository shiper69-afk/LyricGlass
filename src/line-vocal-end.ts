import type { LrcLine } from "./lrc";

/** End of sung content on a line (ignores LRCLIB “held” last tokens through instrumental). */
export function lineVocalEndMs(line: LrcLine, nextLineStartMs: number): number {
  if (line.words.length === 0) {
    return Math.min(nextLineStartMs, line.timeMs + 400);
  }
  const last = line.words[line.words.length - 1]!;
  const gap = nextLineStartMs - last.startMs;
  const dur = last.endMs - last.startMs;
  if (dur > 900 && gap > dur * 0.85) {
    const prev = line.words[line.words.length - 2];
    if (prev) return prev.endMs;
    return last.startMs + 280;
  }
  return Math.min(last.endMs, nextLineStartMs);
}