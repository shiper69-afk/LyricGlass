import {
  findActiveWordIndex,
  type TimedWord,
  wordProgress,
} from "./word-timing";

export interface KaraokeSyncState {
  timeMs: number;
  activeIndex: number;
  /** 0..1 hop phase within the timed active word (ball motion only). */
  progress: number;
  /**
   * 0 = resting on active word; (0,1) = airborne toward next word in active word's tail;
   * completes as `timeMs` reaches `words[activeIndex+1].startMs`.
   */
  interWordT: number;
}

/** Ball is the source of truth for which words are fully highlighted. */
export interface KaraokeBallState {
  sync: KaraokeSyncState;
  /** Inclusive index: words `0..ballWordIndex` are fully colored. */
  ballWordIndex: number;
  positionFrom: number;
  positionTo: number;
  positionBlend: number;
}

function smoothstep(t: number): number {
  const x = Math.max(0, Math.min(1, t));
  return x * x * (3 - 2 * x);
}

/** Single per-frame karaoke clock (timing); combine with {@link deriveBallState}. */
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
 * Derive ball position + highlight index from sync.
 * While flying toward word N (`interWordT` in (0,1)), ballWordIndex stays on N-1;
 * next word stays uncolored until hop completes (`interWordT` === 0 on word N).
 */
export function deriveBallState(
  sync: KaraokeSyncState,
  wordCount: number,
): KaraokeBallState {
  const inFlight = sync.interWordT > 0 && sync.interWordT < 1;
  const ballWordIndex = inFlight ? sync.activeIndex : sync.activeIndex;

  let positionFrom = sync.activeIndex;
  let positionTo = sync.activeIndex;
  let positionBlend = 0;

  if (inFlight && positionFrom + 1 < wordCount) {
    positionTo = positionFrom + 1;
    positionBlend = sync.interWordT;
  }

  return {
    sync,
    ballWordIndex,
    positionFrom,
    positionTo,
    positionBlend,
  };
}

/** Highlighted word index === ball landed word (inclusive full color). */
export function highlightedWordIndex(state: KaraokeBallState): number {
  return state.ballWordIndex;
}

/** @deprecated use highlightedWordIndex(deriveBallState(...)) */
export function highlightWordIndex(sync: KaraokeSyncState): number {
  return deriveBallState(sync, Number.MAX_SAFE_INTEGER).ballWordIndex;
}

export function ballAnchorIndices(
  state: KaraokeBallState,
): { from: number; to: number; blend: number } {
  return {
    from: state.positionFrom,
    to: state.positionTo,
    blend: state.positionBlend,
  };
}