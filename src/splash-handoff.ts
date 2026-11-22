/** Animated WebP length from Shiperverse splash spec (play once, then hold final frame). */
export const SPLASH_HOLD_MS = 2600;

export interface SplashHandoffOptions {
  holdMs: number;
  image: HTMLImageElement;
  onHandoff: () => void | Promise<void>;
  now?: () => number;
  setTimer?: typeof window.setTimeout;
}

/** Schedule a one-shot handoff after the splash image is ready. */
export function scheduleSplashHandoff(options: SplashHandoffOptions): void {
  const { holdMs, image, onHandoff } = options;
  const setTimer = options.setTimer ?? window.setTimeout.bind(window);
  let scheduled = false;

  const fire = () => {
    if (scheduled) return;
    scheduled = true;
    void Promise.resolve(onHandoff());
  };

  const arm = () => setTimer(fire, holdMs);

  if (image.complete && image.naturalWidth > 0) {
    arm();
  } else {
    image.addEventListener("load", arm, { once: true });
    image.addEventListener("error", () => setTimer(fire, 0), { once: true });
  }
}