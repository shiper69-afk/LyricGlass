/** Built-in GSMTC lag compensation (v0.6.1). Slider 0 = this applied. */
export const DEFAULT_BUILTIN_SYNC_LAG_MS = 600;

let builtinLagOverride: number | null = null;

/** E2e harnesses call this with `0` so simulated clocks stay unshifted. */
export function setBuiltinSyncLagOverride(ms: number | null): void {
  builtinLagOverride = ms;
}

export function getBuiltinSyncLagMs(): number {
  return builtinLagOverride ?? DEFAULT_BUILTIN_SYNC_LAG_MS;
}

export function effectiveSyncOffsetMs(userOffsetMs: number): number {
  return userOffsetMs + getBuiltinSyncLagMs();
}