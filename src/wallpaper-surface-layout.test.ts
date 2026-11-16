/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import { LyricsView } from "./lyrics-view";
import { installWallpaperLayoutVars } from "./wallpaper-layout";
import { reference1080pContextLines } from "./wallpaper-layout";

const SAMPLE_LRC = `[00:00.00] Line zero
[00:05.00] Line one
[00:10.00] Line two
[00:15.00] Line three
[00:20.00] Line four
[00:25.00] Line five
[00:30.00] Line six
[00:35.00] Line seven
[00:40.00] Line eight
[00:45.00] Line nine
[00:50.00] Line ten
`;

describe("wallpaper surface lyrics layout", () => {
  it("renders more than 3 lines at 1080p context", () => {
    expect(reference1080pContextLines()).toBeGreaterThan(6);
  });

  it("centers active line with a wide line window", () => {
    const app = document.createElement("div");
    app.id = "app";
    app.classList.add("wallpaper-mode");
    document.body.appendChild(app);
    Object.defineProperty(window, "innerHeight", {
      value: 1080,
      configurable: true,
    });
    Object.defineProperty(window, "innerWidth", {
      value: 1920,
      configurable: true,
    });
    installWallpaperLayoutVars(app);

    const root = document.createElement("section");
    root.className = "lyrics-panel";
    root.style.height = "900px";
    root.innerHTML = `
      <p class="np-header"><span id="np-title"></span><span id="np-artist"></span></p>
      <p id="lyrics-status"></p><p id="sync-offset-label"></p>
      <div id="lyrics-lines"></div><div id="lyrics-plain" class="hidden"></div>
      <p id="duration-warn" class="hidden"></p>
    `;
    app.appendChild(root);

    const linesEl = root.querySelector("#lyrics-lines") as HTMLElement;
    Object.defineProperty(linesEl, "clientHeight", {
      value: 820,
      configurable: true,
    });

    const view = new LyricsView(root);
    view.setWallpaperLayout(true);
    view.onLyricsResponse({
      status: "synced",
      message: "ok",
      syncedLrc: SAMPLE_LRC,
      instrumental: false,
    });
    view.onLyricsResponse({
      status: "synced",
      message: "ok",
      syncedLrc: SAMPLE_LRC,
      instrumental: false,
    });
    view.tick(25_000, 120_000);

    const count = view.getRenderedLineCount();
    expect(count).toBeGreaterThan(3);

    const lineEls = Array.from(
      root.querySelectorAll<HTMLElement>(".lyric-line"),
    );
    const activeIdx = lineEls.findIndex((el) =>
      el.classList.contains("active"),
    );
    expect(activeIdx).toBeGreaterThanOrEqual(0);
    const above = activeIdx;
    const below = lineEls.length - 1 - activeIdx;
    expect(above).toBeGreaterThan(2);
    expect(below).toBeGreaterThan(2);
    expect(Math.abs(above - below)).toBeLessThanOrEqual(2);
  });
});