import { LyricsView } from "../lyrics-view";

const HEBREW_LRC = `[00:00.00] <00:00.00>שלום <00:00.60>עולם <00:01.20>יפה
[00:04.00] <00:04.00>שורה <00:04.60>שניה <00:05.20>כאן`;

const root = document.getElementById("lyrics-root")!;
const view = new LyricsView(root);
view.setKaraokeBouncer("comet");

view.onLyricsResponse({
  status: "synced",
  message: "Hebrew harness",
  syncedLrc: HEBREW_LRC,
  instrumental: false,
  lyricsSource: "lrclib-synced",
});
view.tick(400, 8_000);

export interface HebrewSample {
  playbackMs: number;
  ball: { x: number; y: number } | null;
  activeDir: string;
  ballWordApprox: number;
}

export interface HebrewHarnessApi {
  runWordHop(): HebrewSample[];
  activeLineDirection(): string;
}

let lastTickMs = -1;

function tickContinuous(ms: number) {
  if (lastTickMs < 0) {
    lastTickMs = 0;
    view.tick(0, 8000);
  }
  for (let t = lastTickMs + 40; t <= ms; t += 40) {
    view.tick(t, 8000);
    lastTickMs = t;
  }
  view.tick(ms, 8000);
}

const api: HebrewHarnessApi = {
  runWordHop() {
    lastTickMs = -1;
    const out: HebrewSample[] = [];
    for (let t = 0; t <= 900; t += 80) {
      tickContinuous(t);
      const line = document.querySelector<HTMLElement>(".lyric-line.active");
      out.push({
        playbackMs: t,
        ball: view.getBouncerClientPosition(),
        activeDir: line ? getComputedStyle(line).direction : "",
        ballWordApprox: countSungWords(line),
      });
    }
    return out;
  },
  activeLineDirection() {
    const line = document.querySelector<HTMLElement>(".lyric-line.active");
    return line ? getComputedStyle(line).direction : "";
  },
};

function countSungWords(line: HTMLElement | null): number {
  if (!line) return -1;
  return line.querySelectorAll(".lyric-word.sung").length;
}

(window as unknown as { __hebrewHarness: HebrewHarnessApi }).__hebrewHarness =
  api;