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
});