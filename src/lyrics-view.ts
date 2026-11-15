import type { PlaybackClockMedia } from "./playback-clock";
import { KaraokeBall, type CrossLineHandoff } from "./karaoke-ball";
import {
  findActiveLineIndex,
  parseLrc,
  type LrcLine,
  type ParsedLrc,
} from "./lrc";
import {
  SCROLL_LONG_GAP_MS,
  type LineScrollAnimation,
  scrollDurationMs,
  scrollProgress,
  scrollStartTimeMs,
} from "./lyrics-scroll";
import type { TimedWord } from "./word-timing";
import {
  plainScrollOffsetPx,
  textLooksHebrew,
} from "./plain-scroll";

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
  lyricsSource?: string;
  shironetWrkid?: number | null;
  shironetPrfid?: number | null;
  shironetMatchTitle?: string | null;
  shironetMatchArtist?: string | null;
  shironetPageUrl?: string | null;
}

export interface LineDiagnostics {
  index: number;
  lineTimeMs: number;
  nextLineTimeMs: number;
  computedMs: number;
}

interface RenderOptions {
  wordLineIndices?: Set<number>;
}

export interface IdleBounceTarget {
  lineIndex: number;
  hopAtMs: number;
  words: TimedWord[];
}

/** Playback time jump that triggers scroll snap (ms). */
export const SEEK_JUMP_MS = 2_000;

export class LyricsView {
  private syncOffsetMs = 0;
  private parsed: ParsedLrc | null = null;
  private lastLyrics: LyricsResponse | null = null;
  private statusMessage = "Play a song in Spotify";

  private trackKey = "";
  private lastLineIndex = -2;
  private lastTickPlaybackMs = -1;
  private karaokeEnabled = true;
  private readonly karaoke: KaraokeBall;
  private resizeObserver: ResizeObserver | null = null;

  private readonly headerTitle: HTMLElement;
  private readonly headerArtist: HTMLElement;
  private readonly statusEl: HTMLElement;
  private readonly linesEl: HTMLElement;
  private readonly scrollInner: HTMLDivElement;
  private readonly plainEl: HTMLElement;
  private readonly plainScrollInner: HTMLDivElement;
  private readonly offsetEl: HTMLElement;
  private readonly durationWarnEl: HTMLElement | null;

  private plainMode = false;
  private plainScrollDisplayed = 0;

  private snapScroll = true;
  private scrollAnim: LineScrollAnimation | null = null;
  private renderedCenterIndex = -2;

  constructor(root: HTMLElement) {
    this.headerTitle = root.querySelector("#np-title")!;
    this.headerArtist = root.querySelector("#np-artist")!;
    this.statusEl = root.querySelector("#lyrics-status")!;
    this.linesEl = root.querySelector("#lyrics-lines")!;
    this.plainEl = root.querySelector("#lyrics-plain")!;
    this.plainScrollInner = document.createElement("div");
    this.plainScrollInner.className = "plain-scroll-inner";
    this.plainScrollInner.dataset.testid = "plain-scroll-inner";
    this.plainEl.appendChild(this.plainScrollInner);
    this.offsetEl = root.querySelector("#sync-offset-label")!;
    this.durationWarnEl = document.getElementById("duration-warn");
    this.scrollInner = document.createElement("div");
    this.scrollInner.className = "lyrics-scroll-inner";
    this.scrollInner.dataset.testid = "lyrics-scroll-inner";
    this.linesEl.appendChild(this.scrollInner);
    this.karaoke = new KaraokeBall(this.linesEl);
    if (typeof ResizeObserver !== "undefined") {
      this.resizeObserver = new ResizeObserver(() => this.karaoke.invalidate());
      this.resizeObserver.observe(this.linesEl);
    }
  }

