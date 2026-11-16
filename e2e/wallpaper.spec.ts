import { expect, test } from "@playwright/test";

test.describe("wallpaper UI", () => {
  test("toggles wallpaper layout class and persists setting", async ({ page }) => {
    await page.goto("/wallpaper-harness.html");
    await page.evaluate(() => localStorage.removeItem("lg_wallpaper_mode"));

    expect(
      await page.evaluate(() => window.__wallpaperHarness.isLayoutOn()),
    ).toBe(false);

    await page.evaluate(() => window.__wallpaperHarness.toggle(true));
    expect(
      await page.evaluate(() => window.__wallpaperHarness.isLayoutOn()),
    ).toBe(true);
    expect(
      await page.evaluate(() => window.__wallpaperHarness.isPersisted()),
    ).toBe(true);

    await page.reload();
    expect(
      await page.evaluate(() => window.__wallpaperHarness.isLayoutOn()),
    ).toBe(true);
  });

  test("hides chrome in wallpaper layout", async ({ page }) => {
    await page.goto("/wallpaper-harness.html");
    await page.evaluate(() => window.__wallpaperHarness.toggle(true));
    const titlebar = page.locator(".titlebar");
    await expect(titlebar).toBeHidden();
    const panel = page.locator(".lyrics-panel");
    await expect(panel).toBeVisible();
  });
});