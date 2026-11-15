# LyricGlass

Windows app: floating, semi-transparent synced lyrics overlay for Spotify desktop (RTL Hebrew, wallpaper mode, LRCLIB lyrics in **Phase 1**).

## Phase 1 (v0.2.3)

| Area | What it does |
| --- | --- |
| **LRCLIB** | Rust client with duration-aware match (prefer ±2s; warn if Δ>3s). User-Agent: `LyricGlass/0.2.3` + repo URL. |
| **Playback clock** | Dual extrapolation: `LastUpdatedTime` **and** a monotonic wall anchor (Store Spotify often refreshes time while `Position` stays stale). |
| **Self-diagnostics** | Tray → diagnostics; **Copy diagnostics**; auto log on track change; **Ctrl+Alt+M** sync mark when a line starts. |
| **Quick settings** | Title bar **⚙** opens a pinned popover: **opacity** (persisted) and **timing** ±10s per track (syncs with Ctrl+Alt+[ / ]). |
| **Window** | **Always on top** (📌, default ON, persisted). Wallpaper mode (🖼 / Ctrl+Alt+W) temporarily disables top-most. |
| **Cache** | SQLite at `%LOCALAPPDATA%\LyricGlass\lyrics-cache.sqlite3` (7d hits, 6h for not-found/errors). Offline replay uses cache. |
| **LRC** | TypeScript parser + Vitest (multi-timestamp lines, `[offset:]`, blanks, enhanced word tags stripped for display). |
| **Overlay** | Previous / current / next lines, highlight + smooth CSS transitions, smoothed GSMTC clock. Pause freezes line; seek jumps; track change shows loading. `dir="auto"` per line. Subtle now-playing header. |
| **P0 retained** | Wallpaper attach, tray, hotkeys, startup logs. **Diagnostics** hidden by default — tray → **Show diagnostics**. |

**Out of scope:** Shironet, karaoke ball, installer/icon polish.

## Stack

Tauri 2, Rust (`reqwest`, `rusqlite`, `windows` GSMTC), TypeScript + Vite.

## Build locally (Windows)

Prerequisites: https://v2.tauri.app/start/prerequisites/

```powershell
git clone https://github.com/shiper69-afk/LyricGlass.git
cd LyricGlass
npm install
npm test
npm run tauri dev
```

Release: `npm run tauri build` → `src-tauri\target\release\bundle\nsis\` and `lyricglass.exe`.

### Logs & cache paths

- Startup log: `%LOCALAPPDATA%\LyricGlass\logs\lyricglass-<unix-time>.log`
- Lyrics cache: `%LOCALAPPDATA%\LyricGlass\lyrics-cache.sqlite3`

### CI artifacts

Artifact name: **`lyricglass-windows-<commit-sha>`** (NSIS + exe) from `windows-latest` on push/PR.

## Manual test checklist — Phase 1 (Windows)

Use Spotify desktop (Microsoft Store id `SpotifyAB.SpotifyMusic_...!Spotify` is supported).

### 1. Lyrics sync

1. Launch LyricGlass; play a mainstream track with synced lyrics on LRCLIB.
2. Let it play ~30s — highlight should track the vocal within a few hundred ms.
3. Tray → **Show diagnostics** — **Computed clock** advances; panel must not hang the app.

### 2. Timing & settings

1. Title bar **⚙** — popover stays open while dragging sliders; close with **⚙**, outside click, or **Esc**.
2. **Ctrl+Alt+]** / **Ctrl+Alt+[** nudge ±0.1s; **Ctrl+Alt+M** on line start.
3. **Copy diagnostics** or track-change log under `%LOCALAPPDATA%\LyricGlass\logs\`.

### 3. Overlay & modes

1. **📌** always on top (default on, persists).
2. **Ctrl+Alt+W** / 🖼 — wallpaper attach/detach.

### 4. Regression

1. Cold start — click-through disabled; log reaches `tauri setup complete`.
2. `npm test` and CI green.

## Hotkeys

| Key | Action |
| --- | --- |
| Ctrl+Alt+W | Toggle wallpaper attach |
| Ctrl+Alt+[ | Lyrics sync −100 ms |
| Ctrl+Alt+] | Lyrics sync +100 ms |
| Ctrl+Alt+M | Mark line start (logs clock vs LRC) |

## Layout

`src/` frontend · `src-tauri/src/lyrics/` LRCLIB + cache · `.github/workflows/windows-build.yml`