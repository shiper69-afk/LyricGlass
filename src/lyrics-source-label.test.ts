import { describe, expect, it } from "vitest";
import { lyricsProviderLabel } from "./lyrics-source-label";

describe("lyricsProviderLabel", () => {
  it("labels NetEase word-level", () => {
    expect(lyricsProviderLabel("netease-yrc", true)).toBe("NetEase · word");
  });

  it("labels estimated LRCLIB", () => {
    expect(lyricsProviderLabel("lrclib-synced", false)).toBe("estimated");
  });
});