import {
  findActiveWordIndex,
  type TimedWord,
  wordProgress,
} from "./word-timing";

/** Ball visual size (px) — keep in sync with `.karaoke-ball` width/height. */
export const KARAOKE_BALL_PX = 10;
/** Peak arc lift between words (px). */
export const KARAOKE_ARC_PX = 14;
/** Gap between glyph top and ball bottom (px). */
export const KARAOKE_CLEARANCE_PX = 4;
/** Reserved lane above every line so the arc never overlaps the line above. */
export const KARAOKE_LANE_PX =
  KARAOKE_BALL_PX + KARAOKE_ARC_PX + KARAOKE_CLEARANCE_PX + 6;

export const DEFAULT_BALL_COLOR = "#ffc857";
export const DEFAULT_SUNG_HIGHLIGHT = "#ffdc8c";

interface RectAnchor {
  landX: number;
  landY: number;
}

export interface WordAnchor {
  index: number;
  landX: number;
  /** Y where the ball's bottom should sit (just above word top). */
  landY: number;
  /** Wrapped / bidi fragments (e.g. multi-line word). */
  fragments: RectAnchor[];
}

export interface CrossLineHandoff {
  fromLine: HTMLElement;
  toLine: HTMLElement;
  scrollTranslateY: number;
  /** 0..1 eased progress along the inter-line arc. */
  t: number;
}

export class KaraokeBall {
  private readonly container: HTMLElement;
  private readonly ballEl: HTMLDivElement;
  private anchors: WordAnchor[] = [];
  private measureGeneration = 0;
  private activeLineEl: HTMLElement | null = null;
  private visible = false;
  private ballColor = DEFAULT_BALL_COLOR;
  private sungHighlight = DEFAULT_SUNG_HIGHLIGHT;

  constructor(linesContainer: HTMLElement) {
    this.container = linesContainer;
    this.container.classList.add("karaoke-stage");
    this.ballEl = document.createElement("div");
    this.ballEl.className = "karaoke-ball";
    this.ballEl.setAttribute("aria-hidden", "true");
    this.container.appendChild(this.ballEl);
    this.applyLaneCss();
    this.applyThemeColors();
    this.hide();
  }

  private applyLaneCss() {
    this.container.style.setProperty(
      "--karaoke-reserve",
      `${KARAOKE_LANE_PX}px`,
    );
    this.container.style.setProperty(
      "--karaoke-arc",
      `${KARAOKE_ARC_PX}px`,
    );
  }

  setThemeColors(ballColor: string, sungHighlight: string) {
    this.ballColor = ballColor || DEFAULT_BALL_COLOR;
    this.sungHighlight = sungHighlight || DEFAULT_SUNG_HIGHLIGHT;
    this.applyThemeColors();
  }

  private applyThemeColors() {
    this.container.style.setProperty("--karaoke-ball-color", this.ballColor);
    this.container.style.setProperty("--sung-highlight", this.sungHighlight);
    this.container.style.setProperty(
      "--sung-highlight-solid",
      this.sungHighlight,
    );
    const sungMuted = `color-mix(in srgb, ${this.sungHighlight} 72%, #fff8e7)`;
    this.container.style.setProperty("--sung-color", sungMuted);
  }

  setEnabled(enabled: boolean) {
    this.container.classList.toggle("karaoke-on", enabled);
    if (!enabled) this.hide();
    this.visible = enabled;
  }

  isEnabled() {
    return this.visible;
  }

  invalidate() {
    this.anchors = [];
    this.activeLineEl = null;
    this.measureGeneration += 1;
  }

  scheduleMeasure(activeLine: HTMLElement) {
    if (this.activeLineEl === activeLine && this.anchors.length > 0) return;
    this.activeLineEl = activeLine;
    const gen = ++this.measureGeneration;
    requestAnimationFrame(() => {
      if (gen !== this.measureGeneration) return;
      this.anchors = this.measureWords(activeLine);
    });
  }

  measureWords(lineEl: HTMLElement): WordAnchor[] {
    const containerRect = this.container.getBoundingClientRect();
    const spans = lineEl.querySelectorAll<HTMLElement>(".lyric-word");
    const anchors: WordAnchor[] = [];

    spans.forEach((span, index) => {
      const clientRects = Array.from(span.getClientRects());
      const rects =
        clientRects.length > 0 ? clientRects : [span.getBoundingClientRect()];
      const fragments: RectAnchor[] = [];
      let minTop = Infinity;
      let cxWeighted = 0;
      let wSum = 0;

      for (const r of rects) {
        if (r.width < 0.5 || r.height < 0.5) continue;
        const relTop = r.top - containerRect.top;
        const relLeft = r.left - containerRect.left;
        minTop = Math.min(minTop, relTop);
        const cx = relLeft + r.width / 2;
        const landY = relTop - KARAOKE_CLEARANCE_PX;
        fragments.push({ landX: cx, landY });
        cxWeighted += cx * r.width;
        wSum += r.width;
      }

      if (fragments.length === 0 || !Number.isFinite(minTop)) return;

      const landX = wSum > 0 ? cxWeighted / wSum : fragments[0].landX;
      const landY = minTop - KARAOKE_CLEARANCE_PX;

      anchors.push({ index, landX, landY, fragments });
    });

    return anchors;
  }

  measureWordByIndex(lineEl: HTMLElement, wordIndex: number): WordAnchor | null {
    const anchors = this.measureWords(lineEl);
    const hit = anchors.find((a) => a.index === wordIndex);
    return hit ?? null;
  }

