export function smoothedPositionMs(
  lastReportedMs: number,
  playing: boolean,
  elapsedSinceReportMs: number,
): number {
  if (!playing) return lastReportedMs;
  return lastReportedMs + elapsedSinceReportMs;
}

export function formatMs(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}