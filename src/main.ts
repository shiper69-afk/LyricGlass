import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { formatMs, timelineAnchorAgeMs } from "./clock";
import {
  createPlaybackAnchor,
  playbackClockMs,
  type PlaybackAnchor,
} from "./playback-clock";
import {
  buildDiagnosticsBlock,
  buildSyncMarkBlock,
  type FullMediaState,
} from "./diagnostics";
import {
  LyricsView,
  type LyricsResponse,
} from "./lyrics-view";
import { plainSourceLabel } from "./plain-scroll";
import { applyWallpaperLayout } from "./wallpaper-ui";
import {
  applyDesktopWallpaperCss,
  applyWallpaperSurfaceChrome,
  type DesktopWallpaperPayload,
  type WallpaperSurfaceState,
} from "./wallpaper-surface";
import { installWallpaperLayoutVars } from "./wallpaper-layout";
import {
  fetchWallpaperStatus,
  type WallpaperStatus,
} from "./wallpaper-status";
import { BOUNCER_META, normalizeBouncerId } from "./bouncers/ids";
import { bouncerSvgMarkup } from "./bouncers/svg-store";
import "./fonts/faces";
import { applyLyricsFont, setLyricsFontChangeHook } from "./fonts/apply";
import { buildFontPicker, setFontPickerSelection } from "./font-picker";

interface UiState {
  alwaysOnTop: boolean;
  wallpaperMode: boolean;
  debugVisible: boolean;
  wallpaperError?: string | null;
}

interface AppearanceState {
  overlayOpacity: number;
  karaokeBall: boolean;
  karaokeBallColor: string;
  sungHighlightColor: string;
  karaokeBouncer: string;
  lyricsFont: string;
}

const DEFAULT_BALL_COLOR = "#ffc857";
const DEFAULT_SUNG_COLOR = "#ffdc8c";

let wallpaperSurface =
  new URLSearchParams(window.location.search).get("mode") === "wallpaper";

const lyricsRoot = document.getElementById("lyrics-root")!;
const lyricsView = new LyricsView(lyricsRoot);
setLyricsFontChangeHook(() => lyricsView.onLyricsFontChanged());
const appRoot = document.getElementById("app")!;

/** E2e / `?mode=wallpaper` harness: chrome before async Tauri boot. */
function primeWallpaperSurfaceChrome() {
  applyWallpaperSurfaceChrome(appRoot);
  applyWallpaperLayout(appRoot, true);
  installWallpaperLayoutVars(appRoot);
  lyricsView.setWallpaperLayout(true);
}

if (wallpaperSurface) {
  primeWallpaperSurfaceChrome();
}

const els = {
  mediaStatus: document.getElementById("media-status")!,
  album: document.getElementById("m-album")!,
  status: document.getElementById("m-status")!,
  posRaw: document.getElementById("m-position-raw")!,
  lutTicks: document.getElementById("m-lut-ticks")!,
  posAge: document.getElementById("m-position-age")!,
  wallAge: document.getElementById("m-wall-age")!,
  posSmooth: document.getElementById("m-position-smooth")!,
  playbackRate: document.getElementById("m-playback-rate")!,
  lineIndex: document.getElementById("m-line-index")!,
  timelineCount: document.getElementById("m-timeline-count")!,
  lastEvent: document.getElementById("m-last-event")!,
  btnAlwaysOnTop: document.getElementById("btn-always-on-top") as HTMLButtonElement,
  btnWallpaper: document.getElementById("btn-wallpaper") as HTMLButtonElement,
  btnSettings: document.getElementById("btn-settings") as HTMLButtonElement,
  settingsPopover: document.getElementById("settings-popover") as HTMLDivElement,
  titlebarTools: document.querySelector(".titlebar-tools")!,
  debugPanel: document.getElementById("debug-panel")!,
  plainBadge: document.getElementById("plain-badge")!,
  btnCopyDiag: document.getElementById("btn-copy-diag") as HTMLButtonElement,
  opacitySlider: document.getElementById("opacity-slider") as HTMLInputElement,
  timingSlider: document.getElementById("timing-slider") as HTMLInputElement,
  timingReset: document.getElementById("timing-reset") as HTMLButtonElement,
  timingReadout: document.getElementById("timing-readout")!,
  karaokeToggle: document.getElementById("karaoke-ball-toggle") as HTMLInputElement,
  wallpaperToggle: document.getElementById("wallpaper-toggle") as HTMLInputElement,
  ballColor: document.getElementById("ball-color") as HTMLInputElement,
  sungColor: document.getElementById("sung-color") as HTMLInputElement,
  colorsReset: document.getElementById("colors-reset") as HTMLButtonElement,
  btnOpenLogFolder: document.getElementById("btn-open-log-folder") as HTMLButtonElement,
  bouncerPicker: document.getElementById("bouncer-picker") as HTMLDivElement,
  fontPicker: document.getElementById("font-picker") as HTMLDivElement,
};

