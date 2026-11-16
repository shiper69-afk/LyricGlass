import {
  normalizeVisualizerKind,
  VISUALIZER_OPTIONS,
  type VisualizerKind,
} from "./visualizer/catalog";

export function buildVisualizerPicker(
  container: HTMLElement,
  onSelect: (id: VisualizerKind) => void,
) {
  container.replaceChildren();
  container.setAttribute("role", "listbox");
  container.setAttribute("aria-label", "Music visualizer");

  const grid = document.createElement("div");
  grid.className = "visualizer-picker-grid";

  for (const opt of VISUALIZER_OPTIONS) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "visualizer-picker-tile";
    btn.dataset.visualizer = opt.id;
    btn.setAttribute("role", "option");
    btn.setAttribute("aria-pressed", "false");
    btn.title = opt.blurb;
    const icon = document.createElement("span");
    icon.className = "visualizer-picker-icon";
    icon.innerHTML = opt.icon;
    const title = document.createElement("span");
    title.className = "visualizer-picker-label";
    title.textContent = opt.label;
    btn.append(icon, title);
    btn.addEventListener("click", () => onSelect(opt.id));
    grid.appendChild(btn);
  }
  container.appendChild(grid);
}

export function setVisualizerPickerSelection(
  container: HTMLElement,
  kind: string,
) {
  const norm = normalizeVisualizerKind(kind);
  container.querySelectorAll<HTMLButtonElement>(".visualizer-picker-tile").forEach((btn) => {
    const on = btn.dataset.visualizer === norm;
    btn.classList.toggle("active", on);
    btn.setAttribute("aria-pressed", String(on));
    btn.setAttribute("aria-selected", String(on));
  });
}