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

const particles: { x: number; y: number; vx: number; vy: number; life: number; hue: number }[] =
  [];
let waveHistory: number[] = [];

const PALETTE: [number, number, number][] = [
  [0, 245, 255],
  [255, 60, 200],
  [160, 90, 255],
  [255, 140, 50],
  [80, 255, 180],
];

function bandColor(
  i: number,
  n: number,
  accent: [number, number, number],
  amp: number,
  alphaMul = 1,
): string {
  const t = i / Math.max(1, n - 1);
  const p0 = PALETTE[Math.floor(t * (PALETTE.length - 1))]!;
  const p1 = PALETTE[Math.min(PALETTE.length - 1, Math.ceil(t * (PALETTE.length - 1)))]!;
  const f = (t * (PALETTE.length - 1)) % 1;
  const mix = (a: number, b: number) => a + (b - a) * f;
  const r = Math.round(mix(p0[0], p1[0]) * 0.65 + accent[0] * 0.35);
  const g = Math.round(mix(p0[1], p1[1]) * 0.65 + accent[1] * 0.35);
  const b = Math.round(mix(p0[2], p1[2]) * 0.65 + accent[2] * 0.35);
  const a = (0.45 + amp * 0.55) * alphaMul;
  return `rgba(${r},${g},${b},${a})`;
}

export class VisualizerEngine {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private rafId = 0;
  private frame: VisualizerFrame | null = null;
  private opts: VisualizerOptions = {
    kind: "off",
    intensity: 1.5,
    opacity: 0.8,
    accentRgb: [255, 90, 110],
  };

