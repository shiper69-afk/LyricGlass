# LyricGlass bouncers

Twelve original karaoke cursors for LyricGlass, a Windows lyrics overlay. They hop on top of the sung word in a floating dark or frosted-glass window, in the same family as the **Glass Orb** app icon: clean vector shapes, coral by default (`#FF3F4C`), a white specular sheen, and a soft dark overlay.

Each file is a 24×24 SVG, drawn to stay readable at 16 and 24 px over lyric text. There are no trademarked characters or logos, and no Spotify green or wordmark.

## Colour

Every bouncer is built to take a **user-chosen colour**.

- The body (and glow, tail, wings, bars, and so on) uses `currentColor`.
- The default is the presentation attribute `color="#FF3F4C"` on the root `<svg>`, so a file used as an `<img>` or in a resource dictionary still renders coral.
- When the SVG is **inlined**, set `color` on the svg or a parent, or set `--lg-bouncer` and `color: var(--lg-bouncer)` from the host stylesheet. Do not put `var(--lg-bouncer)` inside the file itself — some renderers drop the fallback and paint the body black.
- Highlights are a **fixed white sheen**. Shading is a dark `#041218` overlay at low opacity. Both work on any hue, so there is no second baked-in colour to restyle.
- All twelve are fully recolorable. Firefly wings use `currentColor` at 0.75 opacity (plus a white sheen) so they stay visible on light glass, not just on a dark window.

## Motion

Shared hop: stretch tall at the apex, squash toward the baseline on landing, optional contact shadow that scales out in the air. Pose frames for AIR / LAND / SQUASH are on the preview sheet. Per-bouncer extras are listed below.

Origin is the 24×24 viewBox; sit the sprite so the visual mass rests just above the active word. **Comet** and **Paper Plane** trail to the left (hopping right). For RTL Hebrew lyrics, flip those two with `scaleX(-1)` so the tail or nose still follows the reading direction.

## The set

### 1. Glow Orb

File: `svg/glow-orb.svg`

**Description.** The classic karaoke ball: a soft glowing orb that hops along the sung word.

**Motion.** Classic squash-and-stretch. Stretches tall at the top of the hop, flattens on landing, then settles. A soft glow pulses slightly louder at the apex. Optional contact shadow scales out as it leaves the word.

**Colour.** Fully recolorable. Single currentColor fill with a fixed white specular highlight and a dark shade overlay (works on any hue). Glow uses currentColor at low opacity.

### 2. Music Note

File: `svg/music-note.svg`

**Description.** A chunky eighth note that marks the beat as each word is sung.

**Motion.** Tilts back about 18° while airborne, tips forward on landing, and lightly squashes through the note head. The flag can give a tiny delayed follow-through (it lags the stem by a frame or two).

**Colour.** Fully recolorable. The whole note (head, stem, flag) is currentColor. A white sheen sits on the head.

### 3. Spark

File: `svg/spark.svg`

**Description.** A four-point sparkle star that twinkles over the active lyric.

**Motion.** Slowly rotates (about 20–30° per hop) and twinkles with a short scale pulse at the apex. On landing it squashes slightly along the vertical points. A one-frame “flash” (scale 1.15) can mark the start of each word.

**Colour.** Fully recolorable. Star body is currentColor; a white sheen sits on the upper point. Glow uses currentColor.

### 4. Heart

File: `svg/heart.svg`

**Description.** A bold heart that sits on the sung word — clear even at tray-like sizes.

**Motion.** Pulses (a quick beat-scale) as it lands, with a mild squash in the lower tip. Slight sideways tilt in the air, then a spring back to upright. Can double-pulse on a held note.

**Colour.** Fully recolorable. Body is currentColor with a white sheen on the left lobe and a dark overlay in the lower right.

### 5. Glass Drop

File: `svg/glass-drop.svg`

**Description.** A glassy teardrop, like a bead of light resting on the lyric line.

**Motion.** Stretches into a taller drop at the apex, then a wet squash on landing (wide and short) with a short wobble (horizontal shear) as it settles. The highlight can slide up the left edge during the hop.

**Colour.** Fully recolorable. Body is currentColor; the “glass” comes from a white rim sheen and a sliding specular highlight, not a fixed teal.

### 6. Firefly

File: `svg/firefly.svg`

