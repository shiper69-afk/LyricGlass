/** Wallpaper-mode viewport: line count, typography, safe area (frontend only). */

const LINE_GAP_PX = 8;
const KARAOKE_RESERVE_PX = 30;
const HEADER_RESERVE_PX = 56;
const MIN_SAFE_BOTTOM_PX = 60;

export interface LineWindow {
  start: number;
  end: number;
  before: number;
  after: number;
  totalVisible: number;
}

export function wallpaperSafeBottomPx(viewportHeight = window.innerHeight): number {
  return Math.max(MIN_SAFE_BOTTOM_PX, Math.round(viewportHeight * 0.055));
}

export function wallpaperPanelHeight(
  viewportHeight = window.innerHeight,
  safeBottom?: number,
): number {
  const bottom = safeBottom ?? wallpaperSafeBottomPx(viewportHeight);
  return Math.max(320, viewportHeight - bottom - HEADER_RESERVE_PX);
}

/** Typical line block height (font × line-height + gap + optional karaoke pad). */
export function estimateLineBlockPx(
  fontSizePx: number,
  karaokePad = false,
): number {
  const line = fontSizePx * 1.45;
  return Math.round(line + LINE_GAP_PX + (karaokePad ? KARAOKE_RESERVE_PX : 0));
}

/**
 * How many lyric lines fit in the lyrics viewport (must be > 3 at 1080p).
 */
export function contextLineCount(
  panelHeightPx: number,
  lineBlockPx: number,
): number {
  if (lineBlockPx <= 0) return 7;
  const usable = Math.max(200, panelHeightPx - KARAOKE_RESERVE_PX);
  return Math.max(7, Math.floor(usable / lineBlockPx));
}

export function lineWindowForActive(
  active: number,
  lineCount: number,
  visibleLines: number,
): LineWindow {
  if (lineCount <= 0) {
    return { start: 0, end: 0, before: 0, after: 0, totalVisible: 0 };
  }
  if (active < 0) {
    const end = Math.min(lineCount - 1, visibleLines - 1);
    return {
      start: 0,
      end,
      before: 0,
      after: end,
      totalVisible: end + 1,
    };
  }
  const half = Math.floor((visibleLines - 1) / 2);
  const before = Math.min(half, active);
  const after = Math.min(visibleLines - 1 - before, lineCount - 1 - active);
  const start = Math.max(0, active - before);
  const end = Math.min(lineCount - 1, active + after);
  return {
    start,
    end,
    before: active - start,
    after: end - active,
    totalVisible: end - start + 1,
  };
}

/** Reference 1920×1080 @ ~120% DPI (devicePixelRatio 1.25). */
export function reference1080pContextLines(): number {
  const panelH = wallpaperPanelHeight(1080, 72);
  const fontPx = 22;
  const block = estimateLineBlockPx(fontPx, true);
  return contextLineCount(panelH, block);
}

export function wallpaperTypographyVars(
  viewportWidth = window.innerWidth,
  viewportHeight = window.innerHeight,
  dpr = window.devicePixelRatio || 1,
): Record<string, string> {
  const vmin = Math.min(viewportWidth, viewportHeight);
  const scale = Math.min(1.35, Math.max(0.95, vmin / 960 / Math.sqrt(dpr)));
  const titlePx = Math.round(15 * scale);
  const artistPx = Math.round(13 * scale);
  const linePx = Math.round(18 * scale);
  const safeBottom = wallpaperSafeBottomPx(viewportHeight);
  return {
    "--wallpaper-safe-bottom": `${safeBottom}px`,
    "--wallpaper-title-size": `${titlePx}px`,
    "--wallpaper-artist-size": `${artistPx}px`,
    "--wallpaper-line-size": `${linePx}px`,
    "--wallpaper-active-scale": (1.12 + 0.03 * scale).toFixed(2),
  };
}

export function applyWallpaperLayoutVars(root: HTMLElement) {
  const vars = wallpaperTypographyVars();
  for (const [k, v] of Object.entries(vars)) {
    root.style.setProperty(k, v);
  }
}

export function installWallpaperLayoutVars(root: HTMLElement): () => void {
  const update = () => applyWallpaperLayoutVars(root);
  update();
  window.addEventListener("resize", update);
  return () => window.removeEventListener("resize", update);
}

/** Opacity hint from distance to active line (wallpaper fade). */
export function lineFadeOpacity(distanceFromActive: number): number {
  if (distanceFromActive <= 0) return 1;
  if (distanceFromActive === 1) return 0.78;
  if (distanceFromActive === 2) return 0.62;
  if (distanceFromActive === 3) return 0.5;
  return Math.max(0.28, 0.5 - (distanceFromActive - 3) * 0.08);
}