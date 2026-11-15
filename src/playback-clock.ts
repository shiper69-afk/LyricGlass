import { mediaClockMs, type MediaClockInput } from "./clock";

export interface PlaybackClockMedia extends MediaClockInput {
  computedPositionMs?: number;
}

export interface PlaybackAnchor {
  computedMs: number;
  perfMs: number;
  rate: number;
}

/** Playback ms for lyrics sync: backend snapshot + `performance.now()` gap. */
export function playbackClockMs(
  media: PlaybackClockMedia | null,
  anchor: PlaybackAnchor | null,
  nowMs: number = Date.now(),
): number {
  if (!media) return 0;
  const playing = media.playbackStatus.toLowerCase() === "playing";
  if (!playing) return media.positionReportedMs;

  if (
    anchor &&
    media.computedPositionMs != null &&
    media.computedPositionMs > 0
  ) {
    const elapsed = Math.max(0, performance.now() - anchor.perfMs);
    return Math.min(
      anchor.computedMs + Math.floor(elapsed * anchor.rate),
      Number.MAX_SAFE_INTEGER,
    );
  }

  return mediaClockMs(media, nowMs);
}

export function createPlaybackAnchor(media: PlaybackClockMedia, nowMs: number): PlaybackAnchor {
  const computed =
    media.computedPositionMs != null && media.computedPositionMs > 0
      ? media.computedPositionMs
      : mediaClockMs(media, nowMs);
  return {
    computedMs: computed,
    perfMs: performance.now(),
    rate: media.playbackRate ?? 1,
  };
}