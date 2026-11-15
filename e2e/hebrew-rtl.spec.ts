import { expect, test } from "@playwright/test";

interface HebrewSample {
  playbackMs: number;
  ball: { x: number; y: number } | null;
  activeDir: string;
  ballWordApprox: number;
}

test.describe("Hebrew RTL karaoke", () => {
  test("active line is RTL; bouncer moves with word highlights", async ({
    page,
  }) => {
    await page.goto("/hebrew-harness.html");

    const samples = await page.evaluate(() =>
      window.__hebrewHarness.runWordHop(),
    );

    const dir = await page.evaluate(() =>
      window.__hebrewHarness.activeLineDirection(),
    );
    expect(dir).toBe("rtl");
    expect(samples.every((s) => !s.activeDir || s.activeDir === "rtl")).toBe(
      true,
    );

    const withBall = samples.filter((s: HebrewSample) => s.ball);
    expect(withBall.length).toBeGreaterThan(3);

    const highlighted = samples.filter((s) => s.ballWordApprox > 0);
    expect(highlighted.length).toBeGreaterThan(1);

    let prevX: number | null = null;
    for (const s of withBall) {
      if (prevX != null && s.ball) {
        expect(Math.abs(s.ball.x - prevX)).toBeLessThan(120);
      }
      prevX = s.ball?.x ?? null;
    }
  });
});