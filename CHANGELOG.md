# Changelog

All notable user-facing changes to LyricGlass are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and version lines align with squash-merged release pull requests where applicable.

## [0.9.0] - 2026-11-18

### Added

- **NetEase Cloud Music** as an optional lyrics provider: web search + eapi `song/lyric/v1` for **YRC** word-level lyrics, converted to enhanced LRC for karaoke.
- **Lyrics source** setting (General tab): **Auto** (NetEase when strict match, else LRCLIB, then Shironet), **LRCLIB only**, **NetEase only**; persisted in SQLite; changing it reloads the current track.
- Strict title/artist/duration matching (~3 s) so the wrong song is never shown; Shironet Hebrew fallback unchanged in Auto/LRCLIB modes.
- Subtle on-screen and diagnostics labels for lyrics provenance (`NetEase · word`, `LRCLIB`, `estimated`, `Shironet`).

### Changed

- Karaoke uses real YRC word timestamps when available; sync offset (+600 ms built-in + user slider) applies the same way for all providers.

## [0.8.2] - 2026-11-16

[#9](https://github.com/shiper69-afk/LyricGlass/pull/9)

### Added

- Five visualizer styles: **Warp** (starfield), **Mirror**, **Tunnel**, **Matrix**, **Terrain**, with tile icons in the Visualizer tab.
- JavaScript **beat fallback** detector when Rust beat flags are sparse.

### Fixed

- **Particles** visualizer now spawns continuously from level and frequency bands and bursts on beats (Rust beat flag or JS fallback); previously it relied almost entirely on rare Rust beat pulses, so the mode looked empty.

### Changed

- Visualizer picker expanded to **11 tiles** (10 styles + Off); grid scrolls in the settings popover at typical DPI.

## [0.8.1] - 2026-11-16

[#8](https://github.com/shiper69-afk/LyricGlass/pull/8)

### Added

- First shipped **music visualizer**: WASAPI loopback, Rust FFT/bands/beat detection, canvas renderer (Bars, Wave, Radial, Particles, Aurora).

### Changed

- Full-viewport canvas behind lyrics; stronger defaults (intensity, opacity); lyric shadow and active-line scrim for readability on wallpaper.

## [0.8.0] - 2026-11-16

Same release train as [#8](https://github.com/shiper69-afk/LyricGlass/pull/8): visualizer feature branch and CI/portable builds labeled **v0.8.0** during development; user-facing **0.8.1** adjustments landed before merge to `main`.

### Added

- Visualizer settings tab, intensity and opacity controls, idle/pause behavior when not playing.

## [0.7.2] - 2026-11-16

[#7](https://github.com/shiper69-afk/LyricGlass/pull/7)

### Added

- **Font chooser** with bundled Google Fonts (offline woff2).
- **Settings tabs**: General, Bouncer, Font.

### Fixed

- **Window-mode startup** (wallpaper preference cleared on launch).
- Tray and **second-launch focus** handoff improvements.
- Single-instance smoke coverage in CI.

## [0.6.11] - 2026-11-15

[#6](https://github.com/shiper69-afk/LyricGlass/pull/6)

### Added

- **Wallpaper mode**: attach lyrics surface behind desktop icons (WorkerW), sync with separate wallpaper webview.

### Fixed

- Full-screen lyrics layout and media/heartbeat stability on wallpaper attach.

## [0.6.7] - 2026-11-15

Release tags between wallpaper work and v0.7.2 included media watcher (MTA), watchdog, deferred wallpaper startup, single-instance plugin, and CI smoke hardening (see intermediate `cursor/*` branches merged toward [#6](https://github.com/shiper69-afk/LyricGlass/pull/6) / [#7](https://github.com/shiper69-afk/LyricGlass/pull/7)).

## [0.6.1] - 2026-11-15

### Added

- Built-in **+600 ms** GSMTC sync lag compensation (slider `0` still applies this offset via `effectiveSyncOffsetMs`).

## [0.5.1] - 2026-11-15

[#5](https://github.com/shiper69-afk/LyricGlass/pull/5)

### Added

- **Shironet** Hebrew lyrics fallback when LRCLIB has no match.
- **RTL** lyric layout; karaoke ball waits through the last word on a line.

## [0.5.0] - 2026-11-15

Phase 3 feature set shipped with [#5](https://github.com/shiper69-afk/LyricGlass/pull/5).

## [0.4.3] - 2026-11-15

[#4](https://github.com/shiper69-afk/LyricGlass/pull/4)

### Added

- **12 karaoke bouncers**, ball-led highlight, smooth line scroll, idle bounce.
- Playwright e2e for bouncer/karaoke flows.

## [0.4.0] - 2026-11-15

Karaoke bouncer milestone on the path to [#4](https://github.com/shiper69-afk/LyricGlass/pull/4).

## [0.3.2] - 2026-11-15

[#3](https://github.com/shiper69-afk/LyricGlass/pull/3)

### Added

- Phase 2 **karaoke ball** and word timing.

### Fixed

- NSIS uninstall when the app is not running (custom installer script; targeted `taskkill`).

## [0.2.4] - 2026-11-15

[#2](https://github.com/shiper69-afk/LyricGlass/pull/2)

### Added

- Phase 1: LRCLIB synced lyrics overlay, opacity and timing controls, always-on-top, diagnostics, SQLite cache.

## [0.2.3] - 2026-11-15

Initial Phase 1 merge versioning for [#2](https://github.com/shiper69-afk/LyricGlass/pull/2).

## [0.1.1] - 2026-11-14

[#1](https://github.com/shiper69-afk/LyricGlass/pull/1)

### Added

- Phase 0 spike: Tauri shell, GSMTC Spotify listener, transparent overlay, WorkerW wallpaper proof-of-concept, startup logging.

[0.9.0]: https://github.com/shiper69-afk/LyricGlass/compare/5908ccd...HEAD
[0.8.2]: https://github.com/shiper69-afk/LyricGlass/compare/bed7c26...5908ccd
[0.8.1]: https://github.com/shiper69-afk/LyricGlass/compare/20ac75d...bed7c26
[0.7.2]: https://github.com/shiper69-afk/LyricGlass/compare/cdb7b81...20ac75d
[0.6.11]: https://github.com/shiper69-afk/LyricGlass/compare/597fdf8...cdb7b81
[0.5.1]: https://github.com/shiper69-afk/LyricGlass/compare/8501723...597fdf8
[0.4.3]: https://github.com/shiper69-afk/LyricGlass/compare/0abc986...8501723
[0.3.2]: https://github.com/shiper69-afk/LyricGlass/compare/489554e...0abc986
[0.2.4]: https://github.com/shiper69-afk/LyricGlass/compare/aa7a831...489554e
[0.1.1]: https://github.com/shiper69-afk/LyricGlass/releases/tag/v0.1.1