  /** Test hook: current scroll-inner translateY (px). */
  getScrollTranslateY(): number {
    const tr = this.scrollInner.style.transform;
    const m = /translate3d\(\s*0(?:px)?,\s*([-\d.]+)px/.exec(tr);
    return m ? Number(m[1]) : 0;
  }

  /** Test hook: bouncer position in lines container (px). */
  getBouncerClientPosition(): { x: number; y: number } | null {
    return this.karaoke.getClientPosition();
  }

  /** Test hook: plain inner translateY (px). */
  getPlainScrollTranslateY(): number {
    const tr = this.plainScrollInner.style.transform;
    const m = /translate3d\(\s*0(?:px)?,\s*([-\d.]+)px/.exec(tr);
    return m ? Number(m[1]) : 0;
  }

  getPlainMode(): boolean {
    return this.plainMode;
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
    this.lastTickPlaybackMs = -1;
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
    this.lastTickPlaybackMs = -1;
    void options;
  }

  private resetScroll() {
    this.scrollAnim = null;
    this.scrollInner.style.transform = "";
  }

  private applyScrollTransform(timeMs: number) {
    if (!this.scrollAnim) return;
    const p = scrollProgress(this.scrollAnim, timeMs);
    const ty = -this.scrollAnim.deltaY * p;
    this.scrollInner.style.transform = `translate3d(0, ${ty}px, 0)`;
  }

  private snapToLine(idx: number) {
    this.resetScroll();
    if (this.parsed) {
      if (idx < 0) this.renderLines(this.parsed.lines, -1);
      else this.renderLines(this.parsed.lines, idx);
    }
    this.renderedCenterIndex = idx;
    this.lastLineIndex = idx;
    this.snapScroll = false;
  }

  private isSeekLike(playbackMs: number, idx: number): boolean {
    if (this.snapScroll) return true;
    if (this.lastTickPlaybackMs < 0) return this.snapScroll;
    const dt = playbackMs - this.lastTickPlaybackMs;
    if (dt > SEEK_JUMP_MS || playbackMs < this.lastTickPlaybackMs - 80) {
      return true;
    }
    if (
      idx >= 0 &&
      this.lastLineIndex >= 0 &&
      Math.abs(idx - this.lastLineIndex) > 1
    ) {
      return true;
    }
    if (idx < this.lastLineIndex && this.lastLineIndex >= 0) return true;
    return false;
  }

  onLyricsLoading() {
    this.statusMessage = "Loading lyrics…";
    this.statusEl.textContent = this.statusMessage;
    this.scrollInner.replaceChildren();
    this.plainEl.classList.add("hidden");
    this.plainMode = false;
    this.plainScrollInner.replaceChildren();
    this.plainScrollInner.style.transform = "";
    this.linesEl.classList.remove("hidden");
    this.parsed = null;
    this.lastLyrics = null;
    this.hideDurationWarning();
    this.resetScroll();
    this.lastLineIndex = -2;
    this.lastTickPlaybackMs = -1;
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
      this.plainMode = false;
      this.linesEl.classList.remove("hidden");
      this.linesEl.classList.toggle(
        "lyrics-hebrew",
        textLooksHebrew(res.syncedLrc),
      );
    } else if (res.plainLyrics) {
      this.statusEl.textContent = res.message;
      this.parsed = null;
      this.plainMode = true;
      this.plainScrollDisplayed = 0;
      const rtl = textLooksHebrew(res.plainLyrics);
      this.plainEl.dir = rtl ? "rtl" : "auto";
      this.plainEl.classList.toggle("plain-rtl", rtl);
      this.plainScrollInner.textContent = res.plainLyrics;
      this.plainScrollInner.style.transform = "";
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
    this.lastTickPlaybackMs = -1;
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
    this.lastTickPlaybackMs = -1;
    return true;
  }

  onMediaHeader(media: MediaState) {
    this.headerTitle.textContent = media.title || "—";
    this.headerArtist.textContent = media.artist || "—";
  }

  onSeek() {
    this.snapScroll = true;
    this.lastLineIndex = -2;
    this.lastTickPlaybackMs = -1;
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
    const dy = toEl.offsetTop - fromEl.offsetTop;
    if (dy >= 2) return dy;
    const est = (fromEl.offsetHeight || 28) + 8;
    return Math.max(est, 24);
  }

  private beginScroll(fromIndex: number, toIndex: number, timeMs: number) {
    if (!this.parsed || this.scrollAnim) return;
    const lines = this.parsed.lines;
    const fromLine = lines[fromIndex];
    const toLine = lines[toIndex];
    if (!fromLine || !toLine) return;

    const lastEnd = cappedLineVisualEnd(fromLine, toLine.timeMs);
    let startAt = scrollStartTimeMs(
      fromLine.timeMs,
      toLine.timeMs,
      lastEnd,
    );
    let duration = scrollDurationMs(Math.max(40, toLine.timeMs - startAt));
    if (timeMs >= toLine.timeMs - 40) {
      startAt = Math.min(timeMs, toLine.timeMs) - duration;
    } else if (timeMs > startAt) {
      const remaining = Math.max(40, toLine.timeMs - timeMs);
      duration = scrollDurationMs(remaining);
      startAt = Math.max(fromLine.timeMs, timeMs - 40);
    }

    const wordLines = new Set<number>([fromIndex, toIndex]);
    if (this.karaokeEnabled) {
      wordLines.add(fromIndex);
      wordLines.add(toIndex);
    }
    const haveBoth =
      this.lineByIndex(fromIndex) && this.lineByIndex(toIndex);
    if (!haveBoth) {
      this.renderLines(lines, fromIndex, { wordLineIndices: wordLines });
    } else {
      this.ensureWordSpans(lines, wordLines);
    }

    const deltaY = this.measureScrollDelta(fromIndex, toIndex);
    this.scrollAnim = {
      fromIndex,
      toIndex,
      startTimeMs: startAt,
      durationMs: duration,
      deltaY,
    };
    this.renderedCenterIndex = fromIndex;
    this.applyScrollTransform(timeMs);
  }

  private ensureWordSpans(lines: LrcLine[], wordLineIndices: Set<number>) {
    if (!this.karaokeEnabled) return;
    for (const i of wordLineIndices) {
      const el = this.lineByIndex(i);
      if (!el || lines[i].words.length === 0) continue;
      if (el.querySelector(".lyric-word")) continue;
      el.replaceChildren();
      for (const w of lines[i].words) {
        const span = document.createElement("span");
        span.className = "lyric-word";
        span.textContent = w.text;
        el.appendChild(span);
      }
    }
    this.karaoke.invalidate();
  }

  private finishScrollIfDone(timeMs: number, activeIdx: number) {
    if (!this.scrollAnim) return;
    this.applyScrollTransform(timeMs);
    const p = scrollProgress(this.scrollAnim, timeMs);
    if (p < 1) return;

    const target = this.scrollAnim.toIndex;
    this.scrollInner.style.transform = "";
    this.scrollAnim = null;
    this.promoteLineClasses(target, this.parsed!.lines);
    this.renderedCenterIndex = target;
    this.lastLineIndex = activeIdx >= 0 ? activeIdx : target;
    this.karaoke.invalidate();
  }

  private promoteLineClasses(active: number, lines: LrcLine[]) {
    const start = Math.max(0, active - 2);
    const end = Math.min(lines.length - 1, Math.max(active + 3, active + 1));
    const karaokeOn = this.karaokeEnabled;
    const existing = new Set<number>();
    this.scrollInner.querySelectorAll<HTMLElement>(".lyric-line").forEach((el) => {
      const i = Number(el.dataset.line);
      existing.add(i);
      el.classList.remove("past", "active", "upcoming");
      if (i < active) el.classList.add("past");
      else if (i === active) el.classList.add("active");
      else el.classList.add("upcoming");
    });

    if (end - start + 1 > existing.size) {
      this.renderLines(lines, active);
      return;
    }

    for (let i = start; i <= end; i += 1) {
      if (!existing.has(i)) {
        this.renderLines(lines, active);
        return;
      }
    }

    if (karaokeOn) {
      const wordLines = new Set<number>([active]);
      if (active + 1 < lines.length) wordLines.add(active + 1);
      this.ensureWordSpans(lines, wordLines);
    }
  }

  private promoteLineClassesSimple(active: number) {
    if (!this.parsed) return;
    this.promoteLineClasses(active, this.parsed.lines);
  }

  resolveIdleBounce(timeMs: number, lines: LrcLine[], idx: number): IdleBounceTarget | null {
    if (lines.length === 0) return null;
    if (timeMs < lines[0].timeMs) {
      return {
        lineIndex: 0,
        hopAtMs: lines[0].timeMs,
        words: lines[0].words,
      };
    }
    if (idx >= 0 && idx + 1 < lines.length) {
      const next = lines[idx + 1];
      if (next.timeMs - lines[idx].timeMs >= SCROLL_LONG_GAP_MS) {
        const vocalEnd = cappedLineVisualEnd(lines[idx], next.timeMs);
        if (timeMs >= vocalEnd + 40 && timeMs < next.timeMs) {
          return {
            lineIndex: idx + 1,
            hopAtMs: next.timeMs,
            words: next.words,
          };
        }
      }
    }
    return null;
  }

  tick(playbackMs: number, durationMs = 0) {
    if (this.plainMode) {
      this.tickPlainScroll(playbackMs, durationMs);
      return;
    }
    if (!this.parsed || this.parsed.lines.length === 0) return;

    const timeMs = playbackMs + this.syncOffsetMs;
    const idx = findActiveLineIndex(this.parsed.lines, timeMs);
    const lines = this.parsed.lines;
    const seek = this.isSeekLike(playbackMs, idx);

    if (seek) {
      this.snapToLine(idx >= 0 ? idx : -1);
    } else {
      if (this.renderedCenterIndex === -2) {
        if (timeMs < lines[0].timeMs) {
          this.renderLines(lines, -1, {
            wordLineIndices: new Set(this.karaokeEnabled ? [0] : []),
          });
          this.renderedCenterIndex = -1;
          this.lastLineIndex = -1;
        } else if (idx >= 0) {
          this.renderLines(lines, idx);
          this.renderedCenterIndex = idx;
          this.lastLineIndex = idx;
        }
      }

      if (
        !this.scrollAnim &&
        idx >= 0 &&
        idx + 1 < lines.length &&
        shouldArmScroll(lines, idx, idx + 1, timeMs)
      ) {
        this.beginScroll(idx, idx + 1, timeMs);
      }

      if (
        !this.scrollAnim &&
        this.lastLineIndex >= 0 &&
        idx === this.lastLineIndex + 1
      ) {
        this.beginScroll(this.lastLineIndex, idx, timeMs);
      }

      if (this.scrollAnim) {
        this.finishScrollIfDone(timeMs, idx);
        const anim = this.scrollAnim;
        if (anim && (idx === anim.toIndex || idx > anim.fromIndex)) {
          this.lastLineIndex = Math.max(this.lastLineIndex, idx);
        }
      } else if (idx !== this.lastLineIndex && idx >= 0) {
        this.promoteLineClassesSimple(idx);
        this.lastLineIndex = idx;
        this.renderedCenterIndex = idx;
      }
    }

    this.lastTickPlaybackMs = playbackMs;

    const scrollY = this.getScrollTranslateY();

    const karaokeOn = this.karaokeEnabled && this.parsed !== null;
    const idle = karaokeOn ? this.resolveIdleBounce(timeMs, lines, idx) : null;

    let crossLine: CrossLineHandoff | null = null;
    let restOnLast: { line: HTMLElement; words: LrcLine["words"] } | null = null;

    if (idle && !this.scrollAnim && timeMs < idle.hopAtMs) {
      const lineEl = this.lineByIndex(idle.lineIndex);
      if (lineEl) {
        this.karaoke.updateIdleCenter(lineEl, timeMs, idle.hopAtMs, scrollY);
        return;
      }
    }

    if (
      idle &&
      timeMs >= idle.hopAtMs &&
      timeMs < idle.hopAtMs + 500 &&
      idx < idle.lineIndex
    ) {
      const lineEl = this.lineByIndex(idle.lineIndex);
      if (lineEl) {
        this.karaoke.updateIdleHopToFirstWord(
          lineEl,
          idle.words,
          timeMs,
          idle.hopAtMs,
          scrollY,
        );
        return;
      }
    }

    const activeEl = this.lineByIndex(idx);
    const words = idx >= 0 && karaokeOn ? lines[idx].words : [];

    if (
      karaokeOn &&
      this.scrollAnim &&
      !seek &&
      timeMs >= this.scrollAnim.startTimeMs
    ) {
      const fromEl = this.lineByIndex(this.scrollAnim.fromIndex);
      const toEl = this.lineByIndex(this.scrollAnim.toIndex);
      if (fromEl && toEl) {
        crossLine = {
          fromLine: fromEl,
          toLine: toEl,
          scrollTranslateY: scrollY,
          t: scrollProgress(this.scrollAnim, timeMs),
        };
      }
    }

    if (
      karaokeOn &&
      !this.scrollAnim &&
      !crossLine &&
      idx >= 0 &&
      idx + 1 < lines.length
    ) {
      const gapRest = shortInstrumentalRest(lines, idx, timeMs);
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
      for (let i = 0; i <= Math.min(1, lines.length - 1); i += 1) {
        const p = document.createElement("p");
        p.className = "lyric-line";
        p.dir = "auto";
        p.dataset.line = String(i);
        if (i === 0) p.classList.add("upcoming");
        else p.classList.add("upcoming");
        const useWords =
          karaokeOn && i === 0 && lines[0].words.length > 0 && wordLines.has(0);
        if (useWords) {
          for (const w of lines[0].words) {
            const span = document.createElement("span");
            span.className = "lyric-word";
            span.textContent = w.text;
            p.appendChild(span);
          }
        } else {
          p.textContent = lines[i]?.text ?? "";
        }
        frag.appendChild(p);
      }
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
    const preserveTransform = this.scrollInner.style.transform;
    this.scrollInner.replaceChildren(frag);
    if (this.scrollAnim && preserveTransform) {
      this.scrollInner.style.transform = preserveTransform;
    }
    this.karaoke.reattach();
    this.karaoke.invalidate();
  }

  private escape(s: string) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  }

  private tickPlainScroll(playbackMs: number, durationMs: number) {
    if (durationMs <= 0) return;
    const timeMs = playbackMs + this.syncOffsetMs;
    const contentH = this.plainScrollInner.scrollHeight;
    const viewH = this.plainEl.clientHeight;
    const target = plainScrollOffsetPx(timeMs, durationMs, contentH, viewH);
    const alpha = 0.18;
    this.plainScrollDisplayed +=
      (target - this.plainScrollDisplayed) * alpha;
    this.plainScrollInner.style.transform = `translate3d(0, ${-this.plainScrollDisplayed}px, 0)`;
  }
}

/** End of sung content on a line (ignores LRCLIB “held” last tokens through instrumental). */
export function lineVocalEndMs(line: LrcLine, nextLineStartMs: number): number {
  if (line.words.length === 0) {
    return Math.min(nextLineStartMs, line.timeMs + 400);
  }
  const last = line.words[line.words.length - 1]!;
  const gap = nextLineStartMs - last.startMs;
  const dur = last.endMs - last.startMs;
  if (dur > 900 && gap > dur * 0.85) {
    const prev = line.words[line.words.length - 2];
    if (prev) return prev.endMs;
    return last.startMs + 280;
  }
  return Math.min(last.endMs, nextLineStartMs);
}

function cappedLineVisualEnd(line: LrcLine, nextStartMs: number): number {
  return lineVocalEndMs(line, nextStartMs);
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

function shortInstrumentalRest(
  lines: LrcLine[],
  idx: number,
  timeMs: number,
): boolean {
  const line = lines[idx];
  const next = lines[idx + 1];
  if (!next) return false;
  const gap = next.timeMs - line.timeMs;
  if (gap >= SCROLL_LONG_GAP_MS) return false;
  const lastEnd = cappedLineVisualEnd(line, next.timeMs);
  const startAt = scrollStartTimeMs(line.timeMs, next.timeMs, lastEnd);
  if (timeMs < lastEnd) return false;
  if (timeMs >= startAt) return false;
  return timeMs >= lastEnd + 80;
}