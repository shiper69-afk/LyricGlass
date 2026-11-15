import { describe, expect, it } from "vitest";
import { formatWallpaperDiagnosticsLines } from "./wallpaper-status";

describe("wallpaper-status diagnostics", () => {
  it("shows placeholder when no attempts yet", () => {
    const lines = formatWallpaperDiagnosticsLines({
      attached: false,
      attemptCount: 0,
      logFilePath: "C:\\Users\\x\\AppData\\Local\\LyricGlass\\wallpaper.log",
    });
    expect(lines.some((l) => l.includes("no wallpaper attempts"))).toBe(true);
  });

  it("includes last attempt trace", () => {
    const lines = formatWallpaperDiagnosticsLines({
      attached: false,
      attemptCount: 2,
      lastTrigger: "hotkey",
      lastOutcome: "failure",
      lastError: "spawn: Progman not found",
      lastDiagnostics: "trigger: hotkey\noutcome: failure",
      logFilePath: "C:\\LyricGlass\\wallpaper.log",
    });
    expect(lines.join("\n")).toContain("lastTrigger: hotkey");
    expect(lines.join("\n")).toContain("attachTrace:");
    expect(lines.join("\n")).toContain("trigger: hotkey");
  });
});