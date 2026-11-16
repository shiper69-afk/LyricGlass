import { convertFileSrc } from "@tauri-apps/api/core";
import { applyWallpaperLayout } from "./wallpaper-ui";
import type { LyricsResponse } from "./lyrics-view";
import type { FullMediaState } from "./diagnostics";
export interface DesktopWallpaperPayload {
  path: string;
  dataUrl?: string | null;
  desktopColor: string;
  readError?: string | null;
}

export interface WallpaperSurfaceAppearance {
  overlayOpacity: number;
  karaokeBall: boolean;
  karaokeBallColor: string;
  sungHighlightColor: string;
  karaokeBouncer: string;
}

export interface WallpaperSurfaceState {
  media: FullMediaState;
  appearance: WallpaperSurfaceAppearance;
  syncOffsetMs: number;
  lyrics: LyricsResponse | null;
}

/** Full-screen desktop lyrics chrome (no title bar). */
export function applyWallpaperSurfaceChrome(root: HTMLElement) {
  applyWallpaperLayout(root, true);
  root.querySelector(".titlebar")?.classList.add("hidden");
  root.querySelector(".hints")?.classList.add("hidden");
  root.querySelector(".debug-panel")?.classList.add("hidden");
  document.body.classList.add("wallpaper-surface");
  root.classList.add("wallpaper-surface");
}

export function applyDesktopWallpaperCss(payload: DesktopWallpaperPayload) {
  const root = document.documentElement;
  if (payload.dataUrl) {
    root.style.setProperty(
      "--desktop-wallpaper-url",
      `url("${payload.dataUrl.replace(/"/g, "%22")}")`,
    );
  } else if (payload.path) {
    const src = convertFileSrc(payload.path);
    root.style.setProperty(
      "--desktop-wallpaper-url",
      `url("${src.replace(/"/g, "%22")}")`,
    );
  } else {
    root.style.setProperty(
      "--desktop-wallpaper-url",
      `linear-gradient(${payload.desktopColor}, ${payload.desktopColor})`,
    );
  }
  root.style.setProperty("--desktop-wallpaper-fallback", payload.desktopColor);
}

export function lyricsTextFromResponse(res: LyricsResponse): string {
  if (res.plainLyrics) {
    return res.plainLyrics;
  }
  if (res.syncedLrc) {
  return res.syncedLrc;
  }
  return res.message ?? "";
}