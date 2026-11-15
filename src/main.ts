import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { formatMs, smoothedPositionMs } from "./clock";

interface MediaState {
  spotifyRunning: boolean;
  sourceAppId: string;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  positionReportedMs: number;
  positionReportedAtMs: number;
  playbackStatus: string;
  lastEventKind: string;
  timelineUpdates: number;
  message: string;
}

interface UiState {
  clickThrough: boolean;
  wallpaperMode: boolean;
}

const els = {
  mediaStatus: document.getElementById("media-status")!,
  title: document.getElementById("m-title")!,
  artist: document.getElementById("m-artist")!,
  album: document.getElementById("m-album")!,
  status: document.getElementById("m-status")!,
  duration: document.getElementById("m-duration")!,
  posRaw: document.getElementById("m-position-raw")!,
  posSmooth: document.getElementById("m-position-smooth")!,
  timelineCount: document.getElementById("m-timeline-count")!,
  lastEvent: document.getElementById("m-last-event")!,
  btnClick: document.getElementById("btn-click") as HTMLButtonElement,
  btnWallpaper: document.getElementById("btn-wallpaper") as HTMLButtonElement,
  animEnabled: document.getElementById("anim-enabled") as HTMLInputElement,
  canvas: document.getElementById("karaoke-canvas") as HTMLCanvasElement,
  fps: document.getElementById("fps-label")!,
};

let latestMedia: MediaState | null = null;

function renderMedia(state: MediaState) {
  latestMedia = state;
  els.mediaStatus.textContent = state.message;
  els.title.textContent = state.title || "—";
  els.artist.textContent = state.artist || "—";
  els.album.textContent = state.album || "—";
  els.status.textContent = state.playbackStatus;
  els.duration.textContent = formatMs(state.durationMs);
  els.posRaw.textContent = formatMs(state.positionReportedMs);
  els.timelineCount.textContent = String(state.timelineUpdates);
  els.lastEvent.textContent = state.lastEventKind;
}

function renderUi(state: UiState) {
  els.btnClick.classList.toggle("active", state.clickThrough);
  els.btnWallpaper.classList.toggle("active", state.wallpaperMode);
}

function updateSmoothedClock() {
  if (!latestMedia) return;
  const now = Date.now();
  const playing = latestMedia.playbackStatus.toLowerCase() === "playing";
  const smooth = smoothedPositionMs(
    latestMedia.positionReportedMs,
    playing,
    now - latestMedia.positionReportedAtMs,
  );
  els.posSmooth.textContent = formatMs(smooth);
}

els.btnClick.addEventListener("click", async () => {
  await invoke("toggle_click_through");
});

els.btnWallpaper.addEventListener("click", async () => {
  await invoke("toggle_wallpaper_mode");
});

let rafId = 0;
let clockTimer: number | undefined;

const ctx = els.canvas.getContext("2d")!;

function drawBouncingBall(t: number) {
  const w = els.canvas.width;
  const h = els.canvas.height;
  ctx.clearRect(0, 0, w, h);

  const x = (w * 0.1) + (Math.sin(t * 0.0025) * 0.5 + 0.5) * (w * 0.8);
  const y = h * 0.65 + Math.abs(Math.sin(t * 0.006)) * -18;

  ctx.fillStyle = "rgba(255, 210, 90, 0.95)";
  ctx.beginPath();
  ctx.arc(x, y, 9, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = "rgba(255, 255, 255, 0.35)";
  ctx.fillRect(0, h - 6, w, 2);
}

let lastFpsT = 0;
let frameCount = 0;
let fpsValue = 0;

function loop(now: number) {
  if (!els.animEnabled.checked) {
    rafId = 0;
    els.fps.textContent = "FPS: idle";
    return;
  }

  drawBouncingBall(now);
  frameCount += 1;
  if (now - lastFpsT >= 500) {
    fpsValue = Math.round((frameCount * 1000) / (now - lastFpsT));
    frameCount = 0;
    lastFpsT = now;
    els.fps.textContent = `FPS: ${fpsValue}`;
  }

  updateSmoothedClock();
  rafId = requestAnimationFrame(loop);
}

function startAnimationLoop() {
  if (rafId) cancelAnimationFrame(rafId);
  if (els.animEnabled.checked) {
    lastFpsT = performance.now();
    frameCount = 0;
    rafId = requestAnimationFrame(loop);
  } else {
    els.fps.textContent = "FPS: idle";
    if (!clockTimer) {
      clockTimer = window.setInterval(updateSmoothedClock, 250);
    }
  }
}

els.animEnabled.addEventListener("change", () => {
  if (els.animEnabled.checked && clockTimer) {
    clearInterval(clockTimer);
    clockTimer = undefined;
  }
  startAnimationLoop();
});

async function boot() {
  await listen<MediaState>("media-state", (e) => renderMedia(e.payload));
  await listen<UiState>("ui-state", (e) => renderUi(e.payload));

  const media = await invoke<MediaState>("get_media_state");
  renderMedia(media);
  const ui = await invoke<UiState>("get_ui_state");
  renderUi(ui);

  startAnimationLoop();

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      if (rafId) {
        cancelAnimationFrame(rafId);
        rafId = 0;
      }
    } else if (els.animEnabled.checked) {
      startAnimationLoop();
    }
  });

  void getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (!focused && rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
      els.fps.textContent = "FPS: paused (unfocused)";
    } else if (focused && els.animEnabled.checked) {
      startAnimationLoop();
    }
  });
}

void boot();