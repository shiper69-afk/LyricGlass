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
  computeKaraokeSync,
  deriveBallState,
  highlightedWordIndex,
  type KaraokeBallState,
  type KaraokeSyncState,
} from "./karaoke-sync";
import type { TimedWord } from "./word-timing";

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
    this.anchors = this.measureWords(activeLine);
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
    sync: KaraokeSyncState,
    words: TimedWord[],
  ) {
    const hopProgress = sync.progress;
    const wordIndex = sync.activeIndex;
    const w = words[wordIndex];
    const heldNote = Boolean(
      w && w.endMs - w.startMs > 700 && hopProgress > 0.25 && hopProgress < 0.92,
    );

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
    const crossSync: KaraokeSyncState = {
      timeMs: 0,
      activeIndex: fromWords - 1,
      progress: eased,
      interWordT: eased,
    };
    this.applyPose(x, baseY - arc, crossSync, []);
    const lastIdx = Math.max(0, fromWords - 1);
    const fromBall: KaraokeBallState = {
      sync: crossSync,
      ballWordIndex: lastIdx,
      positionFrom: lastIdx,
      positionTo: lastIdx,
      positionBlend: 0,
    };
    this.applyWordHighlights(handoff.fromLine, fromBall);
    const toBall: KaraokeBallState = {
      sync: crossSync,
      ballWordIndex: eased >= 1 ? 0 : -1,
      positionFrom: 0,
      positionTo: 0,
      positionBlend: 0,
    };
    if (toBall.ballWordIndex >= 0) {
      this.applyWordHighlights(handoff.toLine, toBall);
    } else {
      this.updateWordHighlights(handoff.toLine, [], "none");
    }
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
        const restSync: KaraokeSyncState = {
          timeMs,
          activeIndex: lastIdx,
          progress: hopP,
          interWordT: 0,
        };
        const ball = deriveBallState(restSync, w.length);
        this.applyPose(frag.landX, frag.landY + scrollY, restSync, w);
        this.applyWordHighlights(line, ball);
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

    const sync = computeKaraokeSync(words, timeMs);
    if (!sync) {
      this.hide();
      return;
    }

    const ball = deriveBallState(sync, words.length);
    const { positionFrom: from, positionTo: to, positionBlend: blend } = ball;
    const fromAnchor = this.anchors[from];
    if (!fromAnchor) return;

    const startFrag = this.pickFragment(fromAnchor);
    let x = startFrag.landX;
    let y = startFrag.landY + scrollY;

    if (blend > 0 && to !== from) {
      const nextAnchor = this.anchors[to];
      if (nextAnchor) {
        const endFrag = this.pickFragment(nextAnchor, startFrag.landX);
        x = startFrag.landX + (endFrag.landX - startFrag.landX) * blend;
        const baseY = Math.min(startFrag.landY, endFrag.landY) + scrollY;
        const arc = Math.sin(blend * Math.PI) * KARAOKE_ARC_PX;
        y = baseY - arc;
      }
    }

    this.applyPose(x, y, sync, words);
    this.applyWordHighlights(activeLine, ball);
  }

  private applyWordHighlights(
    lineEl: HTMLElement,
    ball: KaraokeBallState,
  ) {
    const coloredThrough = highlightedWordIndex(ball);
    const spans = lineEl.querySelectorAll<HTMLElement>(".lyric-word");
    spans.forEach((span, i) => {
      span.classList.remove("sung", "singing");
      span.style.removeProperty("--sung-frac");
      if (i <= coloredThrough) {
        span.classList.add("sung");
      }
    });
  }

  private updateWordHighlights(
    lineEl: HTMLElement,
    _words: TimedWord[],
    sync: KaraokeSyncState | KaraokeBallState | "all-sung" | "none",
  ) {
    const spans = lineEl.querySelectorAll<HTMLElement>(".lyric-word");
    if (sync === "all-sung") {
      spans.forEach((span) => {
        span.classList.add("sung");
        span.classList.remove("singing");
        span.style.removeProperty("--sung-frac");
      });
      return;
    }
    if (sync === "none") {
      spans.forEach((span) => {
        span.classList.remove("sung", "singing");
        span.style.removeProperty("--sung-frac");
      });
      return;
    }
    const ball =
      "ballWordIndex" in sync
        ? sync
        : deriveBallState(sync, spans.length);
    this.applyWordHighlights(lineEl, ball);
  }

  getClientPosition(): { x: number; y: number } | null {
    if (this.hostEl.style.opacity === "0") return null;
    const r = this.hostEl.getBoundingClientRect();
    const c = this.container.getBoundingClientRect();
    return {
      x: r.left - c.left + r.width / 2,
      y: r.bottom - c.top,
    };
  }

  /** Centered bounce above an upcoming line (intro / long instrumental). */
  updateIdleCenter(
    lineEl: HTMLElement,
    timeMs: number,
    _hopAtMs: number,
    scrollY: number,
  ) {
    this.activeLineEl = lineEl;
    const containerRect = this.container.getBoundingClientRect();
    const lineRect = lineEl.getBoundingClientRect();
    const x = containerRect.width / 2;
    const landY = lineRect.top - containerRect.top - KARAOKE_CLEARANCE_PX;
    const phase = (timeMs % 900) / 900;
    const sync: KaraokeSyncState = {
      timeMs,
      activeIndex: 0,
      progress: phase,
      interWordT: 0,
    };
    this.applyPose(x, landY + scrollY, sync, []);
    this.updateWordHighlights(lineEl, [], "none");
  }

  /** Hop from center onto word 0 at line start. */
  updateIdleHopToFirstWord(
    lineEl: HTMLElement,
    words: TimedWord[],
    timeMs: number,
    hopAtMs: number,
    scrollY: number,
  ) {
    this.activeLineEl = lineEl;
    this.scheduleMeasure(lineEl);
    const hopDur = Math.max(280, Math.min(450, words[0]?.endMs - hopAtMs || 400));
    const t = Math.max(0, Math.min(1, (timeMs - hopAtMs) / hopDur));
    const containerRect = this.container.getBoundingClientRect();
    const lineRect = lineEl.getBoundingClientRect();
    const cx = containerRect.width / 2;
    const landY = lineRect.top - containerRect.top - KARAOKE_CLEARANCE_PX;
    const anchor = this.anchors[0] ?? this.measureWords(lineEl)[0];
    if (!anchor) return;

    const endFrag = this.pickFragment(anchor);
    const eased = t * t * (3 - 2 * t);
    const x = cx + (endFrag.landX - cx) * eased;
    const arc = Math.sin(eased * Math.PI) * KARAOKE_ARC_PX;
    const y = landY + scrollY - arc + (endFrag.landY - landY) * eased;

    const sync: KaraokeSyncState = {
      timeMs,
      activeIndex: 0,
      progress: eased,
      interWordT: eased < 1 ? 0 : 0,
    };
    this.applyPose(x, y, sync, words);
    const live = computeKaraokeSync(words, timeMs);
    if (timeMs >= hopAtMs && live) {
      const ball = deriveBallState(live, words.length);
      this.applyWordHighlights(lineEl, ball);
    } else {
      this.updateWordHighlights(lineEl, [], "none");
    }
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