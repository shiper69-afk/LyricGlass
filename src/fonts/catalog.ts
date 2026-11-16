export type FontCategory =
  | "Default"
  | "Clean"
  | "Rounded"
  | "Bold display"
  | "Classic"
  | "Handwritten"
  | "Retro";

export interface FontOptionMeta {
  id: string;
  label: string;
  category: FontCategory;
  /** CSS font-family name matching bundled @font-face */
  primaryFamily: string;
  hasHebrew: boolean;
}

export const HEBREW_FALLBACK_FAMILY = "Rubik";

export const DEFAULT_FONT_ID = "default";

export const FONT_CATALOG: FontOptionMeta[] = [
  { id: "default", label: "Default", category: "Default", primaryFamily: "", hasHebrew: true },
  { id: "rubik", label: "Rubik", category: "Clean", primaryFamily: "Rubik", hasHebrew: true },
  { id: "heebo", label: "Heebo", category: "Clean", primaryFamily: "Heebo", hasHebrew: true },
  { id: "assistant", label: "Assistant", category: "Clean", primaryFamily: "Assistant", hasHebrew: true },
  { id: "inter", label: "Inter", category: "Clean", primaryFamily: "Inter", hasHebrew: false },
  { id: "varela-round", label: "Varela Round", category: "Rounded", primaryFamily: "Varela Round", hasHebrew: true },
  { id: "nunito", label: "Nunito", category: "Rounded", primaryFamily: "Nunito", hasHebrew: false },
  { id: "secular-one", label: "Secular One", category: "Bold display", primaryFamily: "Secular One", hasHebrew: true },
  { id: "bebas-neue", label: "Bebas Neue", category: "Bold display", primaryFamily: "Bebas Neue", hasHebrew: false },
  { id: "anton", label: "Anton", category: "Bold display", primaryFamily: "Anton", hasHebrew: false },
  { id: "frank-ruhl-libre", label: "Frank Ruhl Libre", category: "Classic", primaryFamily: "Frank Ruhl Libre", hasHebrew: true },
  { id: "playfair-display", label: "Playfair Display", category: "Classic", primaryFamily: "Playfair Display", hasHebrew: false },
  { id: "amatic-sc", label: "Amatic SC", category: "Handwritten", primaryFamily: "Amatic SC", hasHebrew: true },
  { id: "caveat", label: "Caveat", category: "Handwritten", primaryFamily: "Caveat", hasHebrew: false },
  { id: "pacifico", label: "Pacifico", category: "Handwritten", primaryFamily: "Pacifico", hasHebrew: false },
  { id: "press-start-2p", label: "Press Start 2P", category: "Retro", primaryFamily: "Press Start 2P", hasHebrew: false },
  { id: "monoton", label: "Monoton", category: "Retro", primaryFamily: "Monoton", hasHebrew: false },
  { id: "bungee", label: "Bungee", category: "Retro", primaryFamily: "Bungee", hasHebrew: false },
];

export const FONT_IDS = FONT_CATALOG.map((f) => f.id);

const ID_SET = new Set(FONT_IDS);

export function normalizeFontId(id: string | null | undefined): string {
  const t = (id ?? "").trim().toLowerCase();
  return ID_SET.has(t) ? t : DEFAULT_FONT_ID;
}

const SYSTEM_STACK = '"Segoe UI", system-ui, sans-serif';

export function fontFamilyStack(id: string): string {
  const norm = normalizeFontId(id);
  if (norm === DEFAULT_FONT_ID) {
    return SYSTEM_STACK;
  }
  const opt = FONT_CATALOG.find((f) => f.id === norm);
  if (!opt?.primaryFamily) return SYSTEM_STACK;
  if (opt.hasHebrew) {
    return `"${opt.primaryFamily}", ${SYSTEM_STACK}`;
  }
  return `"${opt.primaryFamily}", "${HEBREW_FALLBACK_FAMILY}", ${SYSTEM_STACK}`;
}

/** Families to probe with document.fonts.load after a change. */
export function fontLoadFamilies(id: string): string[] {
  const norm = normalizeFontId(id);
  if (norm === DEFAULT_FONT_ID) return [];
  const opt = FONT_CATALOG.find((f) => f.id === norm);
  if (!opt) return [];
  const names = [opt.primaryFamily];
  if (!opt.hasHebrew) names.push(HEBREW_FALLBACK_FAMILY);
  return names;
}

export function optionsByCategory(): { category: FontCategory; options: FontOptionMeta[] }[] {
  const order: FontCategory[] = [
    "Default",
    "Clean",
    "Rounded",
    "Bold display",
    "Classic",
    "Handwritten",
    "Retro",
  ];
  return order.map((category) => ({
    category,
    options: FONT_CATALOG.filter((f) => f.category === category),
  }));
}