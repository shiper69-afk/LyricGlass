import {
  findActiveWordIndex,
  type TimedWord,
  wordProgress,
} from "./word-timing";

export interface WordAnchor {
  index: number;
  centerX: number;
  centerY: number;
  left: number;
  right: number;
  top: number;
}

export class KaraokeBall {
  private readonly container: HTMLElement;
  private readonly ballEl: HTMLDivElement;
  private anchors: WordAnchor[] = [];
  private measureGeneration = 0;
  private activeLineEl: HTMLElement | null = null;
  private visible = false;

  constructor(linesContainer: HTMLElement) {
    this.container = linesContainer;
    this.container.classList.add("karaoke-stage");
    this.ballEl = document.createElement("div");
    this.ballEl.className = "karaoke-ball";
    this.ballEl.setAttribute("aria-hidden", "true");
    this.container.appendChild(this.ballEl);
    this.hide();
  }

  setEnabled(enabled: boolean) {
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

  /** Measure word centers once per active line (call after layout). */
  scheduleMeasure(activeLine: HTMLElement) {
    if (this.activeLineEl === activeLine && this.anchors.length > 0) return;
    this.activeLineEl = activeLine;
    const gen = ++this.measureGeneration;
    requestAnimationFrame(() => {
      if (gen !== this.measureGeneration) return;
      this.anchors = this.measureWords(activeLine);
    });
  }

  private measureWords(lineEl: HTMLElement): WordAnchor[] {
    const containerRect = this.container.getBoundingClientRect();
    const spans = lineEl.querySelectorAll<HTMLElement>(".lyric-word");
    const anchors: WordAnchor[] = [];
    spans.forEach((span, index) => {
      const rect = span.getBoundingClientRect();
      if (rect.width < 1 && rect.height < 1) return;
      anchors.push({
        index,
        centerX: rect.left - containerRect.left + rect.width / 2,
        centerY: rect.top - containerRect.top + rect.height * 0.85,
        left: rect.left - containerRect.left,
        right: rect.right - containerRect.left,
        top: rect.top - containerRect.top,
      });
    });
    return anchors;
  }

  update(
    words: TimedWord[],
    timeMs: number,
    enabled: boolean,
    activeLine: HTMLElement | null,
  ) {
    if (!enabled || !activeLine || words.length === 0) {
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

    let x = anchor.centerX;
    let y = anchor.centerY;

    const progress = wordProgress(words, timeMs);
    const nextAnchor = this.anchors[idx + 1];
    if (nextAnchor && progress > 0.55) {
      const t = (progress - 0.55) / 0.45;
      const eased = t * t * (3 - 2 * t);
      x = anchor.centerX + (nextAnchor.centerX - anchor.centerX) * eased;
      const arc = Math.sin(eased * Math.PI) * 10;
      y = anchor.centerY - arc;
    }

    this.ballEl.style.transform = `translate(${x}px, ${y}px) translate(-50%, -100%)`;
    this.ballEl.style.opacity = "1";
    this.updateWordHighlights(activeLine, words, timeMs);
  }

  private updateWordHighlights(
    lineEl: HTMLElement,
    words: TimedWord[],
    timeMs: number,
  ) {
    const spans = lineEl.querySelectorAll<HTMLElement>(".lyric-word");
    const activeIdx = findActiveWordIndex(words, timeMs);
    spans.forEach((span, i) => {
      span.classList.remove("sung", "singing");
      if (i < activeIdx) span.classList.add("sung");
      else if (i === activeIdx) {
        span.classList.add("singing");
        const frac = wordProgress(words, timeMs);
        span.style.setProperty("--sung-frac", String(frac));
      } else {
        span.style.removeProperty("--sung-frac");
      }
    });
  }

  hide() {
    this.ballEl.style.opacity = "0";
  }

  /** Call after lyrics DOM is rebuilt so the ball node stays in the stage. */
  reattach() {
    if (!this.container.contains(this.ballEl)) {
      this.container.appendChild(this.ballEl);
    }
  }
}