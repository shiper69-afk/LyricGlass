import type { PlaybackClockMedia } from "./playback-clock";
import { KaraokeBall, type CrossLineHandoff } from "./karaoke-ball";
import {
  findActiveLineIndex,
  parseLrc,
  type LrcLine,
  type ParsedLrc,
} from "./lrc";
import { findActiveWordIndex } from "./word-timing";
import {
  type LineScrollAnimation,
  scrollDurationMs,
  scrollProgress,
  scrollStartTimeMs,
} from "./lyrics-scroll";

export interface MediaState extends PlaybackClockMedia {
  spotifyRunning: boolean;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
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

interface RenderOptions {
  /** Extra line indices that get per-word spans (for cross-line scroll). */
  wordLineIndices?: Set<number>;
}

export class LyricsView {
  private syncOffsetMs = 0;
  private parsed: ParsedLrc | null = null;
  private lastLyrics: LyricsResponse | null = null;
  private statusMessage = "Play a song in Spotify";

  private trackKey = "";
  private lastLineIndex = -2;
  private karaokeEnabled = true;
  private readonly karaoke: KaraokeBall;
  private resizeObserver: ResizeObserver | null = null;

  private readonly headerTitle: HTMLElement;
  private readonly headerArtist: HTMLElement;
  private readonly statusEl: HTMLElement;
  private readonly linesEl: HTMLElement;
  private readonly scrollInner: HTMLDivElement;
  private readonly plainEl: HTMLElement;
  private readonly offsetEl: HTMLElement;
  private readonly durationWarnEl: HTMLElement | null;

  private snapScroll = true;
  private scrollAnim: LineScrollAnimation | null = null;
  private renderedCenterIndex = -1;

  constructor(root: HTMLElement) {
    this.headerTitle = root.querySelector("#np-title")!;
    this.headerArtist = root.querySelector("#np-artist")!;
    this.statusEl = root.querySelector("#lyrics-status")!;
    this.linesEl = root.querySelector("#lyrics-lines")!;
    this.plainEl = root.querySelector("#lyrics-plain")!;
    this.offsetEl = root.querySelector("#sync-offset-label")!;
    this.durationWarnEl = document.getElementById("duration-warn");
    this.scrollInner = document.createElement("div");
    this.scrollInner.className = "lyrics-scroll-inner";
    this.linesEl.appendChild(this.scrollInner);
    this.karaoke = new KaraokeBall(this.linesEl);
    if (typeof ResizeObserver !== "undefined") {
      this.resizeObserver = new ResizeObserver(() => this.karaoke.invalidate());
      this.resizeObserver.observe(this.linesEl);
    }
  }

  setKaraokeColors(ballColor: string, sungHighlight: string) {
    this.karaoke.setThemeColors(ballColor, sungHighlight);
  }

  setKaraokeBouncer(bouncerId: string) {
    this.karaoke.setBouncerType(bouncerId);
  }

  setKaraokeEnabled(enabled: boolean) {
    this.karaokeEnabled = enabled;
    this.karaoke.setEnabled(enabled);
    this.resetScroll();
    this.lastLineIndex = -2;
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
    this.snapScroll = true;
    this.lastLineIndex = -2;
    void options;
  }

  private resetScroll() {
    this.scrollAnim = null;
    this.scrollInner.style.transform = "";
  }

  private snapToLine(idx: number) {
    this.resetScroll();
    if (this.parsed) this.renderLines(this.parsed.lines, idx);
    this.renderedCenterIndex = idx;
    this.lastLineIndex = idx;
    this.snapScroll = false;
  }

  onLyricsLoading() {
    this.statusMessage = "Loading lyrics…";
    this.statusEl.textContent = this.statusMessage;
    this.scrollInner.replaceChildren();
    this.plainEl.classList.add("hidden");
    this.linesEl.classList.remove("hidden");
    this.parsed = null;
    this.lastLyrics = null;
    this.hideDurationWarning();
    this.resetScroll();
    this.lastLineIndex = -2;
    this.karaoke.invalidate();
    this.karaoke.hide();
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
      this.karaoke.hide();
    } else {
      this.statusEl.textContent = res.message;
      this.parsed = null;
      this.plainEl.classList.add("hidden");
      this.linesEl.classList.remove("hidden");
      this.scrollInner.innerHTML = `<p class="lyrics-empty">${this.escape(res.message)}</p>`;
      this.karaoke.hide();
    }
    this.snapScroll = true;
    this.lastLineIndex = -2;
    this.karaoke.invalidate();
  }

  private showDurationWarning(res: LyricsResponse) {
    if (!this.durationWarnEl) return;
    const lrclib = res.lrclibDurationSec?.toFixed(1) ?? "?";
    const delta = res.durationDeltaSec?.toFixed(1) ?? "?";
    this.durationWarnEl.textContent = `Duration mismatch: LRCLIB ${lrclib}s vs Spotify (Δ ${delta}s) — timing may be off`;
    this.durationWarnEl.classList.remove("hidden");
  }