let latestMedia: FullMediaState | null = null;
let playbackAnchor: PlaybackAnchor | null = null;
let latestLyrics: LyricsResponse | null = null;
let fetchGeneration = 0;
let debugVisible = false;
let timingPersistTimer = 0;
let lastDebugClockRenderMs = 0;
let lastWallpaperStatus: WallpaperStatus | null = null;
const DEBUG_PANEL_HZ_MS = 200;

function applyAppearance(backgroundOpacity: number) {
  const bg = Math.max(0, Math.min(1, backgroundOpacity));
  appRoot.style.setProperty("--bg-alpha", String(bg));
  els.opacitySlider.value = String(Math.round(bg * 100));
}

function applyKaraokeColors(ball: string, sung: string) {
  const ballHex = ball || DEFAULT_BALL_COLOR;
  const sungHex = sung || DEFAULT_SUNG_COLOR;
  els.ballColor.value = ballHex;
  els.sungColor.value = sungHex;
  lyricsView.setKaraokeColors(ballHex, sungHex);
}

function applyBouncerSelection(id: string) {
  const norm = normalizeBouncerId(id);
  els.bouncerPicker
    .querySelectorAll<HTMLButtonElement>(".bouncer-picker-btn")
    .forEach((btn) => {
      const on = btn.dataset.bouncer === norm;
      btn.setAttribute("aria-pressed", String(on));
      btn.setAttribute("aria-selected", String(on));
    });
  lyricsView.setKaraokeBouncer(norm);
}

function buildBouncerPicker() {
  els.bouncerPicker.replaceChildren();
  for (const meta of BOUNCER_META) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "bouncer-picker-btn";
    btn.dataset.bouncer = meta.id;
    btn.setAttribute("role", "option");
    btn.setAttribute("aria-pressed", "false");
    btn.title = meta.name;
    const thumb = document.createElement("span");
    thumb.className = "bouncer-thumb";
    thumb.innerHTML = bouncerSvgMarkup(meta.id);
    const label = document.createElement("span");
    label.textContent = meta.name.split(" ")[0];
    btn.append(thumb, label);
    btn.addEventListener("click", () => {
      applyBouncerSelection(meta.id);
      void invoke("set_karaoke_bouncer", { bouncerId: meta.id });
    });
    els.bouncerPicker.appendChild(btn);
  }
}

async function applyLyricsFontSelection(fontId: string) {
  const id = await applyLyricsFont(fontId);
  setFontPickerSelection(els.fontPicker, id);
  return id;
}

function buildFontPickerUi() {
  buildFontPicker(els.fontPicker, (id) => {
    void (async () => {
      await applyLyricsFontSelection(id);
      try {
        await invoke("set_lyrics_font", { fontId: id });
      } catch {
        /* harness */
      }
    })();
  });
}

function applyAppearanceSettings(state: AppearanceState) {
  applyAppearance(state.overlayOpacity);
  els.karaokeToggle.checked = state.karaokeBall;
  lyricsView.setKaraokeEnabled(state.karaokeBall);
  applyKaraokeColors(state.karaokeBallColor, state.sungHighlightColor);
  applyBouncerSelection(state.karaokeBouncer ?? "glow-orb");
  void applyLyricsFontSelection(state.lyricsFont ?? "default");
}

function updateTimingReadout(ms: number) {
  els.timingReadout.textContent = `${ms >= 0 ? "+" : ""}${(ms / 1000).toFixed(1)}s`;
  els.timingSlider.value = String(ms);
}

