import {
  FONT_CATALOG,
  fontFamilyStack,
  normalizeFontId,
  optionsByCategory,
  type FontOptionMeta,
} from "./fonts/catalog";

export function buildFontPicker(
  container: HTMLElement,
  onSelect: (id: string) => void,
) {
  container.replaceChildren();
  container.setAttribute("role", "listbox");
  container.setAttribute("aria-label", "Lyrics font");

  for (const group of optionsByCategory()) {
    const heading = document.createElement("p");
    heading.className = "font-picker-category";
    heading.textContent = group.category;
    container.appendChild(heading);

    const grid = document.createElement("div");
    grid.className = "font-picker-group";

    for (const opt of group.options) {
      grid.appendChild(makeFontOptionButton(opt, onSelect));
    }
    container.appendChild(grid);
  }
}

function makeFontOptionButton(
  opt: FontOptionMeta,
  onSelect: (id: string) => void,
): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "font-picker-btn";
  btn.dataset.font = opt.id;
  btn.setAttribute("role", "option");
  btn.setAttribute("aria-pressed", "false");
  const previewStack =
    opt.id === "default" ? fontFamilyStack("default") : fontFamilyStack(opt.id);
  btn.style.fontFamily = previewStack;
  btn.textContent = opt.label;
  btn.title = opt.label;
  btn.addEventListener("click", () => onSelect(opt.id));
  return btn;
}

export function setFontPickerSelection(container: HTMLElement, fontId: string) {
  const norm = normalizeFontId(fontId);
  container.querySelectorAll<HTMLButtonElement>(".font-picker-btn").forEach((btn) => {
    const on = btn.dataset.font === norm;
    btn.setAttribute("aria-pressed", String(on));
    btn.setAttribute("aria-selected", String(on));
  });
}

export { FONT_CATALOG };