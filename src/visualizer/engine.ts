import type { VisualizerKind } from "./catalog";

export interface VisualizerFrame {
  bands: number[];
  level: number;
  beat: boolean;
}

export interface VisualizerOptions {
  kind: VisualizerKind;
  intensity: number;
  opacity: number;
  accentRgb: [number, number, number];
}

let waveHistory: number[] = [];
const particles: { x: number; y: number; vx: number; vy: number; life: number }[] = [];

export class VisualizerEngine {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private rafId = 0;
  private frame: VisualizerFrame | null = null;
  private opts: VisualizerOptions = {
    kind: "off",
    intensity: 0.85,
    opacity: 0.35,
    accentRgb: [255, 90, 110],
  };

  constructor(private host: HTMLElement) {
    this.canvas = document.createElement("canvas");
    this.canvas.className = "visualizer-canvas";
    this.canvas.setAttribute("aria-hidden", "true");
    host.prepend(this.canvas);
    const ctx = this.canvas.getContext("2d", { alpha: true });
    if (!ctx) throw new Error("canvas 2d unavailable");
    this.ctx = ctx;
    this.resize();
    window.addEventListener("resize", () => this.resize());
  }

  setOptions(next: Partial<VisualizerOptions>) {
    this.opts = { ...this.opts, ...next };
    const cap = Math.min(this.opts.opacity, 0.55);
    this.host.style.setProperty("--viz-opacity", String(cap));
    if (this.opts.kind === "off") {
      this.stop();
      this.clear();
    } else if (!this.rafId) {
      this.start();
    }
  }

  pushFrame(payload: { b: number[]; l: number; bt: boolean }) {
    this.frame = {
      bands: payload.b.map((v) => v / 255),
      level: payload.l / 255,
      beat: payload.bt,
    };
  }

  start() {
    if (this.opts.kind === "off") return;
    const tick = () => {
      this.draw();
      this.rafId = requestAnimationFrame(tick);
    };
    this.rafId = requestAnimationFrame(tick);
  }

  stop() {
    if (this.rafId) cancelAnimationFrame(this.rafId);
    this.rafId = 0;
  }