async function persistSyncOffset(ms: number) {
  if (!latestMedia?.title && !latestMedia?.artist) return;
  try {
    const total = await invoke<number>("set_sync_offset", {
      artist: latestMedia.artist,
      track: latestMedia.title,
      album: latestMedia.album,
      durationMs: latestMedia.durationMs,
      offsetMs: ms,
    });
    lyricsView.setSyncOffsetAbsolute(total);
    updateTimingReadout(total);
  } catch {
    /* ignore */
  }
}

function schedulePersistTiming() {
  clearTimeout(timingPersistTimer);
  timingPersistTimer = window.setTimeout(() => {
    void persistSyncOffset(Number(els.timingSlider.value));
  }, 200);
}

function renderDebugStatic(media: FullMediaState) {
  els.mediaStatus.textContent = media.message;
  els.album.textContent = media.album || "—";
  els.status.textContent = media.playbackStatus;
  els.posRaw.textContent = formatMs(media.positionReportedMs);
  els.lutTicks.textContent = String(media.lastUpdatedRawTicks ?? 0);
  els.playbackRate.textContent = (media.playbackRate ?? 1).toFixed(2);
  els.timelineCount.textContent = String(media.timelineUpdates);
  els.lastEvent.textContent = media.lastEventKind;
}

function renderDebugClock(force = false) {
  if (!latestMedia || !debugVisible) return;
  const perf = performance.now();
  if (!force && perf - lastDebugClockRenderMs < DEBUG_PANEL_HZ_MS) return;
  lastDebugClockRenderMs = perf;
  const now = Date.now();
  const computed = playbackClockMs(latestMedia, playbackAnchor, now);
  const line = lyricsView.getLineDiagnostics(computed);
  els.posSmooth.textContent = formatMs(computed);
  els.posAge.textContent = `${timelineAnchorAgeMs(latestMedia.positionReportedAtMs, now)} ms`;
  const wall = latestMedia.positionExtrapolationWallMs ?? 0;
  els.wallAge.textContent =
    wall > 0 ? `${timelineAnchorAgeMs(wall, now)} ms` : "—";
  els.lineIndex.textContent =
    line.index >= 0 ? `${line.index} @ ${formatMs(line.lineTimeMs)}` : "—";
}

async function loadSyncOffset(media: FullMediaState) {
  try {
    const off = await invoke<number>("get_sync_offset", {
      artist: media.artist,
      track: media.title,
      album: media.album,
      durationMs: media.durationMs,
    });
    lyricsView.setSyncOffsetAbsolute(off);
    updateTimingReadout(off);
  } catch {
    lyricsView.setSyncOffsetAbsolute(0);
    updateTimingReadout(0);
  }
}

async function resolveWallpaperForDiagnostics(): Promise<WallpaperStatus | null> {
  try {
    return await fetchWallpaperStatus();
  } catch {
    return lastWallpaperStatus;
  }
}

async function logDiagnosticsToFile(reason: string) {
  const wallpaper = await resolveWallpaperForDiagnostics();
  const block = buildDiagnosticsBlock(
    latestMedia,
    latestLyrics,
    lyricsView,
    playbackAnchor,
    wallpaper,
  );
  const full = `${reason}\n\n${block}`;
  await invoke("log_diagnostics", { block: full });
}

async function loadLyrics(media: FullMediaState) {
  if (!lyricsView.shouldFetchLyrics(media)) return;

  const gen = ++fetchGeneration;
  lyricsView.onLyricsLoading();

  try {
    const res = await invoke<LyricsResponse>("fetch_lyrics", {
      artist: media.artist,
      track: media.title,
      album: media.album,
      durationMs: media.durationMs,
    });
    if (gen !== fetchGeneration) return;
    latestLyrics = res;
    lyricsView.onLyricsResponse(res);
    const untimed = res.status === "plain";
    els.plainBadge.classList.toggle("hidden", !untimed);
    if (untimed) {
      els.plainBadge.textContent = `Untimed · ${plainSourceLabel(res.lyricsSource)}`;
    }
  } catch (e) {
    if (gen !== fetchGeneration) return;
    latestLyrics = {
      status: "error",
      message: String(e),
      instrumental: false,
    };
    lyricsView.onLyricsResponse(latestLyrics);
  }
}

