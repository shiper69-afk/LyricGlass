# LyricGlass

Windows app: floating, semi-transparent synced lyrics overlay for Spotify desktop (RTL Hebrew, wallpaper mode, LRCLIB lyrics).

## Phase 2 (v0.4.0) — karaoke bouncers

| Area | What it does |
| --- | --- |
| **Bouncers** | Twelve SVG karaoke cursors (`src/bouncers/svg`, Jack specs); per-type squash/stretch motion; **comet** + **paper plane** mirror in RTL. Default **Glow Orb**. |
| **Karaoke** | Bounces **above** each word; smooth `translate3d` line scroll + inter-line arc (seeks snap). `currentColor` follows ball colour picker. |
| **Word timing** | Enhanced LRC `<mm:ss.xx>` tags when present; else estimate from line start → capped next-line window, weighted by syllable length (English + Hebrew). |
| **Display** | Centered lines; `dir="auto"` for RTL; bouncer tracks measured word rects (wrap + bidi). |
| **Settings** | Gear: karaoke toggle, **bouncer grid** (12), ball/sung colours + reset (persisted). |
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