# LyricGlass

Windows app: floating, semi-transparent synced lyrics overlay for Spotify desktop (RTL Hebrew, wallpaper mode, LRCLIB lyrics).

## Phase 2 (v0.3.1) — karaoke ball

| Area | What it does |
| --- | --- |
| **Karaoke ball** | Glowing ball bounces **above** each word (measured via `getClientRects`, wrapped + RTL); reserved lane on every line prevents overlap with the line above. |
| **Word timing** | Enhanced LRC `<mm:ss.xx>` tags when present; else estimate from line start → capped next-line window, weighted by syllable length (English + Hebrew). |
| **Display** | Centered lines; `dir="auto"` for RTL; ball follows measured word positions (remeasure on line change/resize only). |
| **Settings** | Gear popover: **Karaoke ball** toggle (persisted, default ON). Off for plain/untimed lyrics. |
| **Phase 1** | LRCLIB sync, diagnostics, opacity/timing, always-on-top, wallpaper — unchanged from v0.2.4. |
| **Installer** | Custom `installer.nsi` skips Tauri Restart Manager check; `taskkill` on exact `lyricglass.exe` only (fixes false “app running” on uninstall). |

## Stack

Tauri 2, Rust (`reqwest`, `rusqlite`, `windows` GSMTC), TypeScript + Vite, Vitest.

## Build locally (Windows)

```powershell
npm install && npm test && npm run tauri dev
```

Release: `npm run tauri build` → `src-tauri\target\release\bundle\nsis\`.

## Hotkeys

| Key | Action |
| --- | --- |
| Ctrl+Alt+W | Wallpaper attach |
| Ctrl+Alt+[ / ] | Lyrics sync ±100 ms |
| Ctrl+Alt+M | Sync mark (diagnostics log) |

## CI artifacts

`lyricglass-windows-<commit-sha>` from `windows-latest`.