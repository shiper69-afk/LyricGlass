import { expect, test } from "@playwright/test";

test.describe("wallpaper surface mode", () => {
  test("?mode=wallpaper hides titlebar via CSS classes", async ({ page }) => {
    await page.goto("/index.html?mode=wallpaper");
    await page.waitForTimeout(500);
    const hasMode = await page.evaluate(() =>
      document.getElementById("app")?.classList.contains("wallpaper-mode"),
    );
    expect(hasMode).toBe(true);
  });

  test("wallpaper panel uses full height with safe bottom margin", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto("/index.html?mode=wallpaper");
    await page.waitForTimeout(400);
    const metrics = await page.evaluate(() => {
      const panel = document.querySelector(
        "#app.wallpaper-mode .lyrics-panel",
      ) as HTMLElement | null;
      if (!panel) return null;
      const r = panel.getBoundingClientRect();
      return {
        height: r.height,
        width: r.width,
        top: r.top,
      };
    });
    expect(metrics).not.toBeNull();
    if (metrics) {
      expect(metrics.height).toBeGreaterThan(900);
      expect(metrics.width).toBeGreaterThan(1200);
      expect(metrics.top).toBeLessThanOrEqual(4);
    }
  });
});