  private hideDurationWarning() {
    this.durationWarnEl?.classList.add("hidden");
  }

  shouldFetchLyrics(media: MediaState): boolean {
    if (!media.title.trim() && !media.artist.trim()) return false;
    const key = `${media.artist}|${media.title}|${media.album}|${media.durationMs}`;
    if (key === this.trackKey) return false;
    this.trackKey = key;
    this.snapScroll = true;
    return true;
  }

  onMediaHeader(media: MediaState) {
    this.headerTitle.textContent = media.title || "—";
    this.headerArtist.textContent = media.artist || "—";
  }

  onSeek() {
    this.snapScroll = true;
    this.lastLineIndex = -2;
    this.karaoke.invalidate();
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

  private lineByIndex(i: number): HTMLElement | null {
    return this.scrollInner.querySelector<HTMLElement>(
      `.lyric-line[data-line="${i}"]`,
    );
  }

  private measureScrollDelta(fromIndex: number, toIndex: number): number {
    const fromEl = this.lineByIndex(fromIndex);
    const toEl = this.lineByIndex(toIndex);
    if (!fromEl || !toEl) return 0;
    return Math.max(0, toEl.offsetTop - fromEl.offsetTop);
  }

  private beginScroll(fromIndex: number, toIndex: number) {
    if (!this.parsed) return;
    const lines = this.parsed.lines;
    const fromLine = lines[fromIndex];
    const toLine = lines[toIndex];
    if (!fromLine || !toLine) return;

    const lastEnd =
      fromLine.words[fromLine.words.length - 1]?.endMs ??
      cappedLineVisualEnd(fromLine, toLine.timeMs);
    const startAt = scrollStartTimeMs(
      fromLine.timeMs,
      toLine.timeMs,
      lastEnd,
    );
    const duration = scrollDurationMs(Math.max(40, toLine.timeMs - startAt));

    const wordLines = new Set<number>([fromIndex, toIndex]);
    this.renderLines(lines, fromIndex, { wordLineIndices: wordLines });
    const deltaY = this.measureScrollDelta(fromIndex, toIndex);
    if (deltaY < 2) {
      this.snapToLine(toIndex);
      return;
    }

    this.scrollAnim = {
      fromIndex,
      toIndex,
      startTimeMs: startAt,
      durationMs: duration,
      deltaY,
    };
    this.renderedCenterIndex = fromIndex;
  }

  private finishScrollIfDone(timeMs: number, activeIdx: number) {
    if (!this.scrollAnim) return;
    const p = scrollProgress(this.scrollAnim, timeMs);
    const ty = -this.scrollAnim.deltaY * p;
    this.scrollInner.style.transform = `translate3d(0, ${ty}px, 0)`;

    if (p < 1) return;

    const target = this.scrollAnim.toIndex;
    this.resetScroll();
    this.renderLines(this.parsed!.lines, target);
    this.renderedCenterIndex = target;
    this.lastLineIndex = activeIdx >= 0 ? activeIdx : target;
  }

  tick(playbackMs: number) {
    if (!this.parsed || this.parsed.lines.length === 0) return;

    const timeMs = playbackMs + this.syncOffsetMs;
    const idx = findActiveLineIndex(this.parsed.lines, timeMs);
    const lines = this.parsed.lines;

    if (this.snapScroll) {
      if (idx >= 0) this.snapToLine(idx);
      else if (idx !== this.lastLineIndex) {
        this.renderLines(lines, idx);
        this.lastLineIndex = idx;
        this.renderedCenterIndex = idx;
      }
    } else if (idx !== this.lastLineIndex) {
      const jumped = Math.abs(idx - this.lastLineIndex) > 1 || idx < this.lastLineIndex;
      if (jumped || !this.scrollAnim) {
        this.snapToLine(idx);
      } else if (this.scrollAnim) {
        this.lastLineIndex = idx;
      }
    }

    if (!this.snapScroll) {
      const nextIdx = idx >= 0 ? idx + 1 : -1;
      if (
        !this.scrollAnim &&
        idx >= 0 &&
        nextIdx < lines.length &&
        shouldArmScroll(lines, idx, nextIdx, timeMs)
      ) {
        this.beginScroll(idx, nextIdx);
      }

      if (this.scrollAnim) {
        this.finishScrollIfDone(timeMs, idx);
      } else if (this.renderedCenterIndex !== idx && idx >= 0) {
        const wordLines = new Set<number>();
        if (this.karaokeEnabled) wordLines.add(idx);
        this.renderLines(lines, idx, { wordLineIndices: wordLines });
        this.renderedCenterIndex = idx;
      }
    }

    const scrollY = this.scrollAnim
      ? -this.scrollAnim.deltaY * scrollProgress(this.scrollAnim, timeMs)
      : 0;

    const karaokeOn = this.karaokeEnabled && this.parsed !== null;
    const activeEl = this.lineByIndex(idx);
    const words = idx >= 0 && karaokeOn ? lines[idx].words : [];

    let crossLine: CrossLineHandoff | null = null;
    let restOnLast: { line: HTMLElement; words: LrcLine["words"] } | null =
      null;

    const scrollHandoff =
      karaokeOn &&
      this.scrollAnim &&
      !this.snapScroll &&
      timeMs >= this.scrollAnim.startTimeMs + 40;

    if (scrollHandoff) {
      const fromEl = this.lineByIndex(this.scrollAnim!.fromIndex);
      const toEl = this.lineByIndex(this.scrollAnim!.toIndex);
      if (fromEl && toEl) {
        const stillOnFrom =
          idx === this.scrollAnim!.fromIndex &&
          findActiveWordIndex(lines[idx].words, timeMs) >= 0 &&
          timeMs < cappedLineVisualEnd(lines[idx], lines[this.scrollAnim!.toIndex].timeMs);
        if (!stillOnFrom) {
          crossLine = {
            fromLine: fromEl,
            toLine: toEl,
            scrollTranslateY: scrollY,
            t: scrollProgress(this.scrollAnim!, timeMs),
          };
        }
      }
    } else if (
      karaokeOn &&
      idx >= 0 &&
      idx + 1 < lines.length &&
      !this.scrollAnim
    ) {
      const gapRest = instrumentalRest(lines, idx, timeMs);
      if (gapRest) {
        const lineEl = this.lineByIndex(idx);
        if (lineEl) {
          restOnLast = { line: lineEl, words: lines[idx].words };
        }
      }
    }

    this.karaoke.update(words, timeMs, karaokeOn, activeEl, {
      scrollTranslateY: scrollY,
      crossLine,
      restOnLastWord: restOnLast,
    });
  }

  private renderLines(
    lines: LrcLine[],
    active: number,
    options?: RenderOptions,
  ) {
    const start = Math.max(0, active - 2);
    const end = Math.min(lines.length - 1, Math.max(active + 3, active + 1));
    const frag = document.createDocumentFragment();
    const karaokeOn = this.karaokeEnabled;
    const wordLines = options?.wordLineIndices ?? new Set<number>();

    if (active >= 0) wordLines.add(active);

    if (active < 0) {
      const p = document.createElement("p");
      p.className = "lyric-line upcoming";
      p.dataset.line = "0";
      p.textContent = lines[0]?.text ?? "";
      frag.appendChild(p);
    } else {
      for (let i = start; i <= end; i += 1) {
        const p = document.createElement("p");
        p.className = "lyric-line";
        p.dir = "auto";
        p.dataset.line = String(i);
        if (i < active) p.classList.add("past");
        else if (i === active) p.classList.add("active");
        else p.classList.add("upcoming");

        const useWords =
          karaokeOn && lines[i].words.length > 0 && wordLines.has(i);
        if (useWords) {
          for (const w of lines[i].words) {
            const span = document.createElement("span");
            span.className = "lyric-word";
            span.textContent = w.text;
            p.appendChild(span);
          }
        } else {
          p.textContent = lines[i].text;
        }
        frag.appendChild(p);
      }
    }
    this.scrollInner.replaceChildren(frag);
    this.karaoke.reattach();
    this.karaoke.invalidate();
  }

  private escape(s: string) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  }
}

function cappedLineVisualEnd(line: LrcLine, nextStartMs: number): number {
  const last = line.words[line.words.length - 1]?.endMs;
  if (last != null) return last;
  return Math.min(nextStartMs, line.timeMs + 4000);
}

function shouldArmScroll(
  lines: LrcLine[],
  idx: number,
  nextIdx: number,
  timeMs: number,
): boolean {
  const line = lines[idx];
  const next = lines[nextIdx];
  const lastEnd = cappedLineVisualEnd(line, next.timeMs);
  const startAt = scrollStartTimeMs(line.timeMs, next.timeMs, lastEnd);
  return timeMs >= startAt && timeMs < next.timeMs;
}

function instrumentalRest(
  lines: LrcLine[],
  idx: number,
  timeMs: number,
): boolean {
  const line = lines[idx];
  const next = lines[idx + 1];
  if (!next) return false;
  const lastEnd = cappedLineVisualEnd(line, next.timeMs);
  const startAt = scrollStartTimeMs(line.timeMs, next.timeMs, lastEnd);
  if (timeMs < lastEnd) return false;
  if (timeMs >= startAt) return false;
  return timeMs >= lastEnd + 80;
}
