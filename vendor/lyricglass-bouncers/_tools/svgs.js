'use strict';

const DEFAULT_COLOR = '#FF3F4C';

function starPath(cx, cy, n, outerR, innerR, rotDeg) {
  const pts = [];
  const step = Math.PI / n;
  const rot = (rotDeg * Math.PI) / 180;
  for (let i = 0; i < n * 2; i++) {
    const r = i % 2 === 0 ? outerR : innerR;
    const a = rot + i * step;
    pts.push(`${(cx + Math.cos(a) * r).toFixed(2)},${(cy + Math.sin(a) * r).toFixed(2)}`);
  }
  return `M${pts.join('L')}Z`;
}

function wrap({ id, name, body }) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" fill="none"
     color="${DEFAULT_COLOR}" role="img" aria-label="${name}">
  <title>LyricGlass ${name}</title>
  <desc>Karaoke bouncer for LyricGlass. Recolor by setting the CSS color property or, when inlined, --lg-bouncer on a parent (the host should also set color: var(--lg-bouncer)). Body uses currentColor; highlights are a white sheen and a dark overlay so any user colour still reads. When used as an image, the color="${DEFAULT_COLOR}" presentation attribute is the default.</desc>
  <g class="lg-bouncer lg-${id}">
${body}
  </g>