  private resize() {
    const r = this.host.getBoundingClientRect();
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    this.canvas.width = Math.max(1, Math.floor(r.width * dpr));
    this.canvas.height = Math.max(1, Math.floor(r.height * dpr));
    this.canvas.style.width = `${r.width}px`;
    this.canvas.style.height = `${r.height}px`;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  private clear() {
    const w = this.host.clientWidth;
    const h = this.host.clientHeight;
    this.ctx.clearRect(0, 0, w, h);
  }

  private draw() {
    const kind = this.opts.kind;
    if (kind === "off") return;
    const w = this.host.clientWidth;
    const h = this.host.clientHeight;
    const f = this.frame;
  const level = (f?.level ?? 0) * this.opts.intensity;
    this.ctx.clearRect(0, 0, w, h);
    const [r, g, b] = this.opts.accentRgb;
    const baseA = Math.min(this.opts.opacity, 0.55);

    if (!f) return;

    switch (kind) {
      case "spectrum":
        this.drawBars(w, h, f.bands, r, g, b, baseA);
        break;
      case "waveform":
        this.drawWave(w, h, f.bands, r, g, b, baseA);
        break;
      case "radial":
        this.drawRadial(w, h, f.bands, r, g, b, baseA);
        break;
      case "particles":
        this.drawParticles(w, h, f, r, g, b, baseA);
        break;
      case "aurora":
        this.drawAurora(w, h, level, r, g, b, baseA);
        break;
      default:
        break;
    }
  }

  private drawBars(
    w: number,
    h: number,
    bands: number[],
    r: number,
    g: number,
    b: number,
    a: number,
  ) {
    const n = bands.length;
    const gap = 2;
    const bw = (w - gap * (n + 1)) / n;
    for (let i = 0; i < n; i++) {
      const v = bands[i] * this.opts.intensity;
      const bh = v * h * 0.55;
      const x = gap + i * (bw + gap);
      this.ctx.fillStyle = `rgba(${r},${g},${b},${a * (0.35 + v * 0.65)})`;
      this.ctx.fillRect(x, h - bh - 8, bw, bh);
    }
  }

  private drawWave(
    w: number,
    h: number,
    bands: number[],
    r: number,
    g: number,
    b: number,
    a: number,
  ) {
    const mid = h * 0.72;
    waveHistory.push(bands[0] ?? 0);
    if (waveHistory.length > 128) waveHistory.shift();
    this.ctx.beginPath();
    for (let i = 0; i < waveHistory.length; i++) {
      const t = i / Math.max(1, waveHistory.length - 1);
      const x = t * w;
      const y = mid - waveHistory[i] * h * 0.22 * this.opts.intensity;
      if (i === 0) this.ctx.moveTo(x, y);
      else this.ctx.lineTo(x, y);
    }
    this.ctx.strokeStyle = `rgba(${r},${g},${b},${a * 0.85})`;
    this.ctx.lineWidth = 2;
    this.ctx.stroke();
  }

  private drawRadial(
    w: number,
    h: number,
    bands: number[],
    r: number,
    g: number,
    b: number,
    a: number,
  ) {
    const cx = w / 2;
    const cy = h * 0.78;
    const n = Math.min(bands.length, 48);
    for (let i = 0; i < n; i++) {
      const ang = (i / n) * Math.PI * 2 - Math.PI / 2;
      const v = bands[i] * this.opts.intensity;
      const r0 = 24;
      const r1 = r0 + v * 90;
      this.ctx.strokeStyle = `rgba(${r},${g},${b},${a * (0.25 + v * 0.6)})`;
      this.ctx.lineWidth = 2;
      this.ctx.beginPath();
      this.ctx.moveTo(cx + Math.cos(ang) * r0, cy + Math.sin(ang) * r0);
      this.ctx.lineTo(cx + Math.cos(ang) * r1, cy + Math.sin(ang) * r1);
      this.ctx.stroke();
    }
  }

  private drawParticles(
    w: number,
    h: number,
    f: VisualizerFrame,
    r: number,
    g: number,
    b: number,
    a: number,
  ) {
    if (f.beat) {
      for (let i = 0; i < 6; i++) {
        particles.push({
          x: w * (0.2 + Math.random() * 0.6),
          y: h * 0.75,
          vx: (Math.random() - 0.5) * 2.5,
          vy: -2 - Math.random() * 3,
          life: 1,
        });
      }
    }
    for (let i = particles.length - 1; i >= 0; i--) {
      const p = particles[i];
      p.x += p.vx;
      p.y += p.vy;
      p.vy += 0.04;
      p.life -= 0.02;
      if (p.life <= 0) particles.splice(i, 1);
      else {
        this.ctx.fillStyle = `rgba(${r},${g},${b},${a * p.life * 0.7})`;
        this.ctx.beginPath();
        this.ctx.arc(p.x, p.y, 3 + f.level * 4, 0, Math.PI * 2);
        this.ctx.fill();
      }
    }
  }

  private drawAurora(
    w: number,
    h: number,
    level: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) {
    const g2 = this.ctx.createLinearGradient(0, h * 0.5, 0, h);
    g2.addColorStop(0, `rgba(${r},${g},${b},0)`);
    g2.addColorStop(0.5, `rgba(${r},${g},${b},${a * level * 0.35})`);
    g2.addColorStop(1, `rgba(${r},${g},${b},${a * level * 0.55})`);
    this.ctx.fillStyle = g2;
    this.ctx.fillRect(0, h * 0.45, w, h * 0.55);
  }
}

export function accentFromTheme(ballColor: string): [number, number, number] {
  const hex = ballColor.replace("#", "");
  if (hex.length < 6) return [255, 90, 110];
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  return [r, g, b];
}