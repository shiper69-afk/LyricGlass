import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { formatMs, smoothedPositionMs } from "./clock";
import { LyricsView, type LyricsResponse, type MediaState } from "./lyrics-view";

interface FullMediaState extends MediaState {
  sourceAppId: string;
  lastEventKind: string;
  timelineUpdates: number;
  message: string;
}

interface UiState {
  clickThrough: boolean;
  wallpaperMode: boolean;
  debugVisible: boolean;
}

const lyricsRoot = document.getElementById("lyrics-root")!;
const lyricsView = new LyricsView(lyricsRoot);

const els = {
  mediaStatus: document.getElementById("media-status")!,
  album: document.getElementById("m-album")!,
  status: document.getElementById("m-status")!,
  posRaw: document.getElementById("m-position-raw")!,
  posSmooth: document.getElementById("m-position-smooth")!,
  timelineCount: document.getElementById("m-timeline-count")!,
  lastEvent: document.getElementById("m-last-event")!,
  btnClick: document.getElementById("btn-click") as HTMLButtonElement,
  btnWallpaper: document.getElementById("btn-wallpaper") as HTMLButtonElement,
  debugPanel: document.getElementById("debug-panel")!,
  plainBadge: document.getElementById("plain-badge")!,
};

let latestMedia: FullMediaState | null = null;
let fetchGeneration = 0;

function renderDebug(media: FullMediaState) {
  els.mediaStatus.textContent = media.message;
  els.album.textContent = media.album || "—";
  els.status.textContent = media.playbackStatus;
  els.posRaw.textContent = formatMs(media.positionReportedMs);
  els.timelineCount.textContent = String(media.timelineUpdates);
  els.lastEvent.textContent = media.lastEventKind;

  const playing = media.playbackStatus.toLowerCase() === "playing";
  const smooth = smoothedPositionMs(
    media.positionReportedMs,
    playing,
    Date.now() - media.positionReportedAtMs,
  );
  els.posSmooth.textContent = formatMs(smooth);
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
    lyricsView.onLyricsResponse(res);
    els.plainBadge.classList.toggle("hidden", res.status !== "plain");
  } catch (e) {
    if (gen !== fetchGeneration) return;
    lyricsView.onLyricsResponse({
      status: "error",
      message: String(e),
      instrumental: false,
    });
  }
}

function onMedia(state: FullMediaState) {
  const prevKind = latestMedia?.lastEventKind;
  latestMedia = state;
  lyricsView.onMediaHeader(state);

  const trackChanged =
    !prevKind ||
    state.lastEventKind === "track" ||
    lyricsView.shouldFetchLyrics(state);

  if (trackChanged) {
    void loadLyrics(state);
  }

  renderDebug(state);
}

function renderUi(state: UiState) {
  els.btnClick.classList.toggle("active", state.clickThrough);
  els.btnWallpaper.classList.toggle("active", state.wallpaperMode);
  els.debugPanel.classList.toggle("hidden", !state.debugVisible);
  els.debugPanel.setAttribute("aria-hidden", state.debugVisible ? "false" : "true");
}

let rafId = 0;

function tick() {
  if (latestMedia) {
    lyricsView.tick(latestMedia);
  }
  rafId = requestAnimationFrame(tick);
}

function startTicker() {
  if (!rafId) rafId = requestAnimationFrame(tick);
}

els.btnClick.addEventListener("click", () => void invoke("toggle_click_through"));
els.btnWallpaper.addEventListener("click", () => void invoke("toggle_wallpaper_mode"));

async function boot() {
  await listen<FullMediaState>("media-state", (e) => onMedia(e.payload));
  await listen<UiState>("ui-state", (e) => renderUi(e.payload));
  await listen<number>("sync-offset", (e) => lyricsView.setSyncOffset(e.payload));
  await listen("seek-tick", () => {
    lyricsView.onSeek();
    if (latestMedia) lyricsView.tick(latestMedia);
  });

  const media = await invoke<FullMediaState>("get_media_state");
  onMedia(media);
  renderUi(await invoke<UiState>("get_ui_state"));

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