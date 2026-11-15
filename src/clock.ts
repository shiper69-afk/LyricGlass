/**
 * Playback clock: GSMTC Position extrapolated from LastUpdatedTime and
 * from a monotonic wall anchor (Store Spotify refreshes LastUpdatedTime while
 * Position stays stale).
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
  /** Unix ms — GSMTC `LastUpdatedTime`. */
  positionReportedAtMs: number;
  /** Unix ms — wall time when Position was last observed (monotonic extrapolation). */
  positionExtrapolationWallMs?: number;
  playbackStatus: string;
  playbackRate?: number;
}

export function mediaClockMs(
  media: MediaClockInput,
  nowMs: number = Date.now(),
): number {
  const playing = media.playbackStatus.toLowerCase() === "playing";
  if (!playing) return media.positionReportedMs;

  const rate = media.playbackRate ?? 1;
  const wallAnchor =
    media.positionExtrapolationWallMs ?? media.positionReportedAtMs;

  const elapsedGsmtc = safeElapsed(nowMs, media.positionReportedAtMs);
  const elapsedWall = safeElapsed(nowMs, wallAnchor);

  const fromGsmtc = smoothedPositionMs(
    media.positionReportedMs,
    true,
    elapsedGsmtc,
    rate,
  );
  const fromWall = smoothedPositionMs(
    media.positionReportedMs,
    true,
    elapsedWall,
    rate,
  );

  return Math.max(fromGsmtc, fromWall);
}

function safeElapsed(nowMs: number, anchorMs: number): number {
  if (anchorMs <= 0) return 0;
  return Math.max(0, nowMs - anchorMs);
}

/** Age of the GSMTC timeline anchor (how stale the reported position may be). */
export function timelineAnchorAgeMs(
  positionReportedAtMs: number,
  nowMs: number = Date.now(),
): number {
  return safeElapsed(nowMs, positionReportedAtMs);
}

export function formatMs(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function formatMsPrecise(ms: number): string {
  const sign = ms < 0 ? "-" : "";
  const abs = Math.abs(ms);
  const s = (abs / 1000).toFixed(2);
  return `${sign}${s}s`;
}