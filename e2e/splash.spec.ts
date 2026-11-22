import { test, expect } from "@playwright/test";

test("startup splash shows then marks handoff complete", async ({ page }) => {
  await page.goto("/splash.html");
  await expect(page.locator("#splash-stage")).toBeVisible();
  await expect(page.locator("#splash-img")).toBeVisible();
  await page.waitForFunction(
    () => document.body.dataset.splashDone === "1",
    undefined,
    { timeout: 4500 },
  );
});