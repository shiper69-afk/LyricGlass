import { expect, test } from "@playwright/test";
import { FONT_CATALOG } from "../src/fonts/catalog";

test.describe("settings panel layout", () => {
  test.use({
    viewport: { width: 650, height: 550 },
    deviceScaleFactor: 1.25,
  });

  test("fits viewport, single scroll, all font tiles reachable", async ({
    page,
  }) => {
    await page.goto("/index.html");
    await page.click("#btn-settings");
    const popover = page.locator("#settings-popover");
    await expect(popover).toBeVisible();

    const layout = await page.evaluate(() => {
      const pop = document.getElementById("settings-popover")!;
      const body = document.getElementById("settings-body")!;
      const pr = pop.getBoundingClientRect();
      const vh = window.innerHeight;
      const offenders: string[] = [];
      pop.querySelectorAll<HTMLElement>("*").forEach((el) => {
        if (el === body) return;
        const s = getComputedStyle(el);
        const oy = s.overflowY;
        if (
          (oy === "auto" || oy === "scroll") &&
          el.scrollHeight > el.clientHeight + 4
        ) {
          offenders.push(
            `${el.className || el.tagName} ${el.scrollHeight}/${el.clientHeight}`,
          );
        }
      });
      return {
        bottomInside: pr.bottom <= vh + 1,
        top: pr.top,
        bottom: pr.bottom,
        vh,
        nestedScrollers: offenders,
        bodyScrollable: body.scrollHeight > body.clientHeight,
      };
    });

    expect(layout.bottomInside).toBe(true);
    expect(layout.nestedScrollers).toEqual([]);

    await page.locator('[data-settings-tab="font"]').click();
    const fontCount = await page.locator(".font-picker-btn").count();
    expect(fontCount).toBe(FONT_CATALOG.length);

    const lastTile = page.locator(".font-picker-btn").last();
    await lastTile.scrollIntoViewIfNeeded();
    await expect(lastTile).toBeVisible();

    const scrollOk = await page.evaluate(() => {
      const body = document.getElementById("settings-body")!;
      const last = document.querySelector<HTMLElement>(
        ".font-picker-btn:last-of-type",
      );
      if (!last) return false;
      const br = last.getBoundingClientRect();
      const wr = body.getBoundingClientRect();
      const clipped = br.bottom <= wr.bottom + 4 && br.top >= wr.top - 4;
      const reached =
        body.scrollTop + body.clientHeight >= body.scrollHeight - 4;
      return clipped || reached;
    });
    expect(scrollOk).toBe(true);
  });

  test("before/after screenshots at 650x550", async ({ page }) => {
    await page.goto("/index.html");
    await page.screenshot({
      path: "/opt/cursor/artifacts/settings-panel-closed-650x550.png",
    });
    await page.click("#btn-settings");
    await page.locator('[data-settings-tab="general"]').click();
    await page.screenshot({
      path: "/opt/cursor/artifacts/settings-panel-general-tab.png",
    });
    await page.locator('[data-settings-tab="font"]').click();
    await page.screenshot({
      path: "/opt/cursor/artifacts/settings-panel-font-tab.png",
    });
  });
});