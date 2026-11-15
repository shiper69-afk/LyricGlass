import { LyricsView } from "../lyrics-view";

const INTRO_LRC = `[00:02.00] <00:02.00>One <00:02.40>two <00:02.80>three <00:03.10>end
[00:08.00] <00:08.00>Four <00:08.40>five <00:08.80>six <00:09.10>end`;

const HEBREW_GAP_LRC = `[00:00.00] <00:00.00>שלום <00:00.60>עולם <00:01.20>יפה
[00:06.00] <00:06.00>שורה <00:06.60>שניה <00:07.20>כאן`;

const root = document.getElementById("lyrics-root")!;
const view = new LyricsView(root);
view.setKaraokeBouncer("comet");

const mode = new URLSearchParams(location.search).get("mode") ?? "ltr";

if (mode === "rtl") {
  view.onLyricsResponse({
    status: "synced",
    message: "RTL gap",
    syncedLrc: HEBREW_GAP_LRC,
    instrumental: false,
    lyricsSource: "lrclib-synced",
  });
} else {
  view.onLyricsResponse({
    status: "ok",
    message: "Gap harness",
    syncedLrc: INTRO_LRC,
    instrumental: false,
  });
}

function wordCenterX(lineIndex: number, wordIndex: number): number | null {
  const linesEl = document.getElementById("lyrics-lines")!;
  const line = linesEl.querySelector<HTMLElement>(
    `.lyric-line[data-line="${lineIndex}"]`,
  );
  if (!line) return null;
  const words = line.querySelectorAll<HTMLElement>(".lyric-word");
  const word = words[wordIndex];
  if (!word) return null;
  const c = linesEl.getBoundingClientRect();
  const r = word.getBoundingClientRect();
  return r.left - c.left + r.width / 2;
}

function lastWordCenterX(lineIndex: number): number | null {
  const linesEl = document.getElementById("lyrics-lines")!;
  const line = linesEl.querySelector<HTMLElement>(
    `.lyric-line[data-line="${lineIndex}"]`,
  );
  if (!line) return null;
  const words = line.querySelectorAll<HTMLElement>(".lyric-word");
  if (words.length === 0) return null;
  const last = words[words.length - 1]!;
  const c = linesEl.getBoundingClientRect();
  const r = last.getBoundingClientRect();
  return r.left - c.left + r.width / 2;
}

function containerCenterX(): number {
  const linesEl = document.getElementById("lyrics-lines")!;
  return linesEl.clientWidth / 2;
}

let lastTick = -1;
function tickTo(ms: number) {
  if (lastTick < 0) {
    view.tick(0, 12_000);
    lastTick = 0;
  }
  for (let t = lastTick + 40; t <= ms; t += 40) {
    view.tick(t, 12_000);
    lastTick = t;
  }
  view.tick(ms, 12_000);
  lastTick = ms;
}

export interface GapHarnessApi {
  introCentered(): { ballX: number; centerX: number; delta: number };
  gapOnLastWord(lineIndex: number, playbackMs: number): {
    ballX: number;
    lastWordX: number;
    delta: number;
  };
}

const api: GapHarnessApi = {
  introCentered() {
    lastTick = -1;
    tickTo(900);
    const ball = view.getBouncerClientPosition();
    const centerX = containerCenterX();
    const ballX = ball?.x ?? 0;
    return { ballX, centerX, delta: Math.abs(ballX - centerX) };
  },
  gapOnLastWord(lineIndex, playbackMs) {
    lastTick = -1;
    tickTo(playbackMs);
    const ball = view.getBouncerClientPosition();
    const lastWordX = lastWordCenterX(lineIndex);
    const ballX = ball?.x ?? 0;
    return {
      ballX,
      lastWordX: lastWordX ?? 0,
      delta: lastWordX != null ? Math.abs(ballX - lastWordX) : 999,
    };
  },
};

(window as unknown as { __gapHarness: GapHarnessApi }).__gapHarness = api;

void wordCenterX;