**Description.** A tiny lantern bug: glowing abdomen, faint glass wings, and a small head.

**Motion.** Hover more than bounce — a softer landing with less squash, a constant idle bob, and a glow that pulses independently of the hop. Wings flutter with a short scale/rotate loop. On a held note it can hover in place and pulse.

**Colour.** Fully recolorable. Body, head, glow and wings use currentColor (wings at 0.7 opacity so they stay visible on light glass). A white sheen sits on each wing and on the abdomen.

### 7. Comet

File: `svg/comet.svg`

**Description.** A bright nucleus with a trailing tail that streams behind the hop.

**Motion.** The nucleus uses the same squash-and-stretch as Glow Orb. The tail always streams opposite the hop direction: long and loose in the air, compressed or tucked on landing. In RTL lyrics, flip the tail with scaleX(-1) so it still trails behind.

**Colour.** Fully recolorable. Nucleus and tail are currentColor; tail pieces fade by opacity. Nucleus gets the standard white highlight and dark overlay.

### 8. EQ Pulse

File: `svg/eq-pulse.svg`

**Description.** Three rounded equalizer bars that bounce as a little meter on the word.

**Motion.** The group hops as one, but each bar independently scales in height (transform-origin at the bottom) on the beat. On landing the whole meter squashes flatter. On a held note the bars can keep pulsing in place without hopping.

**Colour.** Fully recolorable. All three bars are currentColor at slightly different opacities so the meter still reads as a set. Top-cap highlights are white.

### 9. Mic

File: `svg/mic.svg`

**Description.** A tiny handheld microphone — the karaoke object itself, hopping the line.

**Motion.** Swings like a short pendulum (about ±16°) while airborne, then squashes through the capsule on landing. A very small “sound ring” can flash around the grille at the start of a word.

**Colour.** Fully recolorable. Capsule, handle and ferrule are currentColor. A dark grille band and white sheen are overlays, so any user colour still reads as a mic.

### 10. Paper Plane

File: `svg/paper-plane.svg`

**Description.** A folded paper dart that banks from word to word like a short flight.

**Motion.** Banks (rolls) into the hop, glides with the nose slightly up, then dips the nose and flattens on landing. Travel between words can include a short horizontal glide, not only a vertical bounce. In RTL, flip with scaleX(-1).

**Colour.** Fully recolorable. Both facets are currentColor; the fold is a dark overlay on the lower wing and a white sheen on the upper wing, so the paper-fold reads in any colour.

### 11. Pixel

File: `svg/pixel.svg`

**Description.** A chunky square pixel with a 1-bit highlight — like a cursor on the lyric.

**Motion.** Snappy, not fluid. The hop can be stepped (a few discrete frames) rather than eased. On landing it flattens into a wider rectangle for one frame, then pops back. Optional 1-frame “blink” (the highlight flashes) at the start of each word.

**Colour.** Fully recolorable. The square is currentColor with a darker bottom/right bevel overlay and a white 1-bit highlight pixel in the corner.

### 12. Crescent

File: `svg/crescent.svg`

**Description.** A thick crescent moon that rocks gently over the sung word.

**Motion.** Rocks like a little hammock: tilts back about 16° in the air, then tips forward on landing with a mild squash. A soft glow on the inner edge can breathe slowly, independent of the hop, for a quieter “night” feel on slow songs.

**Colour.** Fully recolorable. The moon is currentColor with a white sheen along the outer limb and a dark overlay toward the tips.

## Files

- `svg/*.svg` — one 24×24 master per bouncer
- `preview/preview-sheet.png` — all twelve at 96, 24 and 16 px, over lyric words on dark glass and on light/frosted glass, a violet user-colour row, Glow Orb in six swatches, AIR / LAND / SQUASH poses, and a 16 px ×8 nearest-neighbour stress test
- `preview/png/` — standalone 16, 24 and 96 px PNGs (coral, transparent)
- `demo/index.html` — offline colour-picker and hop-along-lyrics demo (open the file in a browser)
- `_tools/` — `svgs.js` sources, `build.js` rebuilds everything (needs the `@resvg/resvg-js` already installed under `/workspace/lyricglass-icons/_tools/node_modules`)

Rebuild:

```bash
node /workspace/lyricglass-bouncers/_tools/build.js
python3 /workspace/lyricglass-bouncers/_tools/compose-preview.py
```
