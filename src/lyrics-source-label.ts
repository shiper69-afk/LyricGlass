/** User-facing labels for the active lyrics timing source. */
export function lyricsProviderLabel(
  source?: string | null,
  wordLevel?: boolean,
): string {
  const s = (source ?? "").trim().toLowerCase();
  if (s === "netease-yrc" || (s.startsWith("netease") && wordLevel)) {
    return "NetEase · word";
  }
  if (s === "netease-synced" || s === "netease") {
    return "NetEase";
  }
  if (s === "shironet") {
    return "Shironet";
  }
  if (s === "lrclib-synced" && wordLevel) {
    return "LRCLIB";
  }
  if (s === "lrclib-synced" || s === "lrclib-plain") {
    return wordLevel ? "LRCLIB" : "estimated";
  }
  if (s === "synced" && wordLevel) {
    return "LRCLIB";
  }
  if (s.startsWith("lrclib")) {
    return wordLevel ? "LRCLIB" : "estimated";
  }
  return "";
}

export const LYRICS_SOURCE_OPTIONS = [
  { id: "auto", label: "Auto" },
  { id: "lrclib", label: "LRCLIB only" },
  { id: "netease", label: "NetEase only" },
] as const;

export type LyricsSourceMode = (typeof LYRICS_SOURCE_OPTIONS)[number]["id"];