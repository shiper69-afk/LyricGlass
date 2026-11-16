import { describe, expect, it } from "vitest";
import { buildDiagnosticsBlock } from "./diagnostics";
import type { LyricsView } from "./lyrics-view";
import type { WallpaperStatus } from "./wallpaper-status";

function mockLyricsView(): LyricsView {
  return {
    getSyncOffsetMs: () => 0,
    getBuiltinSyncLagMs: () => 600,
    getEffectiveSyncOffsetMs: () => 600,
    getLineDiagnostics: () => ({
      index: -1,
      lineTimeMs: 0,
      nextLineTimeMs: -1,
      computedMs: 0,
    }),
    getLastLyrics: () => null,
  } as unknown as LyricsView;
}

describe("buildDiagnosticsBlock wallpaper", () => {
  it("includes wallpaper attempt fields when status is provided", () => {
    const wallpaper: WallpaperStatus = {
      attached: false,
      attemptCount: 1,
      lastTrigger: "gear",
      lastOutcome: "failure",
      lastError: "find_workerw failed",
      lastDiagnostics: "progman_hwnd=0x100\n",
      logFilePath: "%LOCALAPPDATA%\\LyricGlass\\wallpaper.log",
    };
    const block = buildDiagnosticsBlock(null, null, mockLyricsView(), null, wallpaper);
    expect(block).toContain("Wallpaper");
    expect(block).toContain("lastTrigger: gear");
    expect(block).toContain("attachTrace:");
    expect(block).not.toContain("attached: —");
  });
});