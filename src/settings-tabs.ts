export type SettingsTab = "general" | "bouncer" | "font" | "visualizer" | "about";

const STORAGE_KEY = "lyricglass-settings-tab";

const VALID: SettingsTab[] = ["general", "bouncer", "font", "visualizer", "about"];

export function readStoredSettingsTab(): SettingsTab {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v && VALID.includes(v as SettingsTab)) return v as SettingsTab;
  } catch {
    /* private mode */
  }
  return "general";
}

export function storeSettingsTab(tab: SettingsTab) {
  try {
    localStorage.setItem(STORAGE_KEY, tab);
  } catch {
    /* ignore */
  }
}

export function initSettingsTabs(popover: HTMLElement): SettingsTab {
  const tabBar = popover.querySelector(".settings-tab-bar");
  const body = popover.querySelector(".settings-body");
  if (!tabBar || !body) return "general";

  const tabs = tabBar.querySelectorAll<HTMLButtonElement>("[data-settings-tab]");
  const panels = body.querySelectorAll<HTMLElement>("[data-settings-panel]");

  const activate = (tab: SettingsTab) => {
    tabs.forEach((btn) => {
      const on = btn.dataset.settingsTab === tab;
      btn.classList.toggle("active", on);
      btn.setAttribute("aria-selected", String(on));
      btn.tabIndex = on ? 0 : -1;
    });
    panels.forEach((panel) => {
      const on = panel.dataset.settingsPanel === tab;
      panel.hidden = !on;
    });
    body.scrollTop = 0;
    storeSettingsTab(tab);
  };

  tabs.forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      const tab = btn.dataset.settingsTab as SettingsTab;
      if (VALID.includes(tab)) activate(tab);
    });
  });

  const initial = readStoredSettingsTab();
  activate(initial);
  return initial;
}