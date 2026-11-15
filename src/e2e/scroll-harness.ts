import { LyricsView } from "../lyrics-view";
import { setBuiltinSyncLagOverride } from "../sync-lag";

setBuiltinSyncLagOverride(0);

const THREE_LINE_LRC = `[00:00.00] <00:00.00>One <00:00.40>two <00:00.80>three <00:01.10>end
[00:03.00] <00:03.00>Four <00:03.40>five <00:03.80>six <00:04.10>end
[00:06.00] <00:06.00>Seven <00:06.40>eight <00:06.80>nine <00:07.10>end`;

const root = document.getElementById("lyrics-root")!;
const view = new LyricsView(root);

view.onLyricsResponse({
  status: "ok",
  message: "Harness",
  syncedLrc: THREE_LINE_LRC,
  instrumental: false,
});

export interface HarnessSample {
  playbackMs: number;
  scrollY: number;
  ball: { x: number; y: number } | null;
}

export interface HarnessApi {
  tick(playbackMs: number): HarnessSample;
  runLineTransition(): HarnessSample[];
}

function sampleAt(playbackMs: number): HarnessSample {
  view.tick(playbackMs);
  return {
    playbackMs,
    scrollY: view.getScrollTranslateY(),
    ball: view.getBouncerClientPosition(),
  };
}

let lastHarnessTickMs = -1;

function tickContinuous(playbackMs: number): HarnessSample {
  if (lastHarnessTickMs < 0) {
    lastHarnessTickMs = 0;
    sampleAt(0);
  }
  const step = 45;
  for (let t = lastHarnessTickMs + step; t <= playbackMs; t += step) {
    sampleAt(t);
    lastHarnessTickMs = t;
  }
  return sampleAt(playbackMs);
}

const api: HarnessApi = {
  tick: tickContinuous,
  runLineTransition() {
    lastHarnessTickMs = -1;
    const captureFrom = 2_550;
    const captureTo = 3_050;
    const captureStep = 40;
    const out: HarnessSample[] = [];
    for (let t = captureFrom; t <= captureTo; t += captureStep) {
      out.push(tickContinuous(t));
    }
    return out;
  },
};

(window as unknown as { __harness: HarnessApi }).__harness = api;