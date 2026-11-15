/**
 * Playback clock anchored to GSMTC `Position` at `LastUpdatedTime` (not poll receive time).
 */
export function smoothedPositionMs(
  positionMs: number,
  playing: boolean,
  elapsedSinceAnchorMs: number,
  playbackRate = 1,
): number {
  if (!playing || playbackRate <= 0) return positionMs;
  const extra = Math.max(0, elapsedSinceAnchorMs) * playbackRate;
  return Math.min(positionMs + Math.floor(extra), Number.MAX_SAFE_INTEGER);
}

export interface MediaClockInput {
  positionReportedMs: number;
  /** Unix ms — GSMTC timeline `LastUpdatedTime`, or receive time when unknown. */
  positionReportedAtMs: number;
  playbackStatus: string;
  playbackRate?: number;
}

export function mediaClockMs(
  media: MediaClockInput,
  nowMs: number = Date.now(),
): number {
  const playing = media.playbackStatus.toLowerCase() === "playing";
  const rate = media.playbackRate ?? 1;
  const elapsed = Math.max(0, nowMs - media.positionReportedAtMs);
  return smoothedPositionMs(
    media.positionReportedMs,
    playing,
    elapsed,
    rate,
  );
}

/** Age of the GSMTC timeline anchor (how stale the reported position may be). */
export function timelineAnchorAgeMs(
  positionReportedAtMs: number,
  nowMs: number = Date.now(),
): number {
  if (positionReportedAtMs <= 0) return 0;
  return Math.max(0, nowMs - positionReportedAtMs);
}

export function formatMs(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}