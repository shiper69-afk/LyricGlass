# LyricGlass

Windows app: floating, semi-transparent synced lyrics overlay for Spotify desktop (RTL Hebrew, wallpaper mode, LRCLIB lyrics in **Phase 1**).

## Phase 1 (v0.2.2)

| Area | What it does |
| --- | --- |
| **LRCLIB** | Rust client with duration-aware match (prefer ±2s; warn if Δ>3s). User-Agent: `LyricGlass/0.2.2` + repo URL. |
| **Playback clock** | Dual extrapolation: `LastUpdatedTime` **and** a monotonic wall anchor (Store Spotify often refreshes time while `Position` stays stale). |
| **Self-diagnostics** | Tray → diagnostics; **Copy diagnostics**; auto log on track change; **Ctrl+Alt+M** sync mark when a line starts. |
| **Quick settings** | Hover title bar: **opacity** slider (persisted) and **timing** ±10s slider (per track, syncs with Ctrl+Alt+[ / ]). |
| **Cache** | SQLite at `%LOCALAPPDATA%\LyricGlass\lyrics-cache.sqlite3` (7d hits, 6h for not-found/errors). Offline replay uses cache. |
| **LRC** | TypeScript parser + Vitest (multi-timestamp lines, `[offset:]`, blanks, enhanced word tags stripped for display). |
| **Overlay** | Previous / current / next lines, highlight + smooth CSS transitions, smoothed GSMTC clock. Pause freezes line; seek jumps; track change shows loading. `dir="auto"` per line. Subtle now-playing header. |
| **P0 retained** | Click-through, wallpaper attach, tray, hotkeys, startup logs. **Diagnostics** panel hidden by default — tray → **Show diagnostics**. |

**Out of scope:** Shironet, karaoke ball, settings UI, installer/icon polish.

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

1. Launch LyricGlass; play a mainstream track with synced lyrics on LRCLIB (e.g. well-known English pop).
2. **Loading** → synced lines appear; **current line highlighted**; window shows previous/next lines.
3. Let it play ~30s — highlight should track the vocal within a few hundred ms (not 2–4 s late).
4. Tray → **Show diagnostics** — verify **Computed clock** advances smoothly while **Timeline age** may sit at several seconds between GSMTC updates; highlight should still follow audio.
5. **Pause** — highlight stays on the current line (no drift).
6. **Seek** backward/forward — line jumps promptly (within ~1 s of GSMTC catching up).
7. **Skip track** — brief loading, new lyrics or **No lyrics found**.
8. Try a track with only plain lyrics on LRCLIB — **Untimed lyrics** badge and scrollable text.
9. Disconnect network, replay a previously cached track — lyrics load instantly (**cached** in log/status area).

### 2. Timing & diagnostics

1. Hover the **title bar** — adjust **Opacity** and **Timing** (±10s); timing saves per track.
2. **Ctrl+Alt+]** / **Ctrl+Alt+[** nudge ±0.1s; **Ctrl+Alt+M** when a line starts (logs measured delta).
3. Tray → **Show diagnostics** → **Copy diagnostics** (paste to bug reports). Each track change also appends to `%LOCALAPPDATA%\\LyricGlass\\logs\\`.

### 3. Overlay & modes

1. Semi-transparent, readable on busy wallpaper (text shadow).
2. Hebrew line in synced LRC — correct RTL via `dir="auto"`.
3. **Ctrl+Alt+C** / tray — click-through.
4. **Ctrl+Alt+W** — wallpaper attach/detach.
5. Tray → **Show diagnostics** — Phase 0 GSMTC panel; hide again from tray.

### 4. Regression

1. Cold start — no crash; log reaches `tauri setup complete`.
2. Quit tray → uninstall — no false “Failed to kill LyricGlass”.
3. `npm test` and CI (build + smoke 10s) green.

Report: LRCLIB misses, plain-vs-synced, offset needed, multi-monitor wallpaper quirks.

## Hotkeys

| Key | Action |
| --- | --- |
| Ctrl+Alt+C | Toggle click-through |
| Ctrl+Alt+W | Toggle wallpaper attach |
| Ctrl+Alt+[ | Lyrics sync −500 ms |
| Ctrl+Alt+] | Lyrics sync +500 ms |

## Layout

`src/` frontend · `src-tauri/src/lyrics/` LRCLIB + cache · `.github/workflows/windows-build.yml`