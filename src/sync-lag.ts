/** Built-in GSMTC lag compensation (v0.6.1). Slider 0 = this applied. */
export const BUILTIN_SYNC_LAG_MS = 600;

export function effectiveSyncOffsetMs(userOffsetMs: number): number {
  return userOffsetMs + BUILTIN_SYNC_LAG_MS;
}