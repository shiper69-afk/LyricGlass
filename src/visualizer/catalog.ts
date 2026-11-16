export type VisualizerKind =
  | "off"
  | "spectrum"
  | "waveform"
  | "radial"
  | "particles"
  | "aurora";

export interface VisualizerOptionMeta {
  id: VisualizerKind;
  label: string;
  blurb: string;
}

export const VISUALIZER_OPTIONS: VisualizerOptionMeta[] = [
  { id: "off", label: "Off", blurb: "No visualizer" },
  { id: "spectrum", label: "Bars", blurb: "Spectrum bars" },
  { id: "waveform", label: "Wave", blurb: "Waveform line" },
  { id: "radial", label: "Radial", blurb: "Circular spectrum" },
  { id: "particles", label: "Particles", blurb: "Beat particles" },
  { id: "aurora", label: "Aurora", blurb: "Soft glow pulse" },
];

const VALID = new Set(VISUALIZER_OPTIONS.map((o) => o.id));

export function normalizeVisualizerKind(id: string): VisualizerKind {
  const k = id.trim().toLowerCase() as VisualizerKind;
  return VALID.has(k) ? k : "off";
}