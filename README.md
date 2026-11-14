# LyricGlass

Planned Windows app: a floating, always-on-top, semi-transparent lyrics overlay for the Spotify desktop client (with synced lyrics, RTL Hebrew support, and wallpaper mode in later phases).

This repository's **Phase 0** spike is a small test app that de-risks three integrations on real Windows 10/11 machines.

## What this spike covers

| Area | Purpose |
| --- | --- |
| **Windows media controls** | Spotify now-playing via `GlobalSystemMediaTransportControlsSessionManager` (no Spotify login). Shows title, artist, album, duration, status, raw timeline position, and a smoothed interpolated clock between timeline events. |
| **Transparent overlay window** | Frameless, always-on-top, transparent WebView2 window; draggable title bar; click-through toggle (UI, tray, **Ctrl+Alt+C**); Hebrew RTL + English sample lines; bouncing-ball `requestAnimationFrame` demo and FPS counter (idle when animation is off). |
| **WorkerW wallpaper attach** | Re-parent behind desktop icons (Progman `0x052C` / WorkerW) and undo (**Ctrl+Alt+W**, tray, UI). |

**Out of scope:** LRCLIB / Shironet lyrics, real karaoke, settings, installer polish.

## Stack

Tauri 2, Rust (`windows` crate), TypeScript + Vite (WebView2).

## Build locally (Windows)

Prerequisites: https://v2.tauri.app/start/prerequisites/

```powershell
git clone https://github.com/shiper69-afk/LyricGlass.git
cd LyricGlass
npm install
npm run tauri dev
```

Release: `npm run tauri build` → installer under `src-tauri\target\release\bundle\nsis\`, exe at `src-tauri\target\release\lyricglass.exe`.

### CI artifacts

GitHub Actions on `windows-latest` uploads artifact **`lyricglass-windows-<commit-sha>`** (NSIS installer and/or `lyricglass.exe`) on each push/PR. Download from the workflow run **Artifacts** section.

## Manual test checklist (Windows)

Use Spotify desktop with LyricGlass running.

### A. Media session

1. Spotify closed → status says Spotify not in sessions; no crash.
2. Play a track → metadata and `playing` appear.
3. Pause → smoothed position stops; reported updates on timeline events.
4. Resume → smoothed clock advances between events.
5. Seek → reported position jumps; **Timeline events** increments.
6. Skip track → metadata changes; last event may be `track`.

Report: typical timeline event interval while playing, metadata gaps, smoothed vs reported drift.

### B. Overlay

1. Frameless, semi-transparent, always on top.
2. Drag title bar.
3. Animate on → ball moves, FPS ~60.
4. Animate off → FPS idle, low CPU.
5. English + Hebrew RTL lines visible.
6. Click-through on/off (button, tray, **Ctrl+Alt+C**).

Report: WebView2 transparency flash/ghosting, click-through stuck state.

### C. Wallpaper

1. Off → always-on-top overlay.
2. On → behind desktop icons.
3. Off again → restored; repeat without Explorer issues.

Report: multi-monitor, window missing, Explorer restart needed.

### D. Tray

Right-click: toggles + Quit. Left-click: show/focus window.

## Hotkeys

| Key | Action |
| --- | --- |
| Ctrl+Alt+C | Toggle click-through |
| Ctrl+Alt+W | Toggle wallpaper attach |

## Layout

`src/` frontend · `src-tauri/` Rust · `.github/workflows/` CI

Do not merge Phase 0 until manual Windows testing passes.
