import { test, expect } from "@playwright/test";

test("startup splash shows then marks handoff complete", async ({ page }) => {
  await page.goto("/splash.html");
  await expect(page.locator("#splash-stage")).toBeVisible();
  await expect(page.locator("#splash-img")).toBeVisible();
  const fit = await page.locator("#splash-img").evaluate((el) => {
    const style = getComputedStyle(el);
    return {
      objectFit: style.objectFit,
      maxWidth: style.maxWidth,
      maxHeight: style.maxHeight,
    };
  });
  expect(fit.objectFit).toBe("contain");
  expect(fit.maxWidth).toBe("100%");
  expect(fit.maxHeight).toBe("100%");
  const inset = await page.evaluate(() => {
    const stage = document.getElementById("splash-stage")!.getBoundingClientRect();
    const img = document.getElementById("splash-img")!.getBoundingClientRect();
    return {
      top: img.top - stage.top,
      bottom: stage.bottom - img.bottom,
      left: img.left - stage.left,
      right: stage.right - img.right,
    };
  });
  expect(inset.top).toBeGreaterThan(4);
  expect(inset.bottom).toBeGreaterThan(4);
  expect(inset.left).toBeGreaterThan(4);
  expect(inset.right).toBeGreaterThan(4);
  await page.waitForFunction(
    () => document.body.dataset.splashDone === "1",
    undefined,
    { timeout: 4500 },
  );
});

test("settings about shows inverse Shiperverse lockup", async ({ page }) => {
  await page.goto("/index.html");
  await page.click("#btn-settings");
  await page.locator('[data-settings-tab="about"]').click();
  const logo = page.locator('.about-logo');
  await expect(logo).toBeVisible();
  await expect(logo).toHaveAttribute("src", /logo-about\.png/);
  await page.waitForFunction(
    async () => {
      const img = document.querySelector<HTMLImageElement>(".about-logo");
      if (!img?.complete || img.naturalWidth < 1) return false;
      const canvas = document.createElement("canvas");
      canvas.width = 8;
      canvas.height = 8;
      const ctx = canvas.getContext("2d");
      if (!ctx) return false;
      ctx.drawImage(img, 0, 0, 8, 8);
      const data = ctx.getImageData(0, 0, 8, 8).data;
      let bright = 0;
      for (let i = 0; i < data.length; i += 4) {
        bright += (data[i]! + data[i + 1]! + data[i + 2]!) / 3;
      }
      return bright / (data.length / 4) > 120;
    },
    undefined,
    { timeout: 5000 },
  );
});

test("shiperverse branding screenshots", async ({ page }) => {
  test.skip(Boolean(process.env.CI), "artifact screenshots are for release capture");
  await page.setViewportSize({ width: 500, height: 800 });
  await page.goto("/splash.html");
  await page.waitForFunction(
    () => document.body.dataset.splashDone === "1",
    undefined,
    { timeout: 4500 },
  );
  await page.screenshot({ path: "/opt/cursor/artifacts/shiperverse-splash-done.png" });
  await page.setViewportSize({ width: 480, height: 400 });
  await page.goto("/index.html");
  await page.click("#btn-settings");
  await page.locator('[data-settings-tab="about"]').click();
  await expect(page.locator(".about-logo")).toBeVisible();
  await page.screenshot({ path: "/opt/cursor/artifacts/shiperverse-about-tab.png" });
});