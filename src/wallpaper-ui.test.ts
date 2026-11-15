/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import { applyWallpaperLayout } from "./wallpaper-ui";

describe("wallpaper-ui", () => {
  it("toggles layout class on app root and body", () => {
    const root = document.createElement("div");
    applyWallpaperLayout(root, true);
    expect(root.classList.contains("wallpaper-mode")).toBe(true);
    expect(document.body.classList.contains("wallpaper-mode")).toBe(true);
    applyWallpaperLayout(root, false);
    expect(root.classList.contains("wallpaper-mode")).toBe(false);
    expect(document.body.classList.contains("wallpaper-mode")).toBe(false);
  });
});