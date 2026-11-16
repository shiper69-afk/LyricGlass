import type { VisualizerKind } from "./catalog";
import { combinedEnergy, JsBeatDetector } from "./beat-fallback";
import { bandColor } from "./palette";

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

type Particle = { x: number; y: number; vx: number; vy: number; life: number; hue: number };
type Star = { x: number; y: number; z: number };
type MatrixCol = { x: number; y: number; speed: number; band: number; chars: string };

const MAX_PARTICLES = 140;
const MAX_STARS = 90;

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

  private readonly beatDetector = new JsBeatDetector();
  private particles: Particle[] = [];
  private waveHistory: number[] = [];
  private terrainRows: number[][] = [];
  private stars: Star[] = [];
  private matrixCols: MatrixCol[] = [];
  private tunnelPhase = 0;

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
    return { w: window.innerWidth, h: window.innerHeight };
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

  private lowBandEnergy(f: VisualizerFrame): number {
    const slice = f.bands.slice(0, 8);
    if (!slice.length) return 0;
    return slice.reduce((a, b) => a + b, 0) / slice.length;
  }

  private isBeat(f: VisualizerFrame): boolean {
    const energy = combinedEnergy(f.level, f.bands.slice(0, 8));
    return f.beat || this.beatDetector.tick(energy);
  }

  private withGlow(drawFn: () => void, blur = 18) {
    this.ctx.save();
    this.ctx.globalCompositeOperation = "lighter";
    this.ctx.shadowBlur = blur;
    this.ctx.shadowColor = "rgba(180, 120, 255, 0.85)";
    drawFn();
    this.ctx.restore();
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
      case "starfield":
        this.drawStarfield(w, h, f, accent, intensity);
        break;
      case "mirror":
        this.drawMirror(w, h, f.bands, accent, intensity);
        break;
      case "tunnel":
        this.drawTunnel(w, h, f, accent, intensity);
        break;
      case "matrix":
        this.drawMatrix(w, h, f, accent, intensity);
        break;
      case "terrain":
        this.drawTerrain(w, h, f.bands, accent, intensity);
        break;
      default:
        break;
    }
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
    const avg = bands.length > 0 ? bands.reduce((a, b) => a + b, 0) / bands.length : 0;
    this.waveHistory.push(avg);
    const maxHist = Math.max(128, Math.floor(w / 6));
    if (this.waveHistory.length > maxHist) this.waveHistory.shift();
    const stroke = () => {
      this.ctx.beginPath();
      this.ctx.lineWidth = 5 + intensity * 0.8;
      this.ctx.lineJoin = "round";
      this.ctx.lineCap = "round";
      for (let i = 0; i < this.waveHistory.length; i++) {
        const t = i / Math.max(1, this.waveHistory.length - 1);
        const x = t * w;
        const y = mid - this.waveHistory[i]! * amp;
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

  private spawnParticle(w: number, h: number, intensity: number, burst: boolean) {
    const yBase = burst ? h * 0.92 : h * (0.5 + Math.random() * 0.45);
    this.particles.push({
      x: Math.random() * w,
      y: yBase,
      vx: (Math.random() - 0.5) * (burst ? 6 : 3) * intensity,
      vy: (-1.2 - Math.random() * (burst ? 5 : 2.5)) * intensity,
      life: burst ? 1.2 : 0.85 + Math.random() * 0.35,
      hue: Math.random(),
    });
  }

  private drawParticles(
    w: number,
    h: number,
    f: VisualizerFrame,
    accent: [number, number, number],
    intensity: number,
  ) {
    const beat = this.isBeat(f);
    const drive = Math.min(1, f.level * intensity * 1.1 + this.lowBandEnergy(f) * 0.5);
    const ambient = Math.min(6, Math.floor(drive * 8));
    for (let i = 0; i < ambient; i++) this.spawnParticle(w, h, intensity, false);
    if (beat) {
      const burst = 12 + Math.floor(intensity * 6);
      for (let i = 0; i < burst; i++) this.spawnParticle(w, h, intensity, true);
    }
    while (this.particles.length > MAX_PARTICLES) this.particles.shift();

    this.ctx.globalCompositeOperation = "lighter";
    for (let i = this.particles.length - 1; i >= 0; i--) {
      const p = this.particles[i]!;
      p.x += p.vx;
      p.y += p.vy;
      p.vy += 0.025;
      p.life -= 0.0055;
      if (p.life <= 0 || p.y < -40) this.particles.splice(i, 1);
      else {
        const [r, g, b] = accent;
        const cr = Math.min(255, r + p.hue * 140);
        const cg = Math.min(255, g + (1 - p.hue) * 90);
        const cb = Math.min(255, b + 110);
        const alpha = Math.min(1, p.life * 0.95);
        const rad = (3.5 + f.level * 12 + drive * 6) * Math.min(1.8, intensity * 0.45);
        this.ctx.save();
        this.ctx.shadowBlur = 14;
        this.ctx.shadowColor = `rgba(${cr},${cg},${cb},0.95)`;
        this.ctx.fillStyle = `rgba(${cr},${cg},${cb},${alpha})`;
        this.ctx.beginPath();
        this.ctx.arc(p.x, p.y, rad, 0, Math.PI * 2);
        this.ctx.fill();
        this.ctx.restore();
      }
    }
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

  private ensureStars() {
    if (this.stars.length >= MAX_STARS) return;
    while (this.stars.length < MAX_STARS) {
      this.stars.push({
        x: Math.random() * 2 - 1,
        y: Math.random() * 2 - 1,
        z: Math.random(),
      });
    }
  }

  private drawStarfield(w: number, h: number, f: VisualizerFrame, accent: [number, number, number], intensity: number) {
    this.ensureStars();
    const cx = w / 2;
    const cy = h / 2;
    const beat = this.isBeat(f);
    const speed = 0.012 + f.level * 0.04 * intensity + (beat ? 0.06 : 0);
    this.ctx.globalCompositeOperation = "lighter";
    for (const s of this.stars) {
      s.z -= speed;
      if (s.z <= 0.02) {
        s.x = Math.random() * 2 - 1;
        s.y = Math.random() * 2 - 1;
        s.z = 1;
      }
      const inv = 1 / s.z;
      const px = cx + s.x * inv * w * 0.45;
      const py = cy + s.y * inv * h * 0.45;
      const size = (1 - s.z) * 4 + f.level * 3;
      const idx = Math.floor((1 - s.z) * 48);
      this.ctx.fillStyle = bandColor(idx, 48, accent, 1 - s.z, 0.5 + (1 - s.z) * 0.5);
      this.ctx.beginPath();
      this.ctx.arc(px, py, size, 0, Math.PI * 2);
      this.ctx.fill();
    }
    this.ctx.globalCompositeOperation = "source-over";
  }

  private drawMirror(w: number, h: number, bands: number[], accent: [number, number, number], intensity: number) {
    const n = bands.length;
    const mid = h * 0.52;
    const maxBarH = h * 0.38;
    const pad = 4;
    const bw = (w - pad * (n + 1)) / n;
    this.ctx.globalCompositeOperation = "lighter";
    for (let i = 0; i < n; i++) {
      const v = Math.min(1, bands[i]! * intensity * 1.1);
      const bh = Math.max(2, v * maxBarH);
      const x = pad + i * (bw + pad);
      this.ctx.fillStyle = bandColor(i, n, accent, v);
      this.ctx.fillRect(x, mid - bh, bw, bh);
      this.ctx.fillRect(x, mid, bw, bh);
    }
    this.ctx.strokeStyle = "rgba(255,255,255,0.08)";
    this.ctx.lineWidth = 1;
    this.ctx.beginPath();
    this.ctx.moveTo(0, mid);
    this.ctx.lineTo(w, mid);
    this.ctx.stroke();
    this.ctx.globalCompositeOperation = "source-over";
  }

  private drawTunnel(w: number, h: number, f: VisualizerFrame, accent: [number, number, number], intensity: number) {
    const cx = w / 2;
    const cy = h / 2;
    const beat = this.isBeat(f);
    this.tunnelPhase += 0.02 + f.level * 0.03 * intensity;
    if (beat) this.tunnelPhase += 0.15;
    const maxR = Math.min(w, h) * 0.48;
    const rings = 10;
    const avg = f.bands.length
      ? f.bands.reduce((a, b) => a + b, 0) / f.bands.length
      : 0;
    this.ctx.globalCompositeOperation = "lighter";
    for (let r = 0; r < rings; r++) {
      const t = (r + (this.tunnelPhase % 1)) / rings;
      const radius = t * maxR;
      const v = Math.min(1, avg * intensity * (1 - t * 0.4));
      this.ctx.strokeStyle = bandColor(r, rings, accent, v, 0.35 + v * 0.5);
      this.ctx.lineWidth = 2 + v * 4;
      this.ctx.beginPath();
      this.ctx.arc(cx, cy, radius, 0, Math.PI * 2);
      this.ctx.stroke();
    }
    this.ctx.globalCompositeOperation = "source-over";
  }

  private ensureMatrix(w: number, bands: number[]) {
    const cols = Math.min(36, Math.max(18, Math.floor(w / 28)));
    if (this.matrixCols.length === cols) return;
    this.matrixCols = [];
    for (let i = 0; i < cols; i++) {
      const band = Math.floor((i / cols) * Math.max(1, bands.length - 1));
      this.matrixCols.push({
        x: (i + 0.5) * (w / cols),
        y: Math.random() * window.innerHeight,
        speed: 1.5 + Math.random() * 2,
        band,
        chars: "01",
      });
    }
  }

  private drawMatrix(w: number, h: number, f: VisualizerFrame, accent: [number, number, number], intensity: number) {
    this.ensureMatrix(w, f.bands);
    this.ctx.font = "12px monospace";
    this.ctx.globalCompositeOperation = "lighter";
    for (const col of this.matrixCols) {
      const v = Math.min(1, (f.bands[col.band] ?? 0) * intensity);
      col.speed = 1.2 + v * 5;
      col.y += col.speed;
      if (col.y > h + 20) col.y = -20;
      const n = 3 + Math.floor(v * 5);
      for (let j = 0; j < n; j++) {
        const yy = col.y - j * 14;
        if (yy < 0 || yy > h) continue;
        this.ctx.fillStyle = bandColor(col.band, f.bands.length, accent, v, 0.25 + v * 0.5);
        this.ctx.fillText(j === 0 ? "1" : "0", col.x, yy);
      }
    }
    this.ctx.globalCompositeOperation = "source-over";
  }

  private drawTerrain(w: number, h: number, bands: number[], accent: [number, number, number], intensity: number) {
    const row = bands.map((b) => Math.min(1, b * intensity));
    this.terrainRows.push(row);
    const maxRows = 48;
    if (this.terrainRows.length > maxRows) this.terrainRows.shift();
    const horizon = h * 0.45;
    const depth = this.terrainRows.length;
    this.ctx.globalCompositeOperation = "lighter";
    for (let d = 0; d < depth; d++) {
      const rowData = this.terrainRows[d]!;
      const t = (d + 1) / maxRows;
      const yBase = horizon + t * (h - horizon);
      const scale = 0.15 + t * 0.85;
      this.ctx.beginPath();
      this.ctx.moveTo(0, yBase);
      for (let i = 0; i < rowData.length; i++) {
        const x = (i / (rowData.length - 1)) * w;
        const peak = rowData[i]! * h * 0.18 * scale;
        this.ctx.lineTo(x, yBase - peak);
      }
      this.ctx.lineTo(w, yBase);
      this.ctx.closePath();
      this.ctx.fillStyle = bandColor(d, maxRows, accent, t, 0.12 + t * 0.35);
      this.ctx.fill();
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