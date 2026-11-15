import { applyWallpaperLayout } from "../wallpaper-ui";

const STORAGE_KEY = "lg_wallpaper_mode";

const app = document.getElementById("app")!;
const toggle = document.getElementById("wallpaper-toggle") as HTMLInputElement;

function setMode(on: boolean, persist = true) {
  toggle.checked = on;
  applyWallpaperLayout(app, on);
  if (persist) {
    localStorage.setItem(STORAGE_KEY, on ? "1" : "0");
  }
}

setMode(localStorage.getItem(STORAGE_KEY) === "1", false);

toggle.addEventListener("change", () => setMode(toggle.checked));

export interface WallpaperHarnessApi {
  toggle(on: boolean): void;
  isLayoutOn(): boolean;
  isPersisted(): boolean;
}

(window as unknown as { __wallpaperHarness: WallpaperHarnessApi }).__wallpaperHarness =
  {
    toggle(on: boolean) {
      setMode(on);
    },
    isLayoutOn() {
      return app.classList.contains("wallpaper-mode");
    },
    isPersisted() {
      return localStorage.getItem(STORAGE_KEY) === "1";
    },
  };