</svg>
`;
}

const bouncers = [
  {
    id: 'glow-orb',
    name: 'Glow Orb',
    description: 'The classic karaoke ball: a soft glowing orb that hops along the sung word.',
    motion: 'Classic squash-and-stretch. Stretches tall at the top of the hop, flattens on landing, then settles. A soft glow pulses slightly louder at the apex. Optional contact shadow scales out as it leaves the word.',
    colorSupport: 'Fully recolorable. Single currentColor fill with a fixed white specular highlight and a dark shade overlay (works on any hue). Glow uses currentColor at low opacity.',
    recorable: true,
    colorHow: 'single fill + auto white highlight + dark overlay',
    body: `
    <circle class="lg-glow" cx="12" cy="12.1" r="8.8" fill="currentColor" opacity="0.16"/>
    <circle class="lg-body" cx="12" cy="12.1" r="7.15" fill="currentColor"/>
    <ellipse class="lg-shade" cx="13.2" cy="14.55" rx="5.15" ry="4.2" fill="#041218" opacity="0.28"/>
    <ellipse class="lg-highlight" cx="9.55" cy="9.55" rx="3.15" ry="2.25" fill="#FFFFFF" opacity="0.58"/>
    <ellipse class="lg-highlight-core" cx="8.7" cy="8.8" rx="1.15" ry="0.8" fill="#FFFFFF" opacity="0.85"/>`,
  },
  {
    id: 'music-note',
    name: 'Music Note',
    description: 'A chunky eighth note that marks the beat as each word is sung.',
    motion: 'Tilts back about 18° while airborne, tips forward on landing, and lightly squashes through the note head. The flag can give a tiny delayed follow-through (it lags the stem by a frame or two).',
    colorSupport: 'Fully recolorable. The whole note (head, stem, flag) is currentColor. A white sheen sits on the head.',
    recorable: true,
    colorHow: 'single fill + auto white highlight on the head',
    body: `
    <circle class="lg-glow" cx="11.2" cy="13.2" r="9.0" fill="currentColor" opacity="0.12"/>
    <path class="lg-flag" d="M14.9 4.1 C21.15 4.55 22.9 9.2 20.15 13.0 C17.5 10.2 15.95 8.55 14.9 8.85 Z" fill="currentColor"/>
    <rect class="lg-stem" x="12.72" y="4.1" width="2.4" height="12.7" rx="0.9" fill="currentColor"/>
    <g class="lg-head" transform="rotate(-24 9.15 16.75)">
      <ellipse class="lg-body" cx="9.15" cy="16.75" rx="4.8" ry="3.5" fill="currentColor"/>
      <ellipse class="lg-highlight" cx="8.1" cy="15.7" rx="2.2" ry="1.5" fill="#FFFFFF" opacity="0.42"/>
    </g>`,
  },
  {
    id: 'spark',
    name: 'Spark',
    description: 'A four-point sparkle star that twinkles over the active lyric.',
    motion: 'Slowly rotates (about 20–30° per hop) and twinkles with a short scale pulse at the apex. On landing it squashes slightly along the vertical points. A one-frame “flash” (scale 1.15) can mark the start of each word.',
    colorSupport: 'Fully recolorable. Star body is currentColor; a white sheen sits on the upper point. Glow uses currentColor.',
    recorable: true,
    colorHow: 'single fill + auto white highlight',
    body: `
    <circle class="lg-glow" cx="12" cy="12" r="9.2" fill="currentColor" opacity="0.14"/>
    <path class="lg-body" d="${starPath(12, 12, 4, 9.45, 3.9, -90)}" fill="currentColor"/>
    <path class="lg-highlight" d="${starPath(12, 9.3, 4, 3.35, 1.15, -90)}" fill="#FFFFFF" opacity="0.42"/>
    <circle class="lg-highlight-core" cx="12" cy="9.1" r="1.05" fill="#FFFFFF" opacity="0.8"/>`,
  },
  {
    id: 'heart',
    name: 'Heart',
    description: 'A bold heart that sits on the sung word — clear even at tray-like sizes.',
    motion: 'Pulses (a quick beat-scale) as it lands, with a mild squash in the lower tip. Slight sideways tilt in the air, then a spring back to upright. Can double-pulse on a held note.',
    colorSupport: 'Fully recolorable. Body is currentColor with a white sheen on the left lobe and a dark overlay in the lower right.',
    recorable: true,
    colorHow: 'single fill + auto white highlight + dark overlay',
    body: `
    <circle class="lg-glow" cx="12" cy="13" r="9.0" fill="currentColor" opacity="0.14"/>
    <path class="lg-body" fill="currentColor" d="M12 20.55 C12 20.55 3.45 14.7 3.45 9.2 C3.45 6.15 5.75 4.15 8.55 4.15 C10.25 4.15 11.45 5.05 12 6.4 C12.55 5.05 13.75 4.15 15.45 4.15 C18.25 4.15 20.55 6.15 20.55 9.2 C20.55 14.7 12 20.55 12 20.55 Z"/>
    <path class="lg-shade" fill="#041218" opacity="0.2" d="M12 20.55 C12 20.55 16.8 16.4 19.2 12.2 C18.7 16.2 15.6 18.9 12 20.55 Z"/>
    <ellipse class="lg-highlight" cx="8.7" cy="8.05" rx="2.45" ry="1.85" fill="#FFFFFF" opacity="0.48"/>
    <ellipse class="lg-highlight-core" cx="8.05" cy="7.35" rx="0.95" ry="0.7" fill="#FFFFFF" opacity="0.8"/>`,
  },
  {
    id: 'glass-drop',
    name: 'Glass Drop',
    description: 'A glassy teardrop, like a bead of light resting on the lyric line.',
    motion: 'Stretches into a taller drop at the apex, then a wet squash on landing (wide and short) with a short wobble (horizontal shear) as it settles. The highlight can slide up the left edge during the hop.',
    colorSupport: 'Fully recolorable. Body is currentColor; the “glass” comes from a white rim sheen and a sliding specular highlight, not a fixed teal.',
    recorable: true,
    colorHow: 'single fill + auto white glass sheen + dark overlay',
    body: `
    <ellipse class="lg-glow" cx="12" cy="14.4" rx="8.6" ry="7.6" fill="currentColor" opacity="0.14"/>
    <path class="lg-body" fill="currentColor" d="M12 3.05 C13.35 5.7 20.05 11.55 20.05 15.85 C20.05 20.25 16.55 22.05 12 22.05 C7.45 22.05 3.95 20.25 3.95 15.85 C3.95 11.55 10.65 5.7 12 3.05 Z"/>
    <path class="lg-shade" fill="#041218" opacity="0.22" d="M14.8 13.6 C17.6 15.6 17.9 18.6 15.4 20.5 C18.6 19.2 19.6 16.2 18.2 13.8 C17.2 13.1 15.8 13.1 14.8 13.6 Z"/>
    <path class="lg-highlight" fill="#FFFFFF" opacity="0.5" d="M9.0 8.2 C8.15 10.5 7.55 13.2 7.85 15.3 C8.05 13.0 8.7 10.3 10.35 7.7 C10.0 7.7 9.45 7.85 9.0 8.2 Z"/>
    <ellipse class="lg-highlight-core" cx="9.35" cy="9.55" rx="1.25" ry="1.7" fill="#FFFFFF" opacity="0.8" transform="rotate(-18 9.35 9.55)"/>`,
  },
  {
    id: 'firefly',
    name: 'Firefly',
    description: 'A tiny lantern bug: glowing abdomen, faint glass wings, and a small head.',
    motion: 'Hover more than bounce — a softer landing with less squash, a constant idle bob, and a glow that pulses independently of the hop. Wings flutter with a short scale/rotate loop. On a held note it can hover in place and pulse.',
    colorSupport: 'Fully recolorable. Body, head, glow and wings use currentColor (wings at 0.7 opacity so they stay visible on light glass). A white sheen sits on each wing and on the abdomen.',
    recorable: true,
    colorHow: 'currentColor body, head and wings + auto white sheen',
    body: `
    <circle class="lg-glow" cx="12" cy="13.6" r="8.2" fill="currentColor" opacity="0.14"/>
    <ellipse class="lg-wing lg-wing-l" cx="4.15" cy="8.85" rx="6.0" ry="2.55" transform="rotate(-32 4.15 8.85)" fill="currentColor" opacity="0.75"/>
    <ellipse class="lg-wing lg-wing-l-sheen" cx="4.15" cy="8.85" rx="3.6" ry="1.15" transform="rotate(-32 4.15 8.85)" fill="#FFFFFF" opacity="0.38"/>
    <ellipse class="lg-wing lg-wing-r" cx="19.85" cy="8.85" rx="6.0" ry="2.55" transform="rotate(32 19.85 8.85)" fill="currentColor" opacity="0.75"/>
    <ellipse class="lg-wing lg-wing-r-sheen" cx="19.85" cy="8.85" rx="3.6" ry="1.15" transform="rotate(32 19.85 8.85)" fill="#FFFFFF" opacity="0.38"/>
    <circle class="lg-body" cx="12" cy="14.05" r="6.15" fill="currentColor"/>
    <ellipse class="lg-shade" cx="12.9" cy="16.0" rx="4.0" ry="3.1" fill="#041218" opacity="0.22"/>
    <ellipse class="lg-highlight" cx="10.05" cy="12.05" rx="2.55" ry="1.85" fill="#FFFFFF" opacity="0.55"/>
    <ellipse class="lg-head" cx="12" cy="7.15" rx="2.35" ry="2.15" fill="currentColor"/>
    <ellipse cx="12" cy="6.7" rx="1.05" ry="0.8" fill="#FFFFFF" opacity="0.42"/>`,
  },
  {
    id: 'comet',
    name: 'Comet',
    description: 'A bright nucleus with a trailing tail that streams behind the hop.',
    motion: 'The nucleus uses the same squash-and-stretch as Glow Orb. The tail always streams opposite the hop direction: long and loose in the air, compressed or tucked on landing. In RTL lyrics, flip the tail with scaleX(-1) so it still trails behind.',
    colorSupport: 'Fully recolorable. Nucleus and tail are currentColor; tail pieces fade by opacity. Nucleus gets the standard white highlight and dark overlay.',
    recorable: true,
    colorHow: 'single fill; tail is currentColor at stepped opacities + auto white highlight on the nucleus',
    body: `
    <path class="lg-tail lg-tail-3" d="M11.6 10.8 L1.5 2.0 L6.8 7.6 L12.6 12.6 Z" fill="currentColor" opacity="0.34"/>
    <path class="lg-tail lg-tail-2" d="M11.4 13.6 L1.9 5.4 L7.4 9.0 L12.8 14.0 Z" fill="currentColor" opacity="0.55"/>
    <path class="lg-tail lg-tail-1" d="M12.8 12.0 L2.8 3.0 L8.0 8.5 L13.6 13.8 Z" fill="currentColor" opacity="0.9"/>
    <circle class="lg-glow" cx="14.35" cy="14.5" r="7.2" fill="currentColor" opacity="0.16"/>
    <circle class="lg-body" cx="14.35" cy="14.5" r="6.35" fill="currentColor"/>
    <ellipse class="lg-shade" cx="15.5" cy="16.55" rx="4.2" ry="3.4" fill="#041218" opacity="0.25"/>
    <ellipse class="lg-highlight" cx="12.3" cy="12.4" rx="2.55" ry="1.85" fill="#FFFFFF" opacity="0.58"/>
    <ellipse class="lg-highlight-core" cx="11.65" cy="11.8" rx="0.9" ry="0.65" fill="#FFFFFF" opacity="0.85"/>`,
  },
  {
    id: 'eq-pulse',
    name: 'EQ Pulse',
    description: 'Three rounded equalizer bars that bounce as a little meter on the word.',
    motion: 'The group hops as one, but each bar independently scales in height (transform-origin at the bottom) on the beat. On landing the whole meter squashes flatter. On a held note the bars can keep pulsing in place without hopping.',
    colorSupport: 'Fully recolorable. All three bars are currentColor at slightly different opacities so the meter still reads as a set. Top-cap highlights are white.',
    recorable: true,
    colorHow: 'single fill at stepped opacities + auto white cap highlights',
    body: `
    <g class="lg-glow" fill="currentColor" opacity="0.16">
      <rect x="3.25" y="9.1" width="4.7" height="12.2" rx="2.35"/>
      <rect x="9.65" y="4.2" width="4.7" height="17.1" rx="2.35"/>
      <rect x="16.05" y="11.5" width="4.7" height="9.8" rx="2.35"/>
    </g>
    <rect class="lg-bar lg-bar-1" x="3.5" y="9.55" width="4.2" height="11.25" rx="2.1" fill="currentColor" opacity="0.78"/>
    <rect class="lg-bar lg-bar-2" x="9.9" y="4.7" width="4.2" height="16.1" rx="2.1" fill="currentColor"/>
    <rect class="lg-bar lg-bar-3" x="16.3" y="11.95" width="4.2" height="8.85" rx="2.1" fill="currentColor" opacity="0.62"/>
    <rect x="4.25" y="10.1" width="2.7" height="2.35" rx="1.15" fill="#FFFFFF" opacity="0.38"/>
    <rect x="10.65" y="5.25" width="2.7" height="2.5" rx="1.15" fill="#FFFFFF" opacity="0.42"/>
    <rect x="17.05" y="12.5" width="2.7" height="2.15" rx="1.15" fill="#FFFFFF" opacity="0.38"/>`,
  },
  {
    id: 'mic',
    name: 'Mic',
    description: 'A tiny handheld microphone — the karaoke object itself, hopping the line.',
    motion: 'Swings like a short pendulum (about ±16°) while airborne, then squashes through the capsule on landing. A very small “sound ring” can flash around the grille at the start of a word.',
    colorSupport: 'Fully recolorable. Capsule, handle and ferrule are currentColor. A dark grille band and white sheen are overlays, so any user colour still reads as a mic.',
    recorable: true,
    colorHow: 'single fill + auto white highlight + dark grille overlay',
    body: `
    <g class="lg-mic-rig" transform="rotate(-8 12 12)">
      <circle class="lg-glow" cx="12" cy="11.2" r="8.8" fill="currentColor" opacity="0.12"/>
      <rect class="lg-handle" x="10.2" y="14.05" width="3.6" height="7.3" rx="1.5" fill="currentColor"/>
      <rect class="lg-ferrule" x="9.25" y="13.3" width="5.5" height="2.05" rx="0.7" fill="currentColor"/>
      <rect x="9.25" y="13.3" width="5.5" height="2.05" rx="0.7" fill="#041218" opacity="0.22"/>
      <rect class="lg-body" x="7.1" y="2.45" width="9.8" height="12.15" rx="4.9" fill="currentColor"/>
      <ellipse class="lg-shade" cx="13.15" cy="10.35" rx="3.65" ry="3.2" fill="#041218" opacity="0.22"/>
      <rect class="lg-grille" x="8.45" y="6.65" width="7.1" height="2.5" rx="1.25" fill="#041218" opacity="0.24"/>
      <ellipse class="lg-highlight" cx="10.15" cy="5.45" rx="2.45" ry="2.05" fill="#FFFFFF" opacity="0.48"/>
    </g>`,
  },
  {
    id: 'paper-plane',
    name: 'Paper Plane',
    description: 'A folded paper dart that banks from word to word like a short flight.',
    motion: 'Banks (rolls) into the hop, glides with the nose slightly up, then dips the nose and flattens on landing. Travel between words can include a short horizontal glide, not only a vertical bounce. In RTL, flip with scaleX(-1).',
    colorSupport: 'Fully recolorable. Both facets are currentColor; the fold is a dark overlay on the lower wing and a white sheen on the upper wing, so the paper-fold reads in any colour.',
    recorable: true,
    colorHow: 'single fill + dark fold overlay + auto white sheen',
    body: `
    <path class="lg-glow" d="M1.6 11.2 L22.2 3.5 L9.2 22.2 Z" fill="currentColor" opacity="0.14"/>
    <path class="lg-body" d="M2.85 11.15 L21.4 4.45 L11.15 13.55 Z" fill="currentColor"/>
    <path class="lg-wing" d="M11.15 13.55 L21.4 4.45 L9.8 21.2 Z" fill="currentColor"/>
    <path class="lg-shade" d="M11.15 13.55 L21.4 4.45 L9.8 21.2 Z" fill="#041218" opacity="0.24"/>
    <path class="lg-highlight" d="M5.15 10.9 L16.7 6.55 L11.15 13.55 Z" fill="#FFFFFF" opacity="0.3"/>`,
  },
  {
    id: 'pixel',
    name: 'Pixel',
    description: 'A chunky square pixel with a 1-bit highlight — like a cursor on the lyric.',
    motion: 'Snappy, not fluid. The hop can be stepped (a few discrete frames) rather than eased. On landing it flattens into a wider rectangle for one frame, then pops back. Optional 1-frame “blink” (the highlight flashes) at the start of each word.',
    colorSupport: 'Fully recolorable. The square is currentColor with a darker bottom/right bevel overlay and a white 1-bit highlight pixel in the corner.',
    recorable: true,
    colorHow: 'single fill + dark bevel overlay + auto white highlight pixel',
    body: `
    <rect class="lg-glow" x="3.5" y="3.5" width="17.0" height="17.0" rx="2.4" fill="currentColor" opacity="0.14"/>
    <rect class="lg-body" x="4.5" y="4.5" width="15" height="15" rx="1.7" fill="currentColor"/>
    <rect class="lg-shade" x="4.5" y="16.2" width="15" height="3.3" fill="#041218" opacity="0.24"/>
    <rect x="16.3" y="4.5" width="3.2" height="15" fill="#041218" opacity="0.12"/>
    <rect class="lg-highlight" x="6.5" y="6.5" width="5.1" height="5.1" rx="0.55" fill="#FFFFFF" opacity="0.5"/>
    <rect class="lg-highlight-core" x="6.5" y="6.5" width="2.3" height="2.3" rx="0.3" fill="#FFFFFF" opacity="0.85"/>`,
  },
  {
    id: 'crescent',
    name: 'Crescent',
    description: 'A thick crescent moon that rocks gently over the sung word.',
    motion: 'Rocks like a little hammock: tilts back about 16° in the air, then tips forward on landing with a mild squash. A soft glow on the inner edge can breathe slowly, independent of the hop, for a quieter “night” feel on slow songs.',
    colorSupport: 'Fully recolorable. The moon is currentColor with a white sheen along the outer limb and a dark overlay toward the tips.',
    recorable: true,
    colorHow: 'single fill + auto white limb sheen + dark overlay',
    body: `
    <g transform="translate(28.4 0) scale(-1 1)">
      <path class="lg-glow" fill="currentColor" opacity="0.16" d="M15.26 19.35 A8.70 8.70 0 1 0 12.19 3.45 A8.20 8.20 0 0 1 15.26 19.35 Z"/>
      <path class="lg-body" fill="currentColor" d="M15.26 19.35 A8.70 8.70 0 1 0 12.19 3.45 A8.20 8.20 0 0 1 15.26 19.35 Z"/>
      <path class="lg-highlight" fill="#FFFFFF" opacity="0.45" d="M18.6 8.4 C20.2 10.5 20.6 13.2 19.7 15.7 C20.8 13.3 20.6 10.6 18.9 8.3 C18.8 8.3 18.7 8.35 18.6 8.4 Z"/>
    </g>`,
  },
];

module.exports = { DEFAULT_COLOR, bouncers, wrap, starPath };
