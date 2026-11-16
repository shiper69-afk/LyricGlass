export type VisualizerKind =
  | "off"
  | "spectrum"
  | "waveform"
  | "radial"
  | "particles"
  | "aurora"
  | "starfield"
  | "mirror"
  | "tunnel"
  | "matrix"
  | "terrain";

export interface VisualizerOptionMeta {
  id: VisualizerKind;
  label: string;
  blurb: string;
  /** Small SVG preview for the settings tile */
  icon: string;
}

const ICON_OFF =
  '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4"/></svg>';
const ICON_BARS =
  '<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="14" width="3" height="7" fill="#00f5ff"/><rect x="8" y="10" width="3" height="11" fill="#ff3cc8"/><rect x="13" y="12" width="3" height="9" fill="#a05aff"/><rect x="18" y="8" width="3" height="13" fill="#ff8c32"/></svg>';

export const VISUALIZER_OPTIONS: VisualizerOptionMeta[] = [
  { id: "off", label: "Off", blurb: "No visualizer", icon: ICON_OFF },
  { id: "spectrum", label: "Bars", blurb: "Spectrum bars", icon: ICON_BARS },
  {
    id: "waveform",
    label: "Wave",
    blurb: "Waveform line",
    icon: '<svg viewBox="0 0 24 24"><path d="M2 14 Q6 8 10 14 T18 10 T22 14" fill="none" stroke="#00f5ff" stroke-width="2"/></svg>',
  },
  {
    id: "radial",
    label: "Radial",
    blurb: "Circular spectrum",
    icon: '<svg viewBox="0 0 24 24"><g stroke="#a05aff" stroke-width="1.5"><line x1="12" y1="12" x2="12" y2="4"/><line x1="12" y1="12" x2="19" y2="16"/><line x1="12" y1="12" x2="5" y2="16"/></g></svg>',
  },
  {
    id: "particles",
    label: "Particles",
    blurb: "Beat particles",
    icon: '<svg viewBox="0 0 24 24"><circle cx="8" cy="16" r="2" fill="#ff3cc8"/><circle cx="14" cy="10" r="2.5" fill="#00f5ff"/><circle cx="18" cy="18" r="1.5" fill="#ff8c32"/></svg>',
  },
  {
    id: "aurora",
    label: "Aurora",
    blurb: "Soft glow pulse",
    icon: '<svg viewBox="0 0 24 24"><defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#a05aff" stop-opacity="0"/><stop offset="1" stop-color="#00f5ff" stop-opacity="0.8"/></linearGradient></defs><rect width="24" height="24" fill="url(#g)"/></svg>',
  },
  {
    id: "starfield",
    label: "Warp",
    blurb: "Starfield rush on beats",
    icon: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="1.5" fill="#fff"/><line x1="4" y1="6" x2="8" y2="10" stroke="#00f5ff" stroke-width="1"/><line x1="16" y1="8" x2="20" y2="4" stroke="#ff3cc8" stroke-width="1"/></svg>',
  },
  {
    id: "mirror",
    label: "Mirror",
    blurb: "Center-split spectrum",
    icon: '<svg viewBox="0 0 24 24"><rect x="5" y="11" width="2" height="4" fill="#00f5ff"/><rect x="9" y="9" width="2" height="6" fill="#ff3cc8"/><rect x="13" y="7" width="2" height="10" fill="#a05aff"/><line x1="2" y1="12" x2="22" y2="12" stroke="currentColor" opacity="0.3"/></svg>',
  },
  {
    id: "tunnel",
    label: "Tunnel",
    blurb: "Pulsing ring tunnel",
    icon: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" fill="none" stroke="#ff8c32" stroke-width="1"/><circle cx="12" cy="12" r="6" fill="none" stroke="#a05aff" stroke-width="1"/><circle cx="12" cy="12" r="3" fill="none" stroke="#00f5ff" stroke-width="1"/></svg>',
  },
  {
    id: "matrix",
    label: "Matrix",
    blurb: "Band-driven code rain",
    icon: '<svg viewBox="0 0 24 24"><text x="4" y="8" fill="#50ffb0" font-size="6" font-family="monospace">01</text><text x="12" y="14" fill="#50ffb0" font-size="6" font-family="monospace">10</text><text x="18" y="20" fill="#50ffb0" font-size="6" font-family="monospace">11</text></svg>',
  },
  {
    id: "terrain",
    label: "Terrain",
    blurb: "Perspective mountain wave",
    icon: '<svg viewBox="0 0 24 24"><polygon points="2,20 8,12 14,16 22,8 22,20" fill="#a05aff" opacity="0.5"/></svg>',
  },
];

const VALID = new Set(VISUALIZER_OPTIONS.map((o) => o.id));

export const VISUALIZER_KIND_COUNT = VISUALIZER_OPTIONS.length;

export function normalizeVisualizerKind(id: string): VisualizerKind {
  const k = id.trim().toLowerCase() as VisualizerKind;
  return VALID.has(k) ? k : "off";
}