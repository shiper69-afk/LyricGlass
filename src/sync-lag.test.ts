import { describe, expect, it } from "vitest";
import { BUILTIN_SYNC_LAG_MS, effectiveSyncOffsetMs } from "./sync-lag";

describe("sync-lag", () => {
  it("adds built-in compensation to user slider offset", () => {
    expect(BUILTIN_SYNC_LAG_MS).toBe(600);
    expect(effectiveSyncOffsetMs(0)).toBe(600);
    expect(effectiveSyncOffsetMs(100)).toBe(700);
    expect(effectiveSyncOffsetMs(-200)).toBe(400);
  });
});