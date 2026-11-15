/** Gentle scroll progress for untimed (plain) lyrics. */

export function plainScrollEase(t: number): number {
  const x = Math.max(0, Math.min(1, t));
  return x * x * (3 - 2 * x);
}

export function plainScrollOffsetPx(
  playbackMs: number,
  durationMs: number,
  contentHeight: number,
  viewportHeight: number,
): number {
  if (durationMs <= 0 || contentHeight <= viewportHeight) return 0;
  const p = plainScrollEase(playbackMs / durationMs);
  const maxScroll = contentHeight - viewportHeight;
  return maxScroll * p;
}

export function textLooksHebrew(text: string): boolean {
  return /[\u0590-\u05FF]/.test(text);
}

export function plainSourceLabel(source: string | undefined): string {
  switch (source) {
    case "shironet":
      return "Shironet";
    case "lrclib-plain":
      return "LRCLIB";
    case "lrclib-synced":
      return "LRCLIB (synced)";
    default:
      return "Untimed";
  }
}