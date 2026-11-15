/** UI chrome when lyrics are embedded on the Windows desktop. */
export function applyWallpaperLayout(root: HTMLElement, enabled: boolean) {
  root.classList.toggle("wallpaper-mode", enabled);
  document.body.classList.toggle("wallpaper-mode", enabled);
}