/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import { LyricsView, type LyricsResponse } from "./lyrics-view";
import {
  applyDesktopWallpaperCss,
  applyWallpaperSurfaceChrome,
  lyricsTextFromResponse,
} from "./wallpaper-surface";

describe("wallpaper-surface", () => {
  it("hides titlebar and applies wallpaper-mode chrome", () => {
    document.body.innerHTML = `<div id="app"><header class="titlebar">x</header><footer class="hints"></footer></div>`;
    const root = document.getElementById("app")!;
    applyWallpaperSurfaceChrome(root);
    expect(root.classList.contains("wallpaper-mode")).toBe(true);
    expect(root.querySelector(".titlebar")?.classList.contains("hidden")).toBe(true);
  });

  it("uses data URL for desktop background when provided", () => {
    applyDesktopWallpaperCss({
      path: "C:\\img.jpg",
      dataUrl: "data:image/jpeg;base64,abc",
      desktopColor: "#112233",
    });
    const v = document.documentElement.style.getPropertyValue("--desktop-wallpaper-url");
    expect(v).toContain("data:image/jpeg;base64,abc");
  });

  it("renders lyrics from bootstrap response and from event-shaped updates", () => {
    const root = document.createElement("section");
    root.innerHTML = `
      <p class="np-header"><span id="np-title"></span><span id="np-artist"></span></p>
      <p id="lyrics-status"></p><p id="sync-offset-label"></p>
      <div id="lyrics-lines"></div><div id="lyrics-plain" class="hidden"></div>
      <p id="duration-warn" class="hidden"></p>
    `;
    const view = new LyricsView(root);
    const initial: LyricsResponse = {
      status: "plain",
      message: "ok",
      plainLyrics: "Line one\nLine two",
      instrumental: false,
    };
    expect(lyricsTextFromResponse(initial)).toContain("Line one");
    view.onLyricsResponse(initial);
    expect(root.textContent).toContain("Line one");

    const fromEvent: LyricsResponse = {
      status: "plain",
      message: "updated",
      plainLyrics: "Event lyric line",
      instrumental: false,
    };
    view.onLyricsResponse(fromEvent);
    expect(root.textContent).toContain("Event lyric line");
  });
});