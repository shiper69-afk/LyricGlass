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
  clickThrough: boolean;
  wallpaperMode: boolean;
  debugVisible: boolean;
}

interface AppearanceState {
  overlayOpacity: number;
}

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
  btnClick: document.getElementById("btn-click") as HTMLButtonElement,
  btnWallpaper: document.getElementById("btn-wallpaper") as HTMLButtonElement,
  debugPanel: document.getElementById("debug-panel")!,
  plainBadge: document.getElementById("plain-badge")!,
  btnCopyDiag: document.getElementById("btn-copy-diag") as HTMLButtonElement,
  opacitySlider: document.getElementById("opacity-slider") as HTMLInputElement,
  timingSlider: document.getElementById("timing-slider") as HTMLInputElement,
  timingReset: document.getElementById("timing-reset") as HTMLButtonElement,
  timingReadout: document.getElementById("timing-readout")!,
};

let latestMedia: FullMediaState | null = null;
let playbackAnchor: PlaybackAnchor | null = null;
let latestLyrics: LyricsResponse | null = null;
let fetchGeneration = 0;
let debugVisible = false;
let timingPersistTimer = 0;

function applyAppearance(opacity: number) {
  const panel = Math.max(0.3, Math.min(1, opacity));
  const text = 0.45 + panel * 0.55;
  appRoot.style.setProperty("--panel-alpha", String(panel * 0.62));
  appRoot.style.setProperty("--text-alpha", String(text));
  els.opacitySlider.value = String(Math.round(panel * 100));
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

function renderDebugClock() {
  if (!latestMedia || !debugVisible) return;
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
  renderDebugClock();
}

function renderUi(state: UiState) {
  debugVisible = state.debugVisible;
  els.btnClick.classList.toggle("active", state.clickThrough);
  els.btnWallpaper.classList.toggle("active", state.wallpaperMode);
  els.debugPanel.classList.toggle("hidden", !state.debugVisible);
  els.debugPanel.setAttribute("aria-hidden", state.debugVisible ? "false" : "true");
  renderDebugClock();
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

els.btnClick.addEventListener("click", () => void invoke("toggle_click_through"));
els.btnWallpaper.addEventListener("click", () => void invoke("toggle_wallpaper_mode"));

els.opacitySlider.addEventListener("input", () => {
  const opacity = Number(els.opacitySlider.value) / 100;
  applyAppearance(opacity);
  void invoke("set_overlay_opacity", { opacity });
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
    applyAppearance(e.payload.overlayOpacity),
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
  applyAppearance((await invoke<AppearanceState>("get_appearance")).overlayOpacity);

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