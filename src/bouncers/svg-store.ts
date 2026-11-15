import type { BouncerId } from "./ids";

import glowOrb from "./svg/glow-orb.svg?raw";
import musicNote from "./svg/music-note.svg?raw";
import spark from "./svg/spark.svg?raw";
import heart from "./svg/heart.svg?raw";
import glassDrop from "./svg/glass-drop.svg?raw";
import firefly from "./svg/firefly.svg?raw";
import comet from "./svg/comet.svg?raw";
import eqPulse from "./svg/eq-pulse.svg?raw";
import mic from "./svg/mic.svg?raw";
import paperPlane from "./svg/paper-plane.svg?raw";
import pixel from "./svg/pixel.svg?raw";
import crescent from "./svg/crescent.svg?raw";

function innerSvgMarkup(body: string): string {
  const open = body.indexOf("<g ");
  const close = body.lastIndexOf("</g>");
  if (open < 0 || close < 0) return body;
  return body.slice(open, close + 4);
}

const svgInner: Record<BouncerId, string> = {
  "glow-orb": innerSvgMarkup(glowOrb),
  "music-note": innerSvgMarkup(musicNote),
  spark: innerSvgMarkup(spark),
  heart: innerSvgMarkup(heart),
  "glass-drop": innerSvgMarkup(glassDrop),
  firefly: innerSvgMarkup(firefly),
  comet: innerSvgMarkup(comet),
  "eq-pulse": innerSvgMarkup(eqPulse),
  mic: innerSvgMarkup(mic),
  "paper-plane": innerSvgMarkup(paperPlane),
  pixel: innerSvgMarkup(pixel),
  crescent: innerSvgMarkup(crescent),
};

export function bouncerSvgInner(id: BouncerId): string {
  return svgInner[id] ?? svgInner["glow-orb"];
}

export function bouncerSvgMarkup(id: BouncerId): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" fill="none" class="lg-bouncer-svg" aria-hidden="true">${bouncerSvgInner(id)}</svg>`;
}