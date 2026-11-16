import { describe, expect, it } from "vitest";
import {
  fontFamilyStack,
  HEBREW_FALLBACK_FAMILY,
  normalizeFontId,
} from "./catalog";

describe("fonts/catalog", () => {
  it("falls back unknown font ids to default", () => {
    expect(normalizeFontId("not-a-font")).toBe("default");
    expect(normalizeFontId("  inter  ")).toBe("inter");
  });

  it("stacks Rubik for latin-only fonts", () => {
    const stack = fontFamilyStack("inter");
    expect(stack).toContain("Inter");
    expect(stack).toContain(HEBREW_FALLBACK_FAMILY);
  });

  it("does not duplicate Rubik in hebrew-capable fonts", () => {
    const stack = fontFamilyStack("rubik");
    expect(stack).toContain("Rubik");
    const parts = stack.split(",").map((s) => s.trim());
    const rubikCount = parts.filter((p) => p.includes("Rubik")).length;
    expect(rubikCount).toBe(1);
  });
});