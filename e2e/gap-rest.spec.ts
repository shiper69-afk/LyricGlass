import { expect, test } from "@playwright/test";

test.describe("inter-line gap ball", () => {
  test("intro bounce stays centered before first line", async ({ page }) => {
    await page.goto("/gap-harness.html");
    const r = await page.evaluate(() => window.__gapHarness.introCentered());
    expect(r.delta).toBeLessThan(36);
    expect(Math.abs(r.ballX - r.centerX)).toBeLessThan(36);
  });

  test("LTR long gap: ball stays over last word of line", async ({ page }) => {
    await page.goto("/gap-harness.html");
    const r = await page.evaluate(() =>
      window.__gapHarness.gapOnLastWord(0, 4_200),
    );
    expect(r.delta).toBeLessThan(28);
  });

  test("RTL long gap: ball stays over last (leftmost) word", async ({ page }) => {
    await page.goto("/gap-harness.html?mode=rtl");
    const r = await page.evaluate(() =>
      window.__gapHarness.gapOnLastWord(0, 3_500),
    );
    expect(r.delta).toBeLessThan(28);
  });
});