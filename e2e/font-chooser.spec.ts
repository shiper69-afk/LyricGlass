import { expect, test } from "@playwright/test";

test.describe("font chooser", () => {
  test("switching font loads faces and keeps ball near active word", async ({
    page,
  }) => {
    await page.goto("/font-harness.html");
    const first = await page.evaluate(() =>
      window.__fontHarness.switchFont("caveat"),
    );
    expect(first.fontId).toBe("caveat");
    expect(first.ball).not.toBeNull();

    const second = await page.evaluate(() =>
      window.__fontHarness.switchFont("inter"),
    );
    expect(second.fontId).toBe("inter");
    expect(second.ball).not.toBeNull();
  });

  test("Hebrew on latin-only font uses bundled Rubik fallback", async ({
    page,
  }) => {
    await page.goto("/font-harness.html");
    const usesRubik = await page.evaluate(() =>
      window.__fontHarness.hebrewUsesRubikFallback("inter"),
    );
    expect(usesRubik).toBe(true);
  });
});