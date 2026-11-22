import { invoke } from "@tauri-apps/api/core";
import { SPLASH_HOLD_MS, scheduleSplashHandoff } from "./splash-handoff";

const img = document.getElementById("splash-img");
if (!(img instanceof HTMLImageElement)) {
  document.body.dataset.splashDone = "1";
} else {
  scheduleSplashHandoff({
    holdMs: SPLASH_HOLD_MS,
    onHandoff: async () => {
      try {
        if ("__TAURI_INTERNALS__" in window) {
          await invoke("complete_startup_splash");
        }
      } catch {
        /* test / dev without backend */
      }
      document.body.dataset.splashDone = "1";
    },
    image: img,
  });
}