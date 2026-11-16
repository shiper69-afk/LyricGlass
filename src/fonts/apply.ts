import {
  DEFAULT_FONT_ID,
  fontFamilyStack,
  fontLoadFamilies,
  normalizeFontId,
} from "./catalog";

export type LyricsFontChangeHook = () => void | Promise<void>;

let afterFontChange: LyricsFontChangeHook | null = null;

export function setLyricsFontChangeHook(hook: LyricsFontChangeHook | null) {
  afterFontChange = hook;
}

export function currentLyricsFontStack(): string {
  const stack = getComputedStyle(document.documentElement).getPropertyValue(
    "--lyrics-font-family",
  );
  return stack.trim() || fontFamilyStack(DEFAULT_FONT_ID);
}

export async function waitForBundledFontFamilies(
  fontId: string,
): Promise<void> {
  const families = fontLoadFamilies(fontId);
  for (const family of families) {
    try {
      await document.fonts.load(`400 16px "${family}"`);
      await document.fonts.load(`700 16px "${family}"`);
    } catch {
      /* jsdom or missing */
    }
  }
  if (typeof document !== "undefined" && document.fonts?.ready) {
    await document.fonts.ready;
  }
}

export async function applyLyricsFont(fontId: string): Promise<string> {
  const id = normalizeFontId(fontId);
  const stack = fontFamilyStack(id);
  document.documentElement.style.setProperty("--lyrics-font-family", stack);
  await waitForBundledFontFamilies(id);
  await afterFontChange?.();
  return id;
}