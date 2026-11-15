import { describe, expect, it } from "vitest";
import {
  DEFAULT_BUILTIN_SYNC_LAG_MS,
  effectiveSyncOffsetMs,
  setBuiltinSyncLagOverride,
} from "./sync-lag";

describe("sync-lag", () => {
  afterEach(() => setBuiltinSyncLagOverride(null));

  it("adds built-in compensation to user slider offset", () => {
    expect(DEFAULT_BUILTIN_SYNC_LAG_MS).toBe(600);
    expect(effectiveSyncOffsetMs(0)).toBe(600);
    expect(effectiveSyncOffsetMs(100)).toBe(700);
    expect(effectiveSyncOffsetMs(-200)).toBe(400);
  });

  it("allows harness override to zero", () => {
    setBuiltinSyncLagOverride(0);
    expect(effectiveSyncOffsetMs(0)).toBe(0);
  });
});