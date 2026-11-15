import { expect, test } from "@playwright/test";

interface HarnessSample {
  playbackMs: number;
  scrollY: number;
  ball: { x: number; y: number } | null;
}

test.describe("line scroll and bouncer continuity", () => {
  test("scroll translateY eases across frames; bouncer does not teleport", async ({
    page,
  }) => {
    await page.goto("/scroll-harness.html");

    const samples = await page.evaluate(() => {
      const h = (window as unknown as { __harness: { runLineTransition: () => HarnessSample[] } })
        .__harness;
      return h.runLineTransition();
    });

    expect(samples.length).toBeGreaterThan(10);

    const preScroll = samples.filter((s) => s.playbackMs < 2_580);
    expect(preScroll.every((s) => Math.abs(s.scrollY) < 2)).toBe(true);

    const mid = samples.filter((s) => s.playbackMs >= 2_590 && s.playbackMs <= 3_020);
    const scrollYs = mid.map((s) => s.scrollY);
    const maxScroll = Math.max(...scrollYs.map((y) => Math.abs(y)));
    expect(maxScroll).toBeGreaterThan(8);

    const deltas = scrollYs.slice(1).map((y, i) => y - scrollYs[i]);
    const maxStep = Math.max(...deltas.map(Math.abs));
    expect(maxStep).toBeLessThan(80);

    const easingSteps = deltas.filter((d) => d < -0.25).length;
    expect(easingSteps).toBeGreaterThan(3);

    let prev: { x: number; y: number } | null = null;
    for (const s of samples) {
      if (!s.ball) continue;
      if (prev) {
        const dx = Math.abs(s.ball.x - prev.x);
        const dy = Math.abs(s.ball.y - prev.y);
        expect(dx).toBeLessThan(92);
        expect(dy).toBeLessThan(90);
      }
      prev = s.ball;
    }
  });
});