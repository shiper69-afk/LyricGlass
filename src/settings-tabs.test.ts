/**
 * @vitest-environment jsdom
 */
import { describe, expect, it, beforeEach } from "vitest";
import {
  initSettingsTabs,
  readStoredSettingsTab,
  storeSettingsTab,
} from "./settings-tabs";

describe("settings-tabs", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("restores last tab from localStorage", () => {
    storeSettingsTab("font");
    document.body.innerHTML = `
      <div id="settings-popover">
        <div class="settings-tab-bar">
          <button data-settings-tab="general"></button>
          <button data-settings-tab="bouncer"></button>
          <button data-settings-tab="font"></button>
        </div>
        <div class="settings-body">
          <div data-settings-panel="general"></div>
          <div data-settings-panel="bouncer" hidden></div>
          <div data-settings-panel="font" hidden></div>
        </div>
      </div>`;
    const tab = initSettingsTabs(document.getElementById("settings-popover")!);
    expect(tab).toBe("font");
    expect(readStoredSettingsTab()).toBe("font");
    const fontPanel = document.querySelector<HTMLElement>(
      '[data-settings-panel="font"]',
    );
    expect(fontPanel?.hidden).toBe(false);
  });
});