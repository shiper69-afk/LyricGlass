export const BOUNCER_IDS = [
  "glow-orb",
  "music-note",
  "spark",
  "heart",
  "glass-drop",
  "firefly",
  "comet",
  "eq-pulse",
  "mic",
  "paper-plane",
  "pixel",
  "crescent",
] as const;

export type BouncerId = (typeof BOUNCER_IDS)[number];

export const DEFAULT_BOUNCER_ID: BouncerId = "glow-orb";

/** Bouncers whose art trails left; mirror in RTL. */
export const RTL_MIRROR_BOUNCERS = new Set<BouncerId>(["comet", "paper-plane"]);

export interface BouncerMeta {
  id: BouncerId;
  name: string;
}

export const BOUNCER_META: BouncerMeta[] = [
  { id: "glow-orb", name: "Glow Orb" },
  { id: "music-note", name: "Music Note" },
  { id: "spark", name: "Spark" },
  { id: "heart", name: "Heart" },
  { id: "glass-drop", name: "Glass Drop" },
  { id: "firefly", name: "Firefly" },
  { id: "comet", name: "Comet" },
  { id: "eq-pulse", name: "EQ Pulse" },
  { id: "mic", name: "Mic" },
  { id: "paper-plane", name: "Paper Plane" },
  { id: "pixel", name: "Pixel" },
  { id: "crescent", name: "Crescent" },
];

export function isBouncerId(raw: string): raw is BouncerId {
  return (BOUNCER_IDS as readonly string[]).includes(raw);
}

export function normalizeBouncerId(raw: string | null | undefined): BouncerId {
  const s = (raw ?? "").trim().toLowerCase();
  return isBouncerId(s) ? s : DEFAULT_BOUNCER_ID;
}