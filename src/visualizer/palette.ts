export const VISUALIZER_PALETTE: [number, number, number][] = [
  [0, 245, 255],
  [255, 60, 200],
  [160, 90, 255],
  [255, 140, 50],
  [80, 255, 180],
];

export function bandColor(
  i: number,
  n: number,
  accent: [number, number, number],
  amp: number,
  alphaMul = 1,
): string {
  const t = i / Math.max(1, n - 1);
  const p0 = VISUALIZER_PALETTE[Math.floor(t * (VISUALIZER_PALETTE.length - 1))]!;
  const p1 =
    VISUALIZER_PALETTE[Math.min(VISUALIZER_PALETTE.length - 1, Math.ceil(t * (VISUALIZER_PALETTE.length - 1)))]!;
  const f = (t * (VISUALIZER_PALETTE.length - 1)) % 1;
  const mix = (a: number, b: number) => a + (b - a) * f;
  const r = Math.round(mix(p0[0], p1[0]) * 0.65 + accent[0] * 0.35);
  const g = Math.round(mix(p0[1], p1[1]) * 0.65 + accent[1] * 0.35);
  const b = Math.round(mix(p0[2], p1[2]) * 0.65 + accent[2] * 0.35);
  const a = (0.45 + amp * 0.55) * alphaMul;
  return `rgba(${r},${g},${b},${a})`;
}