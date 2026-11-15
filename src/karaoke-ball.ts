import {
  bouncerSvgMarkup,
} from "./bouncers/svg-store";
import {
  type BouncerId,
  DEFAULT_BOUNCER_ID,
  normalizeBouncerId,
  RTL_MIRROR_BOUNCERS,
} from "./bouncers/ids";
import {
  cometTailOpacity,
  computeBouncerMotion,
  eqBarScales,
} from "./bouncers/motion";
import {
  findActiveWordIndex,
  type TimedWord,
  wordProgress,
} from "./word-timing";

export const BOUNCER_PX = 24;
export const KARAOKE_ARC_PX = 16;
export const KARAOKE_CLEARANCE_PX = 4;
export const KARAOKE_LANE_PX =
  BOUNCER_PX + KARAOKE_ARC_PX + KARAOKE_CLEARANCE_PX + 8;

/** @deprecated use BOUNCER_PX */
export const KARAOKE_BALL_PX = BOUNCER_PX;

export const DEFAULT_BALL_COLOR = "#ffc857";
export const DEFAULT_SUNG_HIGHLIGHT = "#ffdc8c";

interface RectAnchor {
  landX: number;
  landY: number;
}

export interface WordAnchor {
  index: number;
  landX: number;
  landY: number;
  fragments: RectAnchor[];
}

export interface CrossLineHandoff {
  fromLine: HTMLElement;
  toLine: HTMLElement;
  scrollTranslateY: number;
  t: number;
}

export class KaraokeBall {
  private readonly container: HTMLElement;
  private readonly hostEl: HTMLDivElement;
  private readonly shadowEl: HTMLDivElement;
  private readonly spriteEl: HTMLDivElement;
  private anchors: WordAnchor[] = [];
  private measureGeneration = 0;
  private activeLineEl: HTMLElement | null = null;
  private visible = false;
  private ballColor = DEFAULT_BALL_COLOR;
  private sungHighlight = DEFAULT_SUNG_HIGHLIGHT;
  private bouncerId: BouncerId = DEFAULT_BOUNCER_ID;
  constructor(linesContainer: HTMLElement) {
    this.container = linesContainer;
    this.container.classList.add("karaoke-stage");

    this.hostEl = document.createElement("div");
    this.hostEl.className = "karaoke-bouncer";
    this.hostEl.dataset.bouncer = DEFAULT_BOUNCER_ID;
    this.hostEl.setAttribute("aria-hidden", "true");

    this.shadowEl = document.createElement("div");
    this.shadowEl.className = "karaoke-bouncer-shadow";

    this.spriteEl = document.createElement("div");
    this.spriteEl.className = "karaoke-bouncer-sprite";
    this.spriteEl.innerHTML = bouncerSvgMarkup(DEFAULT_BOUNCER_ID);

    this.hostEl.append(this.shadowEl, this.spriteEl);
    this.container.appendChild(this.hostEl);
    this.applyLaneCss();
    this.applyThemeColors();
    this.hide();
  }

  private applyLaneCss() {
    this.container.style.setProperty(
      "--karaoke-reserve",
      `${KARAOKE_LANE_PX}px`,
    );
    this.container.style.setProperty("--karaoke-arc", `${KARAOKE_ARC_PX}px`);
  }

  setBouncerType(id: string) {
    const next = normalizeBouncerId(id);
    if (next === this.bouncerId) return;
    this.bouncerId = next;
    this.hostEl.dataset.bouncer = next;
    this.spriteEl.innerHTML = bouncerSvgMarkup(next);
    this.invalidate();
  }

  getBouncerType(): BouncerId {
    return this.bouncerId;
  }

  setThemeColors(ballColor: string, sungHighlight: string) {
    this.ballColor = ballColor || DEFAULT_BALL_COLOR;
    this.sungHighlight = sungHighlight || DEFAULT_SUNG_HIGHLIGHT;
    this.applyThemeColors();
  }

  private applyThemeColors() {
    this.container.style.setProperty("--karaoke-ball-color", this.ballColor);
    this.container.style.setProperty("--sung-highlight", this.sungHighlight);
    const sungMuted = `color-mix(in srgb, ${this.sungHighlight} 72%, #fff8e7)`;
    this.container.style.setProperty("--sung-color", sungMuted);
    this.hostEl.style.color = this.ballColor;
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
    return this.measureWords(lineEl).find((a) => a.index === wordIndex) ?? null;
  }

  private pickFragment(anchor: WordAnchor, towardX?: number): RectAnchor {
    if (anchor.fragments.length <= 1) {
      return { landX: anchor.landX, landY: anchor.landY };
    }
    if (towardX == null) {
      return anchor.fragments.reduce((a, b) => (a.landY <= b.landY ? a : b));
    }
    return anchor.fragments.reduce((best, cur) => {
      const bd = Math.abs(cur.landX - towardX);
      const ad = Math.abs(best.landX - towardX);
      return bd < ad ? cur : best;
    });
  }

