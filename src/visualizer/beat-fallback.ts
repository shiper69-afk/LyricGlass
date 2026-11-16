/** Energy-vs-average beat detector for when Rust `bt` is sparse (~33Hz, conservative thresholds). */
export class JsBeatDetector {
  private readonly window: number[];
  private cooldown = 0;

  constructor(
    private maxHistory = 28,
    private readonly beatRatio = 1.32,
    private readonly floor = 0.07,
    private readonly cooldownFrames = 14,
  ) {
    this.window = [];
  }

  reset() {
    this.window.length = 0;
    this.cooldown = 0;
  }

  /** @param energy 0..1 combined level + bass energy */
  tick(energy: number): boolean {
    const e = Math.max(0, Math.min(1, energy));
    this.window.push(e);
    if (this.window.length > this.maxHistory) this.window.shift();
    if (this.cooldown > 0) {
      this.cooldown -= 1;
      return false;
    }
    if (this.window.length < 6) return false;
    const avg =
      this.window.reduce((a, b) => a + b, 0) / Math.max(1, this.window.length);
    if (e > avg * this.beatRatio + 0.06 && e > this.floor) {
      this.cooldown = this.cooldownFrames;
      return true;
    }
    return false;
  }
}

export function combinedEnergy(level: number, lowBands: number[]): number {
  const low =
    lowBands.length > 0
      ? lowBands.reduce((a, b) => a + b, 0) / lowBands.length
      : 0;
  return level * 0.55 + low * 0.45;
}