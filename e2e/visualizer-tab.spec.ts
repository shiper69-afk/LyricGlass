import { expect, test } from "@playwright/test";

test.describe("visualizer settings tab", () => {
  test.use({
    viewport: { width: 650, height: 550 },
    deviceScaleFactor: 1.25,
  });

  test("shows six tiles and sliders", async ({ page }) => {
    await page.goto("/index.html");
    await page.click("#btn-settings");
    await page.locator('[data-settings-tab="visualizer"]').click();

    const tiles = page.locator(".visualizer-picker-tile");
    await expect(tiles).toHaveCount(6);
    await expect(page.locator("#visualizer-intensity")).toBeVisible();
    await expect(page.locator("#visualizer-opacity")).toBeVisible();

    await page.locator('[data-visualizer="spectrum"]').click();
    await expect(
      page.locator('[data-visualizer="spectrum"]'),
    ).toHaveAttribute("aria-pressed", "true");

    await page.locator('[data-visualizer="off"]').click();
    await expect(page.locator('[data-visualizer="off"]')).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
});