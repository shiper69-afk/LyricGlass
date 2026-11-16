import { invoke } from "@tauri-apps/api/core";

export interface WallpaperStatus {
  attached: boolean;
  attemptCount: number;
  lastTimestampUnixMs?: number | null;
  lastTrigger?: string | null;
  lastOutcome?: string | null;
  lastError?: string | null;
  lastDiagnostics?: string | null;
  logFilePath: string;
}

export function wallpaperStatusForDiagnostics(
  status: WallpaperStatus | null | undefined,
): WallpaperStatus | null {
  return status ?? null;
}

export async function fetchWallpaperStatus(): Promise<WallpaperStatus> {
  return invoke<WallpaperStatus>("get_wallpaper_status");
}

export function formatWallpaperDiagnosticsLines(
  status: WallpaperStatus | null | undefined,
): string[] {
  if (!status) {
    return ["  (status unavailable)"];
  }
  if (status.attemptCount === 0) {
    return [
      `  attached: ${status.attached}`,
      "  (no wallpaper attempts recorded yet)",
      `  logFile: ${status.logFilePath}`,
    ];
  }
  const lines = [
    `  attached: ${status.attached}`,
    `  attemptCount: ${status.attemptCount}`,
    `  lastTimestampUnixMs: ${status.lastTimestampUnixMs ?? "—"}`,
    `  lastTrigger: ${status.lastTrigger ?? "—"}`,
    `  lastOutcome: ${status.lastOutcome ?? "—"}`,
    `  lastError: ${status.lastError?.length ? status.lastError : "—"}`,
    `  logFile: ${status.logFilePath}`,
  ];
  if (status.lastDiagnostics?.trim()) {
    lines.push("  attachTrace:");
    for (const row of status.lastDiagnostics.trim().split("\n")) {
      lines.push(`    ${row}`);
    }
  }
  return lines;
}