function onMedia(state: FullMediaState) {
  const prev = latestMedia;
  const trackKey = (m: FullMediaState) =>
    `${m.artist}|${m.title}|${m.album}|${m.durationMs}`;
  const trackChanged =
    !prev ||
    state.lastEventKind === "track" ||
    trackKey(prev) !== trackKey(state);

  latestMedia = state;
  playbackAnchor = createPlaybackAnchor(state, Date.now());
  lyricsView.onMediaHeader(state);

  if (trackChanged) {
    void loadLyrics(state);
    void loadSyncOffset(state);
    void logDiagnosticsToFile(`Track change: ${state.title} — ${state.artist}`);
  }

  renderDebugStatic(state);
  if (debugVisible) renderDebugClock(true);
}

let settingsOpen = false;

function setSettingsOpen(open: boolean) {
  settingsOpen = open;
  els.settingsPopover.hidden = !open;
  els.btnSettings.classList.toggle("active", open);
  els.btnSettings.setAttribute("aria-expanded", open ? "true" : "false");
}

function renderUi(state: UiState) {
  debugVisible = state.debugVisible;
  els.btnAlwaysOnTop.classList.toggle("active", state.alwaysOnTop);
  els.btnWallpaper.classList.toggle("active", state.wallpaperMode);
  els.wallpaperToggle.checked = state.wallpaperMode;
  applyWallpaperLayout(appRoot, state.wallpaperMode);
  void state.wallpaperError;
  els.debugPanel.classList.toggle("hidden", !state.debugVisible);
  els.debugPanel.setAttribute("aria-hidden", state.debugVisible ? "false" : "true");
  renderDebugClock(true);
}

let rafId = 0;

function tick() {
  if (latestMedia) {
    const playbackMs = playbackClockMs(
      latestMedia,
      playbackAnchor,
      Date.now(),
    );
    lyricsView.tick(playbackMs, latestMedia.durationMs);
    renderDebugClock();
  }
  rafId = requestAnimationFrame(tick);
}

function startTicker() {
  if (!rafId) rafId = requestAnimationFrame(tick);
}

els.btnAlwaysOnTop.addEventListener("click", (e) => {
  e.stopPropagation();
  void invoke("toggle_always_on_top");
});
els.btnWallpaper.addEventListener("click", (e) => {
  e.stopPropagation();
  void invoke("toggle_wallpaper_mode");
});

async function refreshWallpaperFromBackend() {
  try {
    lastWallpaperStatus = await fetchWallpaperStatus();
  } catch {
    /* harness */
  }
}

els.wallpaperToggle.addEventListener("change", async () => {
  const want = els.wallpaperToggle.checked;
  try {
    await invoke("set_wallpaper_mode", { enabled: want });
    await refreshWallpaperFromBackend();
  } catch {
    await refreshWallpaperFromBackend();
  }
});

els.btnSettings.addEventListener("click", (e) => {
  e.stopPropagation();
  setSettingsOpen(!settingsOpen);
});

document.addEventListener("mousedown", (e) => {
  if (!settingsOpen) return;
  const target = e.target as Node;
  if (els.settingsPopover.contains(target) || els.btnSettings.contains(target)) {
    return;
  }
  setSettingsOpen(false);
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && settingsOpen) {
    e.preventDefault();
    setSettingsOpen(false);
  }
});

els.opacitySlider.addEventListener("input", () => {
  const bg = Number(els.opacitySlider.value) / 100;
  applyAppearance(bg);
  void invoke("set_overlay_opacity", { opacity: bg });
});

els.timingSlider.addEventListener("input", () => {
  const ms = Number(els.timingSlider.value);
  lyricsView.setSyncOffsetAbsolute(ms);
  updateTimingReadout(ms);
  schedulePersistTiming();
});

els.timingReset.addEventListener("click", () => {
  void persistSyncOffset(0);
});

els.karaokeToggle.addEventListener("change", () => {
  void invoke("set_karaoke_ball", { enabled: els.karaokeToggle.checked });
});

els.ballColor.addEventListener("input", () => {
  applyKaraokeColors(els.ballColor.value, els.sungColor.value);
  els.bouncerPicker
    .querySelectorAll<HTMLElement>(".bouncer-picker-btn")
    .forEach((node) => {
      node.style.color = els.ballColor.value;
    });
  void invoke("set_karaoke_ball_color", { color: els.ballColor.value });
});

