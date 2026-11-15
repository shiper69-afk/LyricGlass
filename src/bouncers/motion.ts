import type { BouncerId } from "./ids";

export interface SquashFrame {
  /** Local px offset (up negative) */
  ty: number;
  sx: number;
  sy: number;
}

/** Demo hop keyposes mapped to word progress 0..1. */
export function squashStretchPhase(progress: number): SquashFrame {
  const p = Math.max(0, Math.min(1, progress));
  if (p < 0.42) {
    const t = p / 0.42;
    return {
      ty: 6 + t * -19,
      sx: 1.26 + t * -0.36,
      sy: 0.7 + t * 0.44,
    };
  }
  if (p < 0.82) {
    const t = (p - 0.42) / 0.4;
    return {
      ty: -13 + t * 13,
      sx: 0.9 + t * 0.32,
      sy: 1.14 + t * -0.4,
    };
  }
  const t = (p - 0.82) / 0.18;
  return {
    ty: 0,
    sx: 1.22 + t * -0.22,
    sy: 0.74 + t * 0.26,
  };
}

export interface BouncerMotion {
  /** CSS transform on sprite (origin bottom center). */
  localTransform: string;
  /** Extra transform on host (shadow, etc.). */
  hostTransform?: string;
  glowOpacity?: number;
  shadowOpacity?: number;
  /** Word-start flash. */
  flash?: boolean;
}

export interface MotionContext {
  id: BouncerId;
  hopProgress: number;
  rtl: boolean;
  heldNote: boolean;
  wordIndex: number;
  interWordBlend: number;
}

function smooth(p: number) {
  return p * p * (3 - 2 * p);
}

export function computeBouncerMotion(ctx: MotionContext): BouncerMotion {
  const hopP = ctx.id === "pixel" ? Math.floor(ctx.hopProgress * 4) / 4 : ctx.hopProgress;
  const base = squashStretchPhase(hopP);
  const apex = Math.sin(ctx.hopProgress * Math.PI);
  let rot = 0;
  let extraSy = 1;
  let extraSx = 1;
  let ty = base.ty;
  let glowOpacity = 0.16;
  let shadowOpacity = 0;
  let flash = ctx.hopProgress < 0.06;

  switch (ctx.id) {
    case "glow-orb":
      glowOpacity = 0.14 + apex * 0.14;
      shadowOpacity = (1 - apex) * 0.35;
      break;
    case "music-note":
      rot = apex * -18 + (1 - apex) * 8;
      extraSy = 1 - apex * 0.06;
      break;
    case "spark":
      rot = ctx.wordIndex * 24 + ctx.hopProgress * 28;
      if (flash) {
        extraSx *= 1.15;
        extraSy *= 1.15;
      }
      extraSy *= 1 - apex * 0.08;
      break;
    case "heart":
      rot = apex * -12 + (1 - apex) * 6;
      if (ctx.heldNote && ctx.hopProgress > 0.55) {
        const pulse = Math.sin(ctx.hopProgress * Math.PI * 4);
        extraSx *= 1 + pulse * 0.06;
        extraSy *= 1 + pulse * 0.06;
      }
      break;
    case "glass-drop":
      extraSy = 1 + apex * 0.22;
      if (ctx.hopProgress > 0.75) {
        const w = Math.sin((ctx.hopProgress - 0.75) * Math.PI / 0.25) * 4;
        rot = w;
      }
      break;
    case "firefly":
      ty *= 0.55;
      base.sy = 0.92 + apex * 0.08;
      glowOpacity = 0.18 + Math.sin(ctx.hopProgress * Math.PI * 2) * 0.08;
      rot = Math.sin(ctx.hopProgress * Math.PI * 6) * 3;
      break;
    case "comet":
      glowOpacity = 0.12 + apex * 0.12;
      shadowOpacity = apex * 0.2;
      break;
    case "eq-pulse":
      if (ctx.heldNote) ty *= 0.25;
      break;
    case "mic":
      rot = Math.sin(ctx.hopProgress * Math.PI) * -16;
      if (flash) glowOpacity = 0.35;
      break;
    case "paper-plane":
      rot = -14 + apex * 10 + ctx.interWordBlend * -8;
      extraSx = 1 + ctx.interWordBlend * 0.06;
      break;
    case "pixel":
      flash = ctx.hopProgress < 0.02;
      break;
    case "crescent":
      rot = apex * -16 + (1 - apex) * 10;
      glowOpacity = 0.14 + Math.sin(ctx.hopProgress * Math.PI * 1.5) * 0.06;
      break;
    default:
      break;
  }

  const sx = base.sx * extraSx;
  const sy = base.sy * extraSy;
  const parts: string[] = [];
  if (ty !== 0) parts.push(`translateY(${ty.toFixed(2)}px)`);
  parts.push(`scale(${sx.toFixed(3)}, ${sy.toFixed(3)})`);
  if (rot !== 0) parts.push(`rotate(${rot.toFixed(2)}deg)`);

  let rtlFlip = "";
  if (ctx.rtl && (ctx.id === "comet" || ctx.id === "paper-plane")) {
    rtlFlip = "scaleX(-1)";
  }

  return {
    localTransform: parts.join(" "),
    hostTransform: rtlFlip || undefined,
    glowOpacity,
    shadowOpacity,
    flash,
  };
}

/** Per-bar EQ animation (transform-only). */
export function eqBarScales(hopProgress: number, heldNote: boolean): [number, number, number] {
  if (heldNote) {
    const t = hopProgress * Math.PI * 2;
    return [
      0.78 + Math.sin(t) * 0.22,
      0.85 + Math.sin(t + 0.9) * 0.28,
      0.7 + Math.sin(t + 1.7) * 0.18,
    ];
  }
  const p = smooth(hopProgress);
  return [
    0.75 + p * 0.35,
    0.9 + Math.sin(p * Math.PI) * 0.35,
    0.65 + p * 0.3,
  ];
}

export function cometTailOpacity(hopProgress: number): [number, number, number] {
  const tuck = 1 - Math.sin(hopProgress * Math.PI);
  return [0.3 + tuck * 0.6, 0.45 + tuck * 0.5, 0.2 + tuck * 0.65];
}