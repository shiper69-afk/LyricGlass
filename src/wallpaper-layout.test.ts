import { describe, expect, it } from "vitest";
import {
  contextLineCount,
  estimateLineBlockPx,
  lineWindowForActive,
  reference1080pContextLines,
  wallpaperPanelHeight,
  wallpaperSafeBottomPx,
} from "./wallpaper-layout";

describe("wallpaper-layout", () => {
  it("uses a safe bottom margin of at least 60px", () => {
    expect(wallpaperSafeBottomPx(1080)).toBeGreaterThanOrEqual(60);
    expect(wallpaperPanelHeight(1080)).toBeLessThan(1080 - 60);
  });

  it("shows more than 3 lines at 1080p reference", () => {
    expect(reference1080pContextLines()).toBeGreaterThan(6);
  });

  it("centers the active line in the visible window", () => {
    const visible = 15;
    const w = lineWindowForActive(20, 80, visible);
    expect(w.totalVisible).toBeGreaterThan(3);
    expect(w.before).toBeGreaterThanOrEqual(6);
    expect(w.after).toBeGreaterThanOrEqual(6);
    expect(w.start).toBe(20 - w.before);
    expect(w.end).toBe(20 + w.after);
  });

  it("contextLineCount grows with panel height", () => {
    const block = estimateLineBlockPx(20, true);
    const short = contextLineCount(240, block);
    const tall = contextLineCount(900, block);
    expect(tall).toBeGreaterThan(short);
    expect(tall).toBeGreaterThan(10);
  });
});