  private lineIsRtl(lineEl: HTMLElement | null): boolean {
    if (!lineEl) return false;
    const dir = lineEl.getAttribute("dir");
    if (dir === "rtl") return true;
    if (dir === "ltr") return false;
    return getComputedStyle(lineEl).direction === "rtl";
  }

  private applySubElementMotion(
    hopProgress: number,
    heldNote: boolean,
    _wordIndex: number,
    motion: ReturnType<typeof computeBouncerMotion>,
  ) {
    const svg = this.spriteEl.querySelector("svg");
    if (!svg) return;

    const glow = svg.querySelector<SVGElement>(".lg-glow");

    if (this.bouncerId === "eq-pulse") {
      const scales = eqBarScales(hopProgress, heldNote);
      const bars = svg.querySelectorAll<SVGRectElement>(".lg-bar");
      bars.forEach((bar, i) => {
        const sy = scales[i] ?? 1;
        bar.style.transform = `scaleY(${sy.toFixed(3)})`;
        bar.style.transformOrigin = "center bottom";
      });
    }

    if (this.bouncerId === "comet") {
      const op = cometTailOpacity(hopProgress);
      svg.querySelectorAll<SVGPathElement>(".lg-tail").forEach((tail, i) => {
        tail.style.opacity = String(op[i] ?? 0.5);
      });
    }

    if (this.bouncerId === "music-note") {
      const flag = svg.querySelector<SVGPathElement>(".lg-flag");
      if (flag) {
        const lag = Math.max(0, hopProgress - 0.08);
        flag.style.transform = `rotate(${(-6 * Math.sin(lag * Math.PI)).toFixed(2)}deg)`;
        flag.style.transformOrigin = "14px 8px";
      }
    }

    if (glow && motion.glowOpacity != null) {
      glow.style.opacity = String(motion.glowOpacity);
    }

    this.shadowEl.style.opacity = String(motion.shadowOpacity ?? 0);
    this.hostEl.classList.toggle("karaoke-flash", Boolean(motion.flash));
  }

  private applyPose(
    x: number,
    y: number,
    hopProgress: number,
    wordIndex: number,
    words: TimedWord[],
    timeMs: number,
  ) {
    const idx = findActiveWordIndex(words, timeMs);
    const w = idx >= 0 ? words[idx] : null;
    const heldNote = Boolean(w && w.endMs - w.startMs > 700 && hopProgress > 0.25 && hopProgress < 0.92);

    const rtl =
      RTL_MIRROR_BOUNCERS.has(this.bouncerId) &&
      this.lineIsRtl(this.activeLineEl);
    const motion = computeBouncerMotion({
      id: this.bouncerId,
      hopProgress,
      rtl,
      heldNote,
      wordIndex,
      interWordBlend: hopProgress > 0.5 ? (hopProgress - 0.5) * 2 : 0,
    });

    this.applySubElementMotion(hopProgress, heldNote, wordIndex, motion);

    const hostParts = [`translate(${x}px, ${y}px)`, "translate(-50%, -100%)"];
    if (motion.hostTransform) hostParts.push(motion.hostTransform);
    this.hostEl.style.transform = hostParts.join(" ");
    this.spriteEl.style.transform = motion.localTransform;
    this.hostEl.style.opacity = "1";
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

    this.activeLineEl = handoff.toLine;
    this.applyPose(x, baseY - arc, eased, fromWords - 1, [], handoff.t);
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
        this.activeLineEl = line;
        const frag = this.pickFragment(anchor);
        const hopP = 0.85;
        this.applyPose(frag.landX, frag.landY + scrollY, hopP, lastIdx, w, timeMs);
        this.updateWordHighlights(line, w, w[lastIdx]?.endMs ?? timeMs);
      }
      return;
    }

    if (!activeLine || words.length === 0) {
      this.hide();
      return;
    }

    this.activeLineEl = activeLine;
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
    const hopProgress = progress;

    const destFrag = nextAnchor ? this.pickFragment(nextAnchor, undefined) : undefined;
    const startFrag = this.pickFragment(anchor, destFrag?.landX);

    let x = startFrag.landX;
    let y = startFrag.landY + scrollY;

    if (nextAnchor && destFrag && progress > 0.5) {
      const t = (progress - 0.5) / 0.5;
      const eased = t * t * (3 - 2 * t);
      const endFrag = this.pickFragment(nextAnchor, startFrag.landX);
      x = startFrag.landX + (endFrag.landX - startFrag.landX) * eased;
      const baseY = Math.min(startFrag.landY, endFrag.landY) + scrollY;
      const arc = Math.sin(eased * Math.PI) * KARAOKE_ARC_PX;
      y = baseY - arc;
    }

    this.applyPose(x, y, hopProgress, idx, words, timeMs);
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
    this.hostEl.style.opacity = "0";
  }

  reattach() {
    if (!this.container.contains(this.hostEl)) {
      this.container.appendChild(this.hostEl);
    }
  }
}