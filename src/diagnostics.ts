import {
  formatMs,
  formatMsPrecise,
  timelineAnchorAgeMs,
} from "./clock";
import { playbackClockMs, type PlaybackAnchor } from "./playback-clock";
import type { LyricsView, LyricsResponse, MediaState } from "./lyrics-view";

const APP_VERSION = "0.3.2";

export interface FullMediaState extends MediaState {
  sourceAppId: string;
  lastEventKind: string;
  timelineUpdates: number;
  message: string;
  positionExtrapolationWallMs?: number;
  lastUpdatedRawTicks?: number;
  serverTimeMs?: number;
  computedPositionMs?: number;
}

export function buildDiagnosticsBlock(
  media: FullMediaState | null,
  lyrics: LyricsResponse | null,
  lyricsView: LyricsView,
  playbackAnchor: PlaybackAnchor | null = null,
): string {
  const now = Date.now();
  const computed = media ? playbackClockMs(media, playbackAnchor, now) : 0;
  const line = lyricsView.getLineDiagnostics(computed);
  const syncMs = lyricsView.getSyncOffsetMs();

  const gsmtcDur = media ? (media.durationMs / 1000).toFixed(1) : "—";
  const lrclibDur =
    lyrics?.lrclibDurationSec != null
      ? lyrics.lrclibDurationSec.toFixed(1)
      : "—";

  return [
    `LyricGlass ${APP_VERSION}`,
    `Time (local): ${new Date(now).toISOString()}`,
    "",
    "Track (GSMTC)",
    `  title: ${media?.title ?? "—"}`,
    `  artist: ${media?.artist ?? "—"}`,
    `  album: ${media?.album ?? "—"}`,
    `  duration: ${gsmtcDur}s (${media?.durationMs ?? 0} ms)`,
    `  playback: ${media?.playbackStatus ?? "—"} rate=${media?.playbackRate ?? 1}`,
    `  lastEvent: ${media?.lastEventKind ?? "—"}`,
    "",
    "LRCLIB match",
    `  id: ${lyrics?.lrclibId ?? "—"}`,
    `  track: ${lyrics?.lrclibTrackName ?? "—"}`,
    `  artist: ${lyrics?.lrclibArtistName ?? "—"}`,
    `  album: ${lyrics?.lrclibAlbumName ?? "—"}`,
    `  duration: ${lrclibDur}s`,
    `  delta vs GSMTC: ${lyrics?.durationDeltaSec != null ? lyrics.durationDeltaSec.toFixed(2) : "—"}s warning=${lyrics?.durationWarning ?? false}`,
    `  lyrics status: ${lyrics?.status ?? "—"}`,
    "",
    "Clock",
    `  gsmtcPosition: ${formatMs(media?.positionReportedMs ?? 0)} (${media?.positionReportedMs ?? 0} ms)`,
    `  lastUpdatedUnixMs: ${media?.positionReportedAtMs ?? 0}`,
    `  lastUpdatedRawTicks: ${media?.lastUpdatedRawTicks ?? 0}`,
    `  extrapolationWallMs: ${media?.positionExtrapolationWallMs ?? 0}`,
    `  timelineAgeMs: ${media ? timelineAnchorAgeMs(media.positionReportedAtMs, now) : 0}`,
    `  serverTimeMs: ${media?.serverTimeMs ?? 0}`,
    `  backendComputedAtEmit: ${media?.computedPositionMs ?? 0} ms`,
    `  computedClock: ${formatMs(computed)} (${computed} ms)`,
    `  userSyncOffsetMs: ${syncMs}`,
    "",
    "Active line",
    `  index: ${line.index}`,
    `  lineStart: ${formatMs(line.lineTimeMs)} (${line.lineTimeMs} ms)`,
    `  nextLineStart: ${line.nextLineTimeMs >= 0 ? `${formatMs(line.nextLineTimeMs)} (${line.nextLineTimeMs} ms)` : "—"}`,
    `  computedVsLineDelta: ${formatMsPrecise(line.computedMs - line.lineTimeMs)}`,
  ].join("\n");
}

export function buildSyncMarkBlock(
  media: FullMediaState | null,
  lyricsView: LyricsView,
  playbackAnchor: PlaybackAnchor | null = null,
): string {
  const now = Date.now();
  const computed = media ? playbackClockMs(media, playbackAnchor, now) : 0;
  const line = lyricsView.getLineDiagnostics(computed);
  return [
    buildDiagnosticsBlock(
      media,
      lyricsView.getLastLyrics(),
      lyricsView,
      playbackAnchor,
    ),
    "",
    `MARK computedClock=${computed}ms activeLineStart=${line.lineTimeMs}ms delta=${computed - line.lineTimeMs}ms`,
  ].join("\n");
}