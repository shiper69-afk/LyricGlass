import { describe, expect, it } from "vitest";
import { findActiveLineIndex, parseLrc, stripEnhancedTags } from "./lrc";

describe("parseLrc", () => {
  it("parses basic timestamps", () => {
    const parsed = parseLrc("[00:12.50] Hello\n[00:15.00] World");
    expect(parsed.lines).toHaveLength(2);
    expect(parsed.lines[0].timeMs).toBe(12_500);
    expect(parsed.lines[1].text).toBe("World");
  });

  it("supports multiple timestamps on one line", () => {
    const parsed = parseLrc("[00:01.00][00:05.00] Repeat");
    expect(parsed.lines).toHaveLength(2);
    expect(parsed.lines[0].timeMs).toBe(1000);
    expect(parsed.lines[1].timeMs).toBe(5000);
    expect(parsed.lines[0].text).toBe("Repeat");
  });

  it("applies offset tag", () => {
    const parsed = parseLrc("[offset:+500]\n[00:01.00] Hi");
    expect(parsed.offsetMs).toBe(500);
    expect(parsed.lines[0].timeMs).toBe(1500);
  });

  it("skips blank lines", () => {
    const parsed = parseLrc("\n\n[00:01.00] Only\n\n");
    expect(parsed.lines).toHaveLength(1);
  });

  it("tolerates enhanced word tags in text", () => {
    const parsed = parseLrc("[00:10.00] <00:10.12>Hello <00:10.40>world");
    expect(parsed.lines[0].text).toBe("Hello world");
    expect(stripEnhancedTags("<00:01.00>test")).toBe("test");
  });

  it("supports colon centiseconds", () => {
    const parsed = parseLrc("[01:02:50] Line");
    expect(parsed.lines[0].timeMs).toBe(62_500);
  });
});

describe("findActiveLineIndex", () => {
  const lines = [
    { timeMs: 0, text: "a" },
    { timeMs: 5000, text: "b" },
    { timeMs: 10000, text: "c" },
  ];

  it("returns last line at or before time", () => {
    expect(findActiveLineIndex(lines, 4999)).toBe(0);
    expect(findActiveLineIndex(lines, 5000)).toBe(1);
    expect(findActiveLineIndex(lines, 99999)).toBe(2);
  });
});