els.sungColor.addEventListener("input", () => {
  applyKaraokeColors(els.ballColor.value, els.sungColor.value);
  void invoke("set_sung_highlight_color", { color: els.sungColor.value });
});

els.colorsReset.addEventListener("click", () => {
  applyKaraokeColors(DEFAULT_BALL_COLOR, DEFAULT_SUNG_COLOR);
  void invoke("reset_karaoke_colors");
});

els.btnOpenLogFolder.addEventListener("click", () => {
  void invoke("open_lyricglass_log_folder");
});

els.btnCopyDiag.addEventListener("click", async () => {
  const wallpaper = await resolveWallpaperForDiagnostics();
  const block = buildDiagnosticsBlock(
    latestMedia,
    latestLyrics,
    lyricsView,
    playbackAnchor,
    wallpaper,
  );
  try {
    await navigator.clipboard.writeText(block);
  } catch {
    await invoke("log_diagnostics", { block });
  }
});

async function onSyncMark() {
  const block = buildSyncMarkBlock(latestMedia, lyricsView, playbackAnchor);
  await invoke("log_sync_mark", { block });
}

async function bootstrapWallpaperFromBackend() {
  const boot = await invoke<WallpaperSurfaceState>("get_wallpaper_surface_state");
  onMedia(boot.media);
  applyAppearanceSettings(boot.appearance);
  lyricsView.setSyncOffsetAbsolute(boot.syncOffsetMs);
  updateTimingReadout(boot.syncOffsetMs);
  if (boot.lyrics) {
    latestLyrics = boot.lyrics;
    lyricsView.onLyricsResponse(boot.lyrics);
  }
}

async function boot() {
  try {
    const label = (await getCurrentWebviewWindow()).label;
    if (label === "wallpaper") {
      wallpaperSurface = true;
    }
  } catch {
    /* non-tauri harness */
  }

  buildBouncerPicker();
  buildFontPickerUi();

  if (wallpaperSurface) {
    primeWallpaperSurfaceChrome();
    try {
      const payload = await invoke<DesktopWallpaperPayload>(
        "get_desktop_wallpaper_payload",
      );
      applyDesktopWallpaperCss(payload);
      if (payload.readError) {
        void logDiagnosticsToFile(
          `Wallpaper bg error: ${payload.readError} path=${payload.path}`,
        );
      }
    } catch (e) {
      void logDiagnosticsToFile(`Wallpaper bg invoke failed: ${e}`);
    }
    await bootstrapWallpaperFromBackend();
  }

  await listen<FullMediaState>("media-state", (e) => onMedia(e.payload));
  await listen<UiState>("ui-state", (e) => renderUi(e.payload));
  await listen<AppearanceState>("appearance-state", (e) =>
    applyAppearanceSettings(e.payload),
  );
  await listen<number>("sync-offset-absolute", (e) => {
    lyricsView.setSyncOffsetAbsolute(e.payload);
    updateTimingReadout(e.payload);
  });
  await listen("seek-tick", () => {
    lyricsView.onSeek();
    if (latestMedia) {
      playbackAnchor = createPlaybackAnchor(latestMedia, Date.now());
      lyricsView.tick(
        playbackClockMs(latestMedia, playbackAnchor, Date.now()),
      );
    }
  });
  await listen("sync-mark-request", () => void onSyncMark());
  await listen<string>("wallpaper-attach-failed", (e) => {
    void e.payload;
    void refreshWallpaperFromBackend();
    void logDiagnosticsToFile(`Wallpaper attach failed: ${e.payload}`);
  });

  await listen<WallpaperStatus>("wallpaper-status", (e) => {
    lastWallpaperStatus = e.payload;
  });

  const media = await invoke<FullMediaState>("get_media_state");
  onMedia(media);
  renderUi(await invoke<UiState>("get_ui_state"));
  applyAppearanceSettings(await invoke<AppearanceState>("get_appearance"));

  startTicker();

  document.addEventListener("visibilitychange", () => {
    if (document.hidden && rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    } else if (!document.hidden) {
      startTicker();
    }
  });
}

void boot();