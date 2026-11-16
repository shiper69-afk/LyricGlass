import "../fonts/faces";
import { applyLyricsFont } from "../fonts/apply";
import { fontFamilyStack } from "../fonts/catalog";
import { LyricsView } from "../lyrics-view";
import { setBuiltinSyncLagOverride } from "../sync-lag";

setBuiltinSyncLagOverride(0);

const LRC = `[00:00.00] <00:00.00>Hello <00:00.50>world <00:01.00>today
[00:04.00] Line two here`;

const HEBREW_LRC = `[00:00.00] <00:00.00>שלום <00:00.60>עולם`;

const root = document.getElementById("lyrics-root")!;
const view = new LyricsView(root);
view.setKaraokeBouncer("glow-orb");

view.onLyricsResponse({
  status: "synced",
  message: "font harness",
  syncedLrc: LRC,
  instrumental: false,
});
view.tick(600, 10_000);

let currentFont = "default";
let hebrewView: LyricsView | null = null;

export interface FontHarnessApi {
  switchFont(fontId: string): Promise<{ fontId: string; ball: { x: number; y: number } | null }>;
  hebrewUsesRubikFallback(fontId: string): Promise<boolean>;
  getCurrentFont(): string;
}

async function ballOnActiveWord(): Promise<{ x: number; y: number } | null> {
  const line = root.querySelector<HTMLElement>(".lyric-line.active");
  if (!line) return view.getBouncerClientPosition();
  const sung = line.querySelector<HTMLElement>(".lyric-word.sung");
  const ball = view.getBouncerClientPosition();
  if (!sung || !ball) return ball;
  const wr = sung.getBoundingClientRect();
  const linesRect = root.querySelector("#lyrics-lines")!.getBoundingClientRect();
  const wordCenterX = wr.left + wr.width / 2 - linesRect.left;
  const ok = Math.abs(ball.x - wordCenterX) < Math.max(48, wr.width);
  return ok ? ball : null;
}

const api: FontHarnessApi = {
  async switchFont(fontId: string) {
    currentFont = await applyLyricsFont(fontId);
    await view.onLyricsFontChanged();
    view.tick(600, 10_000);
    await document.fonts.ready;
    const ball = await ballOnActiveWord();
    return { fontId: currentFont, ball };
  },
  async hebrewUsesRubikFallback(fontId: string) {
    const host = document.createElement("div");
    host.id = "hebrew-probe";
    host.innerHTML = `
      <section class="lyrics-panel" id="lyrics-root-he">
        <p class="np-header"><span id="np-title"></span><span id="np-artist"></span></p>
        <p id="lyrics-status"></p><p id="sync-offset-label"></p>
        <div id="lyrics-lines" class="lyrics-lines karaoke-on"></div>
        <div id="lyrics-plain" class="hidden"></div>
        <p id="duration-warn" class="hidden"></p>
      </section>`;
    document.body.appendChild(host);
    const heRoot = host.querySelector("#lyrics-root-he")! as HTMLElement;
    hebrewView = new LyricsView(heRoot as HTMLElement);
    await applyLyricsFont(fontId);
    await hebrewView.onLyricsFontChanged();
    hebrewView.onLyricsResponse({
      status: "synced",
      message: "he",
      syncedLrc: HEBREW_LRC,
      instrumental: false,
    });
    hebrewView.tick(400, 5000);
    await document.fonts.ready;
    await new Promise((r) => setTimeout(r, 80));
    const line = host.querySelector<HTMLElement>(".lyric-line");
    const family = line ? getComputedStyle(line).fontFamily : "";
    host.remove();
    hebrewView = null;
    const stack = fontFamilyStack(fontId);
    if (fontId === "inter" || fontId === "bebas-neue") {
      return family.toLowerCase().includes("rubik");
    }
    return stack.split(",")[0].replace(/"/g, "").trim().toLowerCase().length > 0;
  },
  getCurrentFont() {
    return currentFont;
  },
};

(window as unknown as { __fontHarness: FontHarnessApi }).__fontHarness = api;