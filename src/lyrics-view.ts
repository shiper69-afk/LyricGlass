import { formatMs, smoothedPositionMs } from "./clock";
import { findActiveLineIndex, parseLrc, type LrcLine, type ParsedLrc } from "./lrc";

export interface MediaState {
  spotifyRunning: boolean;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  positionReportedMs: number;
  positionReportedAtMs: number;
  playbackStatus: string;
}

export interface LyricsResponse {
  status: string;
  message: string;
  syncedLrc?: string | null;
  plainLyrics?: string | null;
  instrumental: boolean;
  fromCache?: boolean;
}

export class LyricsView {
  private syncOffsetMs = 0;
  private parsed: ParsedLrc | null = null;
  private statusMessage = "Play a song in Spotify";

  private trackKey = "";
  private lastLineIndex = -2;
  private lastMediaSignature = "";

  private readonly headerTitle: HTMLElement;
  private readonly headerArtist: HTMLElement;
  private readonly statusEl: HTMLElement;
  private readonly linesEl: HTMLElement;
  private readonly plainEl: HTMLElement;
  private readonly offsetEl: HTMLElement;

  constructor(root: HTMLElement) {
    this.headerTitle = root.querySelector("#np-title")!;
    this.headerArtist = root.querySelector("#np-artist")!;
    this.statusEl = root.querySelector("#lyrics-status")!;
    this.linesEl = root.querySelector("#lyrics-lines")!;
    this.plainEl = root.querySelector("#lyrics-plain")!;
    this.offsetEl = root.querySelector("#sync-offset-label")!;
  }

  setSyncOffset(deltaMs: number) {
    this.syncOffsetMs += deltaMs;
    this.offsetEl.textContent =
      this.syncOffsetMs === 0
        ? ""
        : `Sync offset ${this.syncOffsetMs > 0 ? "+" : ""}${this.syncOffsetMs} ms`;
    this.lastLineIndex = -2;
  }

  onLyricsLoading() {
    this.statusMessage = "Loading lyrics…";
    this.statusEl.textContent = this.statusMessage;
    this.linesEl.innerHTML = "";
    this.plainEl.classList.add("hidden");
    this.linesEl.classList.remove("hidden");
    this.parsed = null;
    this.lastLineIndex = -2;
  }

  onLyricsResponse(res: LyricsResponse) {
    this.statusMessage = res.message;

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

  tick(media: MediaState | null) {
    if (!media) return;

    const signature = `${media.positionReportedMs}|${media.playbackStatus}|${media.positionReportedAtMs}`;
    if (signature !== this.lastMediaSignature) {
      this.lastMediaSignature = signature;
      if (media.playbackStatus.toLowerCase() !== "playing") {
        // still render once on pause/seek
      }
    }

    if (!this.parsed || this.parsed.lines.length === 0) return;

    const playing = media.playbackStatus.toLowerCase() === "playing";
    const now = Date.now();
    const smooth = smoothedPositionMs(
      media.positionReportedMs,
      playing,
      now - media.positionReportedAtMs,
    );
    const timeMs = smooth + this.syncOffsetMs;

    const idx = findActiveLineIndex(this.parsed.lines, timeMs);
    if (idx === this.lastLineIndex) return;
    this.lastLineIndex = idx;
    this.renderLines(this.parsed.lines, idx, timeMs);
  }

  private renderLines(lines: LrcLine[], active: number, timeMs: number) {
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
        if (i === active) {
          p.dataset.time = formatMs(timeMs);
        }
        frag.appendChild(p);
      }
    }
    this.linesEl.replaceChildren(frag);
  }

  private escape(s: string) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  }
}