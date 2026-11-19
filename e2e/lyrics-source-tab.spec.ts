import { expect, test } from "@playwright/test";
import { LYRICS_SOURCE_OPTIONS } from "../src/lyrics-source-label";

test.describe("lyrics source settings", () => {
  test.use({
    viewport: { width: 650, height: 550 },
    deviceScaleFactor: 1.25,
  });

  test("control renders and popover fits viewport", async ({ page }) => {
    await page.goto("/index.html");
    await page.click("#btn-settings");

    const select = page.locator("#lyrics-source-select");
    await expect(select).toBeVisible();
    for (const opt of LYRICS_SOURCE_OPTIONS) {
      await expect(select.locator(`option[value="${opt.id}"]`)).toHaveCount(1);
    }

    await select.selectOption("lrclib");
    await expect(select).toHaveValue("lrclib");

    const popover = page.locator("#settings-popover");
    const box = await popover.boundingBox();
    expect(box).toBeTruthy();
    if (box) {
      expect(box.y + box.height).toBeLessThanOrEqual(550 + 4);
    }
  });
});