  constructor(host: HTMLElement) {
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
    const cap = Math.min(Math.max(this.opts.opacity, 0), 1);
    this.canvas.style.opacity = String(cap);
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

  private viewportSize() {
    return {
      w: window.innerWidth,
      h: window.innerHeight,
    };
  }

  private resize() {
    const { w, h } = this.viewportSize();
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    this.canvas.width = Math.max(1, Math.floor(w * dpr));
    this.canvas.height = Math.max(1, Math.floor(h * dpr));
    this.canvas.style.width = `${w}px`;
    this.canvas.style.height = `${h}px`;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  private clear() {
    const { w, h } = this.viewportSize();
    this.ctx.clearRect(0, 0, w, h);
  }

  private draw() {
    const kind = this.opts.kind;
    if (kind === "off") return;
    const { w, h } = this.viewportSize();
    const f = this.frame;
    const intensity = this.opts.intensity;
    const level = (f?.level ?? 0) * intensity * 0.55;
    const accent = this.opts.accentRgb;
    this.ctx.clearRect(0, 0, w, h);
  if (!f) return;

    switch (kind) {
      case "spectrum":
        this.drawBars(w, h, f.bands, accent, intensity);
        break;
      case "waveform":
        this.drawWave(w, h, f.bands, accent, intensity);
        break;
      case "radial":
        this.drawRadial(w, h, f.bands, accent, intensity);
        break;
      case "particles":
        this.drawParticles(w, h, f, accent, intensity);
        break;
      case "aurora":
        this.drawAurora(w, h, level, f.bands, accent, intensity);
        break;
      default:
        break;
    }
  }

  private withGlow(drawFn: () => void, blur = 18) {
    this.ctx.save();
    this.ctx.globalCompositeOperation = "lighter";
    this.ctx.shadowBlur = blur;
    this.ctx.shadowColor = "rgba(180, 120, 255, 0.85)";
    drawFn();
    this.ctx.restore();
  }

  private drawBars(w: number, h: number, bands: number[], accent: [number, number, number], intensity: number) {
    const n = bands.length;
    const maxBarH = h * 0.48;
    const pad = 4;
    const bw = (w - pad * (n + 1)) / n;
    const baseGlow = () => {
      for (let i = 0; i < n; i++) {
        const v = Math.min(1, bands[i]! * intensity * 1.1);
        const bh = Math.max(2, v * maxBarH);
        const x = pad + i * (bw + pad);
        this.ctx.fillStyle = bandColor(i, n, accent, v);
        this.ctx.fillRect(x, h - bh - pad, bw, bh);
      }
    };
    this.withGlow(baseGlow, 22);
    this.ctx.globalCompositeOperation = "lighter";
    for (let i = 0; i < n; i++) {
      const v = Math.min(1, bands[i]! * intensity * 1.1);
      const bh = Math.max(2, v * maxBarH);
      const x = pad + i * (bw + pad);
      const grd = this.ctx.createLinearGradient(x, h, x, h - bh);
      grd.addColorStop(0, bandColor(i, n, accent, v * 0.4));
      grd.addColorStop(1, bandColor(i, n, accent, v));
      this.ctx.fillStyle = grd;
      this.ctx.fillRect(x, h - bh - pad, bw, bh);
    }
    this.ctx.globalCompositeOperation = "source-over";
  }

  private drawWave(w: number, h: number, bands: number[], accent: [number, number, number], intensity: number) {
    const mid = h * 0.58;
    const amp = h * 0.22 * intensity;
    const avg =
      bands.length > 0 ? bands.reduce((a, b) => a + b, 0) / bands.length : 0;
    waveHistory.push(avg);
    const maxHist = Math.max(128, Math.floor(w / 6));
    if (waveHistory.length > maxHist) waveHistory.shift();
    const stroke = () => {
      this.ctx.beginPath();
      this.ctx.lineWidth = 5 + intensity * 0.8;
      this.ctx.lineJoin = "round";
      this.ctx.lineCap = "round";
      for (let i = 0; i < waveHistory.length; i++) {
        const t = i / Math.max(1, waveHistory.length - 1);
        const x = t * w;
        const y = mid - waveHistory[i]! * amp;
        if (i === 0) this.ctx.moveTo(x, y);
        else this.ctx.lineTo(x, y);
      }
      const [r, g, b] = accent;
      this.ctx.strokeStyle = `rgba(${Math.min(255, r + 80)},${Math.min(255, g + 40)},${Math.min(255, b + 120)},0.92)`;
      this.ctx.stroke();
    };
    this.withGlow(stroke, 28);
    stroke();
  }

  private drawRadial(w: number, h: number, bands: number[], accent: [number, number, number], intensity: number) {
    const cx = w / 2;
    const cy = h / 2;
    const n = Math.min(bands.length, 48);
    const minSide = Math.min(w, h);
    const rInner = minSide * 0.32;
    const rOuter = minSide * 0.42;
    const draw = () => {
      for (let i = 0; i < n; i++) {
        const ang = (i / n) * Math.PI * 2 - Math.PI / 2;
        const v = Math.min(1, bands[i]! * intensity * 1.15);
        const r1 = rInner + v * rOuter;
        this.ctx.strokeStyle = bandColor(i, n, accent, v);
        this.ctx.lineWidth = 3 + v * 4;
        this.ctx.beginPath();
        this.ctx.moveTo(cx + Math.cos(ang) * rInner, cy + Math.sin(ang) * rInner);
        this.ctx.lineTo(cx + Math.cos(ang) * r1, cy + Math.sin(ang) * r1);
        this.ctx.stroke();
      }
    };
    this.withGlow(draw, 24);
    draw();
  }

  private drawParticles(
    w: number,
    h: number,
    f: VisualizerFrame,
    accent: [number, number, number],
    intensity: number,
  ) {
    if (f.beat) {
      const count = 10 + Math.floor(intensity * 4);
      for (let i = 0; i < count; i++) {
        particles.push({
          x: Math.random() * w,
          y: h * (0.55 + Math.random() * 0.4),
          vx: (Math.random() - 0.5) * 4 * intensity,
          vy: (-1.5 - Math.random() * 4) * intensity,
          life: 1,
          hue: Math.random(),
        });
      }
    }
    this.ctx.globalCompositeOperation = "lighter";
    for (let i = particles.length - 1; i >= 0; i--) {
      const p = particles[i]!;
      p.x += p.vx;
      p.y += p.vy;
      p.vy += 0.02;
      p.life -= 0.012;
      if (p.life <= 0 || p.y < -20) particles.splice(i, 1);
      else {
        const [r, g, b] = accent;
        const cr = Math.min(255, r + p.hue * 120);
        const cg = Math.min(255, g + (1 - p.hue) * 80);
        const cb = Math.min(255, b + 100);
        this.ctx.fillStyle = `rgba(${cr},${cg},${cb},${p.life * 0.85})`;
        this.ctx.shadowBlur = 16;
        this.ctx.shadowColor = `rgba(${cr},${cg},${cb},0.9)`;
        this.ctx.beginPath();
        this.ctx.arc(p.x, p.y, (4 + f.level * 10) * intensity * 0.35, 0, Math.PI * 2);
        this.ctx.fill();
      }
    }
    this.ctx.shadowBlur = 0;
    this.ctx.globalCompositeOperation = "source-over";
  }

  private drawAurora(
    w: number,
    h: number,
    level: number,
    bands: number[],
    accent: [number, number, number],
    intensity: number,
  ) {
    const t = Math.min(1, level * 1.2);
    const n = bands.length;
    this.ctx.globalCompositeOperation = "lighter";
    for (let layer = 0; layer < 3; layer++) {
      const g = this.ctx.createLinearGradient(0, 0, w, h);
      for (let i = 0; i < 5; i++) {
        const bi = Math.floor((i / 4) * (n - 1));
        const v = (bands[bi] ?? 0) * intensity;
        g.addColorStop(i / 4, bandColor(bi, n, accent, Math.min(1, v + t * 0.3), 0.12 + layer * 0.1));
      }
      this.ctx.fillStyle = g;
      this.ctx.fillRect(0, h * (0.1 + layer * 0.08), w, h * (0.95 - layer * 0.05));
    }
    this.ctx.globalCompositeOperation = "source-over";
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