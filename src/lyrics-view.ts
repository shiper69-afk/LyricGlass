import { mediaClockMs } from "./clock";
import { findActiveLineIndex, parseLrc, type LrcLine, type ParsedLrc } from "./lrc";

export interface MediaState {
  spotifyRunning: boolean;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  positionReportedMs: number;
  positionReportedAtMs: number;
  positionExtrapolationWallMs?: number;
  playbackRate?: number;
  playbackStatus: string;
}

export interface LyricsResponse {
  status: string;
  message: string;
  syncedLrc?: string | null;
  plainLyrics?: string | null;
  instrumental: boolean;
  fromCache?: boolean;
  lrclibId?: number | null;
  lrclibDurationSec?: number | null;
  lrclibTrackName?: string | null;
  lrclibArtistName?: string | null;
  lrclibAlbumName?: string | null;
  durationDeltaSec?: number | null;
  durationWarning?: boolean;
}

export interface LineDiagnostics {
  index: number;
  lineTimeMs: number;
  nextLineTimeMs: number;
  computedMs: number;
}

export class LyricsView {
  private syncOffsetMs = 0;
  private parsed: ParsedLrc | null = null;
  private lastLyrics: LyricsResponse | null = null;
  private statusMessage = "Play a song in Spotify";

  private trackKey = "";
  private lastLineIndex = -2;

  private readonly headerTitle: HTMLElement;
  private readonly headerArtist: HTMLElement;
  private readonly statusEl: HTMLElement;
  private readonly linesEl: HTMLElement;
  private readonly plainEl: HTMLElement;
  private readonly offsetEl: HTMLElement;
  private readonly durationWarnEl: HTMLElement | null;

  constructor(root: HTMLElement) {
    this.headerTitle = root.querySelector("#np-title")!;
    this.headerArtist = root.querySelector("#np-artist")!;
    this.statusEl = root.querySelector("#lyrics-status")!;
    this.linesEl = root.querySelector("#lyrics-lines")!;
    this.plainEl = root.querySelector("#lyrics-plain")!;
    this.offsetEl = root.querySelector("#sync-offset-label")!;
    this.durationWarnEl = document.getElementById("duration-warn");
  }

  getSyncOffsetMs() {
    return this.syncOffsetMs;
  }

  getLastLyrics() {
    return this.lastLyrics;
  }

  setSyncOffsetAbsolute(offsetMs: number, options?: { persistLabel?: boolean }) {
    this.syncOffsetMs = Math.max(-10_000, Math.min(10_000, offsetMs));
    this.offsetEl.textContent =
      this.syncOffsetMs === 0
        ? ""
        : `Timing ${this.syncOffsetMs > 0 ? "+" : ""}${(this.syncOffsetMs / 1000).toFixed(1)}s`;
    this.lastLineIndex = -2;
    void options;
  }

  onLyricsLoading() {
    this.statusMessage = "Loading lyrics…";
    this.statusEl.textContent = this.statusMessage;
    this.linesEl.innerHTML = "";
    this.plainEl.classList.add("hidden");
    this.linesEl.classList.remove("hidden");
    this.parsed = null;
    this.lastLyrics = null;
    this.hideDurationWarning();
    this.lastLineIndex = -2;
  }

  onLyricsResponse(res: LyricsResponse) {
    this.lastLyrics = res;
    this.statusMessage = res.message;

    if (res.durationWarning && res.lrclibDurationSec != null) {
      this.showDurationWarning(res);
    } else {
      this.hideDurationWarning();
    }

    if (res.syncedLrc) {
      this.statusEl.textContent = res.fromCache ? `${res.message} (cached)` : "";
      this.parsed = parseLrc(res.syncedLrc);
      this.plainEl.classList.add("hidden");
      this.linesEl.classList.remove("hidden");
    } else if (res.plainLyrics) {
      this.statusEl.textContent = res.message;
      this.parsed = null;
      this.plainEl.textContent = res.plainLyrics;
      this.plainEl.classList.remove("hidden");
      this.linesEl.classList.add("hidden");
    } else {
      this.statusEl.textContent = res.message;
      this.parsed = null;
      this.plainEl.classList.add("hidden");
      this.linesEl.classList.remove("hidden");
      this.linesEl.innerHTML = `<p class="lyrics-empty">${this.escape(res.message)}</p>`;
    }
    this.lastLineIndex = -2;
  }

  private showDurationWarning(res: LyricsResponse) {
    if (!this.durationWarnEl) return;
    const gsmtc = res.durationDeltaSec != null && this.lastLyrics;
    const lrclib = res.lrclibDurationSec?.toFixed(1) ?? "?";
    const delta = res.durationDeltaSec?.toFixed(1) ?? "?";
    this.durationWarnEl.textContent = `Duration mismatch: LRCLIB ${lrclib}s vs Spotify (Δ ${delta}s) — timing may be off`;
    this.durationWarnEl.classList.remove("hidden");
    void gsmtc;
  }

  private hideDurationWarning() {
    this.durationWarnEl?.classList.add("hidden");
  }

  shouldFetchLyrics(media: MediaState): boolean {
    if (!media.title.trim() && !media.artist.trim()) return false;
    const key = `${media.artist}|${media.title}|${media.album}|${media.durationMs}`;
    if (key === this.trackKey) return false;
    this.trackKey = key;
    return true;
  }

  onMediaHeader(media: MediaState) {
    this.headerTitle.textContent = media.title || "—";
    this.headerArtist.textContent = media.artist || "—";
  }

  onSeek() {
    this.lastLineIndex = -2;
  }

  getLineDiagnostics(computedMs: number): LineDiagnostics {
    if (!this.parsed?.lines.length) {
      return { index: -1, lineTimeMs: 0, nextLineTimeMs: -1, computedMs };
    }
    const timeMs = computedMs + this.syncOffsetMs;
    const idx = findActiveLineIndex(this.parsed.lines, timeMs);
    const lineTimeMs = idx >= 0 ? this.parsed.lines[idx].timeMs : 0;
    const nextLineTimeMs =
      idx >= 0 && idx + 1 < this.parsed.lines.length
        ? this.parsed.lines[idx + 1].timeMs
        : -1;
    return { index: idx, lineTimeMs, nextLineTimeMs, computedMs: timeMs };
  }

  tick(media: MediaState | null) {
    if (!media || !this.parsed || this.parsed.lines.length === 0) return;

    const smooth = mediaClockMs(media, Date.now());
    const timeMs = smooth + this.syncOffsetMs;

    const idx = findActiveLineIndex(this.parsed.lines, timeMs);
    if (idx === this.lastLineIndex) return;
    this.lastLineIndex = idx;
    this.renderLines(this.parsed.lines, idx);
  }

  private renderLines(lines: LrcLine[], active: number) {
    const start = Math.max(0, active - 2);
    const end = Math.min(lines.length - 1, Math.max(active + 3, active + 1));
    const frag = document.createDocumentFragment();

    if (active < 0) {
      const p = document.createElement("p");
      p.className = "lyric-line upcoming";
      p.textContent = lines[0]?.text ?? "";
      frag.appendChild(p);
    } else {
      for (let i = start; i <= end; i += 1) {
        const p = document.createElement("p");
        p.className = "lyric-line";
        p.dir = "auto";
        if (i < active) p.classList.add("past");
        else if (i === active) p.classList.add("active");
        else p.classList.add("upcoming");
        p.textContent = lines[i].text;
        frag.appendChild(p);
      }
    }
    this.linesEl.replaceChildren(frag);
  }

  private escape(s: string) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  }
}