import { expect, test } from "@playwright/test";
import { VISUALIZER_KIND_COUNT, VISUALIZER_OPTIONS } from "../src/visualizer/catalog";

test.describe("visualizer settings tab", () => {
  test.use({
    viewport: { width: 650, height: 550 },
    deviceScaleFactor: 1.25,
  });

  test("lists all visualizer tiles and fits viewport", async ({ page }) => {
    await page.goto("/index.html");
    await page.click("#btn-settings");
    await page.locator('[data-settings-tab="visualizer"]').click();

    const tiles = page.locator(".visualizer-picker-tile");
    await expect(tiles).toHaveCount(VISUALIZER_KIND_COUNT);
    for (const opt of VISUALIZER_OPTIONS) {
      await expect(page.locator(`[data-visualizer="${opt.id}"]`)).toBeVisible();
      await expect(
        page.locator(`[data-visualizer="${opt.id}"] .visualizer-picker-icon svg`),
      ).toBeVisible();
    }

    const intensity = page.locator("#visualizer-intensity");
    const opacity = page.locator("#visualizer-opacity");
    await expect(intensity).toBeVisible();
    await expect(opacity).toBeVisible();
    await expect(intensity).toHaveAttribute("max", "300");
    await expect(opacity).toHaveAttribute("max", "100");

    const popover = page.locator("#settings-popover");
    const box = await popover.boundingBox();
    expect(box).toBeTruthy();
    if (box) {
      expect(box.y + box.height).toBeLessThanOrEqual(550 + 4);
    }

    const picker = page.locator(".visualizer-picker");
    await expect(picker).toBeVisible();

    await page.locator('[data-visualizer="particles"]').click();
    await expect(page.locator('[data-visualizer="particles"]')).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
});