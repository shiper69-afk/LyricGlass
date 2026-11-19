## LyricGlass v0.9.1

### Highlights

- **NetEase word-level lyrics (YRC)** when a strict catalog match succeeds, with karaoke synced to YRC or enhanced LRC tags.
- **Lyrics source** control: **Auto**, **LRCLIB only**, or **NetEase only** — each mode calls only its provider chain (Shironet remains **Auto**-only for Hebrew).
- **Source badge** on the overlay when lyrics are shown; diagnostics include **lookup trace** for NetEase searches.
- **0.9.1 fixes**: source-isolated errors, dual-query NetEase search + eapi fallback for overseas users, and improved duration matching (±3 s / ±8 s).

### Which file to download?

| Asset | When to use |
| --- | --- |
| **LyricGlass_0.9.1_portable.exe** | Run anywhere; no install (same as CI `lyricglass.exe`). |
| **LyricGlass_0.9.1_x64-setup.exe** | NSIS installer to `%LOCALAPPDATA%\LyricGlass` with uninstaller. |

**Windows x64** and **WebView2 Runtime** are required. See [README](https://github.com/shiper69-afk/LyricGlass#readme) for setup details.