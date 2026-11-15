import {
  findActiveWordIndex,
  type TimedWord,
  wordProgress,
} from "./word-timing";

export interface KaraokeSyncState {
  timeMs: number;
  activeIndex: number;
  /** 0..1 progress within the active word (same clock for ball hop + highlight). */
  progress: number;
  /**
   * 0 = resting on active word; (0,1) = airborne toward next word in active word's tail;
   * reaches 1 exactly at `words[activeIndex+1].startMs`.
   */
  interWordT: number;
}

function smoothstep(t: number): number {
  const x = Math.max(0, Math.min(1, t));
  return x * x * (3 - 2 * x);
}

/** Single per-frame karaoke clock + word index (ball + highlight must use this only). */
export function computeKaraokeSync(
  words: TimedWord[],
  timeMs: number,
): KaraokeSyncState | null {
  const activeIndex = findActiveWordIndex(words, timeMs);
  if (activeIndex < 0) return null;

  const progress = wordProgress(words, timeMs);
  let interWordT = 0;

  const w = words[activeIndex];
  const nextIdx = activeIndex + 1;
  if (nextIdx < words.length) {
    const next = words[nextIdx];
    const dur = Math.max(1, w.endMs - w.startMs);
    const maxFlight = Math.min(dur * 0.35, Math.max(1, next.startMs - w.startMs));
    const flightStart = Math.max(w.startMs, next.startMs - maxFlight);
    if (timeMs >= flightStart && timeMs < next.startMs) {
      const fd = Math.max(1, next.startMs - flightStart);
      interWordT = smoothstep((timeMs - flightStart) / fd);
    }
  }

  return { timeMs, activeIndex, progress, interWordT };
}

/**
 * Index used for highlight classes and for "which word is being sung".
 * Always equals `activeIndex` from {@link computeKaraokeSync} (no look-ahead).
 */
export function highlightWordIndex(state: KaraokeSyncState): number {
  return state.activeIndex;
}

/**
 * Ball anchor indices: start on active word; during tail flight blend toward next
 * but landing completes when `activeIndex` advances to next word (same instant as highlight).
 */
export function ballAnchorIndices(
  state: KaraokeSyncState,
  wordCount: number,
): { from: number; to: number; blend: number } {
  const from = state.activeIndex;
  if (state.interWordT <= 0 || from + 1 >= wordCount) {
    return { from, to: from, blend: 0 };
  }
  return { from, to: from + 1, blend: state.interWordT };
}