  private pickFragment(anchor: WordAnchor, towardX?: number): RectAnchor {
    if (anchor.fragments.length <= 1) {
      return { landX: anchor.landX, landY: anchor.landY };
    }
    if (towardX == null) {
      return anchor.fragments.reduce((a, b) =>
        a.landY <= b.landY ? a : b,
      );
    }
    return anchor.fragments.reduce((best, cur) => {
      const bd = Math.abs(cur.landX - towardX);
      const ad = Math.abs(best.landX - towardX);
      return bd < ad ? cur : best;
    });
  }

  private applyBallAt(x: number, y: number) {
    this.ballEl.style.transform = `translate(${x}px, ${y}px) translate(-50%, -100%)`;
    this.ballEl.style.opacity = "1";
  }

  private renderCrossLine(handoff: CrossLineHandoff) {
    const fromWords = handoff.fromLine.querySelectorAll(".lyric-word").length;
    const fromAnchor = this.measureWordByIndex(
      handoff.fromLine,
      Math.max(0, fromWords - 1),
    );
    const toAnchor = this.measureWordByIndex(handoff.toLine, 0);
    if (!fromAnchor || !toAnchor) {
      this.hide();
      return;
    }

    const ty = handoff.scrollTranslateY;
    const startFrag = this.pickFragment(fromAnchor, toAnchor.landX);
    const endFrag = this.pickFragment(toAnchor, startFrag.landX);
    const sx = startFrag.landX;
    const sy = startFrag.landY + ty;
    const ex = endFrag.landX;
    const ey = endFrag.landY + ty;
    const t = Math.max(0, Math.min(1, handoff.t));
    const eased = t * t * (3 - 2 * t);
    const x = sx + (ex - sx) * eased;
    const baseY = sy + (ey - sy) * eased;
    const arc = Math.sin(eased * Math.PI) * KARAOKE_ARC_PX;
    this.applyBallAt(x, baseY - arc);
    this.updateWordHighlights(handoff.fromLine, [], Number.POSITIVE_INFINITY);
    this.updateWordHighlights(handoff.toLine, [], -1);
  }

  update(
    words: TimedWord[],
    timeMs: number,
    enabled: boolean,
    activeLine: HTMLElement | null,
    options?: {
      scrollTranslateY?: number;
      crossLine?: CrossLineHandoff | null;
      restOnLastWord?: { line: HTMLElement; words: TimedWord[] } | null;
    },
  ) {
    if (!enabled) {
      this.hide();
      return;
    }

    const scrollY = options?.scrollTranslateY ?? 0;

    if (options?.crossLine) {
      this.renderCrossLine(options.crossLine);
      return;
    }

    if (options?.restOnLastWord) {
      const { line, words: w } = options.restOnLastWord;
      const lastIdx = Math.max(0, w.length - 1);
      const anchor = this.measureWordByIndex(line, lastIdx);
      if (anchor) {
        const frag = this.pickFragment(anchor);
        this.applyBallAt(frag.landX, frag.landY + scrollY);
        this.updateWordHighlights(line, w, w[lastIdx]?.endMs ?? timeMs);
      }
      return;
    }

    if (!activeLine || words.length === 0) {
      this.hide();
      return;
    }

    this.scheduleMeasure(activeLine);
    if (this.anchors.length === 0) return;

    const idx = findActiveWordIndex(words, timeMs);
    if (idx < 0) {
      this.hide();
      return;
    }

    const anchor = this.anchors[idx];
    if (!anchor) return;

    const nextAnchor = this.anchors[idx + 1];
    const progress = wordProgress(words, timeMs);

    const destFrag = nextAnchor
      ? this.pickFragment(nextAnchor, undefined)
      : undefined;
    const startFrag = this.pickFragment(anchor, destFrag?.landX);

    let x = startFrag.landX;
    let y = startFrag.landY + scrollY;

    if (nextAnchor && destFrag && progress > 0.5) {
      const t = (progress - 0.5) / 0.5;
      const eased = t * t * (3 - 2 * t);
      const endFrag = this.pickFragment(nextAnchor, startFrag.landX);
      x = startFrag.landX + (endFrag.landX - startFrag.landX) * eased;
      const baseY =
        Math.min(startFrag.landY, endFrag.landY) + scrollY;
      const arc = Math.sin(eased * Math.PI) * KARAOKE_ARC_PX;
      y = baseY - arc;
    }

    this.applyBallAt(x, y);
    this.updateWordHighlights(activeLine, words, timeMs);
  }

  private updateWordHighlights(
    lineEl: HTMLElement,
    words: TimedWord[],
    timeMs: number,
  ) {
    const spans = lineEl.querySelectorAll<HTMLElement>(".lyric-word");
    if (!Number.isFinite(timeMs)) {
      spans.forEach((span) => {
        span.classList.add("sung");
        span.classList.remove("singing");
        span.style.removeProperty("--sung-frac");
      });
      return;
    }
    if (timeMs < 0) {
      spans.forEach((span) => {
        span.classList.remove("sung", "singing");
        span.style.removeProperty("--sung-frac");
      });
      return;
    }
    const activeIdx = findActiveWordIndex(words, timeMs);
    spans.forEach((span, i) => {
      span.classList.remove("sung", "singing");
      if (i < activeIdx) span.classList.add("sung");
      else if (i === activeIdx) {
        span.classList.add("singing");
        span.style.setProperty("--sung-frac", String(wordProgress(words, timeMs)));
      } else {
        span.style.removeProperty("--sung-frac");
      }
    });
  }

  hide() {
    this.ballEl.style.opacity = "0";
  }

  reattach() {
    if (!this.container.contains(this.ballEl)) {
      this.container.appendChild(this.ballEl);
    }
  }
}