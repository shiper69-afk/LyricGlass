import { describe, expect, it, vi } from "vitest";
import { SPLASH_HOLD_MS, scheduleSplashHandoff } from "./splash-handoff";

describe("splash handoff", () => {
  it("uses 2.6s hold to match animated WebP length", () => {
    expect(SPLASH_HOLD_MS).toBe(2600);
  });

  it("hands off after hold when image already loaded", () => {
    vi.useFakeTimers();
    const onHandoff = vi.fn();
    const image = {
      complete: true,
      naturalWidth: 720,
      addEventListener: vi.fn(),
    } as unknown as HTMLImageElement;

    scheduleSplashHandoff({
      holdMs: 100,
      image,
      onHandoff,
      setTimer: (cb, ms) => setTimeout(cb, ms),
    });

    expect(onHandoff).not.toHaveBeenCalled();
    vi.advanceTimersByTime(99);
    expect(onHandoff).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(onHandoff).toHaveBeenCalledOnce();
    vi.useRealTimers();
  });

  it("waits for image load before arming timer", () => {
    vi.useFakeTimers();
    const listeners: Record<string, () => void> = {};
    const onHandoff = vi.fn();
    const image = {
      complete: false,
      naturalWidth: 0,
      addEventListener: (ev: string, fn: () => void) => {
        listeners[ev] = fn;
      },
    } as unknown as HTMLImageElement;

    scheduleSplashHandoff({
      holdMs: 50,
      image,
      onHandoff,
      setTimer: (cb, ms) => setTimeout(cb, ms),
    });

    listeners.load?.();
    vi.advanceTimersByTime(50);
    expect(onHandoff).toHaveBeenCalledOnce();
    vi.useRealTimers();
  });
});