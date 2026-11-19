# LyricGlass

LyricGlass is a Windows desktop app that shows synced song lyrics in a floating, semi-transparent glass overlay, timed to Spotify desktop playback. It is built with **Tauri 2**, a **Rust** backend (media sync, lyrics cache, optional audio capture), and a **TypeScript + Vite** front end (lyrics rendering, settings, canvas visualizer).

The overlay can run as a normal always-on-top window or attach behind desktop icons as **wallpaper mode**. Lyrics stay readable over busy backgrounds thanks to text shadow and a highlight scrim on the active line.

## Features

### Spotify sync and lyrics

- Reads playback state from **Windows Global System Media Transport Controls (GSMTC)**—no Spotify API login.
- **Lyrics source** (General settings): **Auto** (default), **LRCLIB only**, or **NetEase only**—stored in SQLite; changing it reloads the current track.
- **Auto**: tries **NetEase Cloud Music** first when Spotify duration is known and a **strict** title/artist/duration match succeeds, using unofficial `song/lyric/v1` **YRC** word-level lyrics when available; otherwise [LRCLIB](https://lrclib.net); then **Shironet** for Hebrew fallback as before.
- **NetEase** uses short HTTP timeouts (~4 s), caches like LRCLIB, dual search queries plus eapi cloud search when web search is blocked overseas, and never shows a song unless title and artist match and duration fits (±3 s, or ±8 s when title and artist match exactly). Diagnostics include a **lookup trace** for troubleshooting.
- When lyrics are on screen, a small **source badge** shows the active provider (hidden when there are no lyrics).
- **Shironet** fallback for Hebrew tracks when the primary source has no lyrics: plain, untimed text from [Shironet](https://shironet.mako.co.il).
- **RTL Hebrew** support: `dir="auto"` on lyric lines; karaoke bouncers that trail left are mirrored in RTL (Comet, Paper Plane).
- **Per-track timing slider** (−10 s … +10 s, 100 ms steps) plus hotkeys; stored offset is per track. Playback position uses an additional built-in **+600 ms** compensation for typical Microsoft Store Spotify GSMTC lag (effective offset = stored value + 600 ms unless you adjust the slider).
- Optional **diagnostics** panel (timeline, GSMTC fields, wallpaper status); **sync mark** hotkey writes a snapshot to the log.

### Karaoke

- Optional **karaoke ball** with per-word motion when timed lyrics exist; ball and **sung-line** colors are configurable.
- **12 SVG bouncers** (default **Glow Orb**): Glow Orb, Music Note, Spark, Heart, Glass Drop, Firefly, Comet, EQ Pulse, Mic, Paper Plane, Pixel, Crescent.
- Smooth line scrolling; **per-word karaoke** from NetEase YRC or enhanced LRC tags when present, otherwise estimated from line duration and syllable weighting. A small label shows the active source (`NetEase · word`, `LRCLIB`, `estimated`, `Shironet`).

### Appearance and settings

Settings open from the gear menu with four tabs:

| Tab | Contents |
| --- | --- |
| **General** | Overlay opacity, timing offset, **lyrics source**, karaoke on/off, wallpaper mode toggle, ball/sung colors, reset colors, open log folder |
| **Bouncer** | 12-tile bouncer picker |
| **Font** | Bundled lyrics fonts (18 choices including system default) |
| **Visualizer** | Style grid, intensity (0–3), opacity (0–100 %) |

- **Always on top** (pin control in the title bar).
- **Font chooser**: 17 bundled Google Font families (woff2 via Fontsource) plus **Default** (Segoe UI stack); Hebrew-capable families render Hebrew glyphs; Latin-only families fall back to bundled **Rubik** for Hebrew.

### Wallpaper mode

- Pins the lyrics (and visualizer) behind desktop icons on the WorkerW layer—full desktop coverage.
- Toggle from **General → Wallpaper mode**, the wallpaper toolbar button, or **Ctrl+Alt+W**.
- **Startup behavior**: the app always opens in normal **window mode** even if wallpaper was enabled when last closed; wallpaper is cleared on launch until you toggle it again.

### Music visualizer

- **Windows WASAPI loopback** capture, FFT and band analysis in Rust; frames are sent to a full-viewport **Canvas 2D** renderer (single `requestAnimationFrame` loop).
- Capture and emission run only while Spotify reports **playing**, the overlay window is **visible**, and the visualizer is not **Off**; after prolonged silence the pipeline idles to save CPU.
- **Intensity** default 1.5 (max 3.0); **opacity** default 80 %. Lyrics keep shadow/scrim when the visualizer is on.
- Styles in the settings grid (labels match the UI):

| ID | Label | Description |
| --- | --- | --- |
| `off` | Off | No visualizer |
| `spectrum` | Bars | Spectrum bars |
| `waveform` | Wave | Waveform line |
| `radial` | Radial | Circular spectrum |
| `particles` | Particles | Beat-driven particles with level/band motion |
| `aurora` | Aurora | Soft glow pulse |
| `starfield` | Warp | Starfield rush, faster on beats |
| `mirror` | Mirror | Center-split spectrum |
| `tunnel` | Tunnel | Pulsing ring tunnel |
| `matrix` | Matrix | Band-driven code rain |
| `terrain` | Terrain | Perspective scrolling waveform |

**11 tiles** in the grid (10 animated styles plus Off).

### Tray, single instance, and updates

- **Close (X)** hides the main window to the **system tray**; use the tray icon to show the window again or choose **Quit** to exit.
- **Single instance**: launching again while the app is running brings the existing window forward. If a **newer** build is started while an older one is running, the older process hands off and exits so the newer executable takes over.

## Hotkeys

| Shortcut | Action |
| --- | --- |
| **Ctrl+Alt+W** | Toggle wallpaper mode |
| **Ctrl+Alt+[** | Decrease lyrics sync offset by 100 ms |
| **Ctrl+Alt+]** | Increase lyrics sync offset by 100 ms |
| **Ctrl+Alt+M** | Sync mark (log current sync diagnostics) |

Global shortcuts use the Tauri global-shortcut plugin; if registration fails at startup, the failure is logged and other shortcuts may still work.

## Install and run (Windows)

LyricGlass targets **Windows 10/11** on **x64**.

### WebView2

Tauri uses the **Microsoft Edge WebView2** runtime. Most recent Windows installs include it; if the app fails to start, install the [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Evergreen bootstrapper or standalone installer).

### Downloads

CI produces two artifacts from each green Windows build:

| File | Use |
| --- | --- |
| **`lyricglass.exe`** (portable) | Unzip or copy anywhere; run directly. No installer. |
| **`LyricGlass_*_x64-setup.exe`** (NSIS) | Installs under `%LOCALAPPDATA%\LyricGlass` with Start Menu shortcut and uninstaller. |

Pick one: portable for quick trials; NSIS if you want a standard install path and uninstall entry.

## Build from source

**Prerequisites**

- Windows (for the shipped app and WASAPI visualizer)
- [Node.js](https://nodejs.org/) **22** (CI uses 22)
- [Rust](https://www.rust-lang.org/tools/install) stable toolchain
- WebView2 (see above)

**Commands**

```powershell
npm ci
npm test
cd src-tauri; cargo test; cd ..
npm run tauri dev      # development
npm run tauri build    # release + NSIS bundle
```

Release outputs:

- `src-tauri\target\release\lyricglass.exe`
- `src-tauri\target\release\bundle\nsis\LyricGlass_*_x64-setup.exe`

**E2E tests** (Playwright, Chromium):

```powershell
npm run test:e2e:install
npm run test:e2e
```

**CI** (GitHub Actions): on each push/PR, **Windows build** runs `npm ci`, Vitest, `cargo test`, `tauri build`, installer smoke tests, and uploads `lyricglass-windows-<commit-sha>`. **Frontend e2e** runs Playwright on Ubuntu.

## Bundled font licenses

Lyrics fonts ship as woff2 under `node_modules/@fontsource/*`. After `npm ci`, `postinstall` copies each family’s **SIL Open Font License 1.1** text to [`licenses/fonts/<family>/OFL.txt`](licenses/fonts/).

Bundled families (all OFL 1.1 unless noted in that family’s OFL file):

Amatic SC, Anton, Assistant, Bebas Neue, Bungee, Caveat, Frank Ruhl Libre, Heebo, Inter, Monoton, Nunito, Pacifico, Playfair Display, Press Start 2P, Rubik, Secular One, Varela Round.

Hebrew subsets are included where Fontsource provides them; families without Hebrew glyphs use **Rubik** as fallback for Hebrew text.

## License

LyricGlass application source is licensed under **GNU GPL version 3 or any later version** ([`LICENSE`](LICENSE), SPDX `GPL-3.0-or-later`).

Bundled lyrics fonts remain under the **SIL Open Font License 1.1**; see [`licenses/fonts/`](licenses/fonts/) for each family’s `OFL.txt`. Font files are aggregated with the app and are not covered by the GPL.