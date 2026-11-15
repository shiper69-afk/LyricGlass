import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
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

interface UiState {
  alwaysOnTop: boolean;
  wallpaperMode: boolean;
  debugVisible: boolean;
}

interface AppearanceState {
  overlayOpacity: number;
  karaokeBall: boolean;
  karaokeBallColor: string;
  sungHighlightColor: string;
}

const DEFAULT_BALL_COLOR = "#ffc857";
const DEFAULT_SUNG_COLOR = "#ffdc8c";

const lyricsRoot = document.getElementById("lyrics-root")!;
const lyricsView = new LyricsView(lyricsRoot);
const appRoot = document.getElementById("app")!;

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
  ballColor: document.getElementById("ball-color") as HTMLInputElement,
  sungColor: document.getElementById("sung-color") as HTMLInputElement,
  colorsReset: document.getElementById("colors-reset") as HTMLButtonElement,
};

let latestMedia: FullMediaState | null = null;
let playbackAnchor: PlaybackAnchor | null = null;
let latestLyrics: LyricsResponse | null = null;
let fetchGeneration = 0;
let debugVisible = false;
let timingPersistTimer = 0;
let lastDebugClockRenderMs = 0;
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

function applyAppearanceSettings(state: AppearanceState) {
  applyAppearance(state.overlayOpacity);
  els.karaokeToggle.checked = state.karaokeBall;
  lyricsView.setKaraokeEnabled(state.karaokeBall);
  applyKaraokeColors(state.karaokeBallColor, state.sungHighlightColor);
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

async function logDiagnosticsToFile(reason: string) {
  const block = buildDiagnosticsBlock(
    latestMedia,
    latestLyrics,
    lyricsView,
    playbackAnchor,
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
    els.plainBadge.classList.toggle("hidden", res.status !== "plain");
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
    lyricsView.tick(playbackMs);
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

els.btnCopyDiag.addEventListener("click", async () => {
  const block = buildDiagnosticsBlock(
    latestMedia,
    latestLyrics,
    lyricsView,
    playbackAnchor,
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

async function boot() {
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