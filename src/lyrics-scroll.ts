/** Smooth vertical scroll between lyric lines (GPU `translateY` on inner strip). */

export const SCROLL_MS_MIN = 300;
export const SCROLL_MS_MAX = 500;
/** Gaps longer than this use a delayed hop near the next line. */
export const SCROLL_LONG_GAP_MS = 2_500;
/** How early to start a hop before the next line on long gaps. */
export const SCROLL_LONG_LEAD_MS = 420;

export function scrollEase(t: number): number {
  const x = Math.max(0, Math.min(1, t));
  return x * x * (3 - 2 * x);
}

export interface LineScrollAnimation {
  fromIndex: number;
  toIndex: number;
  /** Playback time (ms) when the scroll started. */
  startTimeMs: number;
  durationMs: number;
  /** Pixels to move the inner strip upward (positive number). */
  deltaY: number;
}

export function scrollDurationMs(gapMs: number): number {
  return Math.max(SCROLL_MS_MIN, Math.min(SCROLL_MS_MAX, gapMs));
}

/** When to start scrolling toward the next line (playback ms). */
export function scrollStartTimeMs(
  lineStartMs: number,
  nextLineStartMs: number,
  lastWordEndMs: number,
): number {
  const gap = Math.max(0, nextLineStartMs - lineStartMs);
  const vocalEnd = Math.max(lineStartMs, lastWordEndMs);
  if (gap >= SCROLL_LONG_GAP_MS) {
    return Math.max(
      vocalEnd,
      nextLineStartMs - Math.min(SCROLL_MS_MAX, SCROLL_LONG_LEAD_MS),
    );
  }
  const lead = scrollDurationMs(nextLineStartMs - vocalEnd);
  return Math.max(vocalEnd, nextLineStartMs - lead);
}

export function scrollProgress(
  anim: LineScrollAnimation,
  timeMs: number,
): number {
  const dt = anim.durationMs <= 0 ? 1 : (timeMs - anim.startTimeMs) / anim.durationMs;
  return scrollEase(dt);
}