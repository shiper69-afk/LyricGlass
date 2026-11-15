'use strict';

const fs = require('fs');
const path = require('path');
const { bouncers, wrap, DEFAULT_COLOR } = require('./svgs');
const { renderSvg } = require('./render-svg');

const ROOT = path.join(__dirname, '..');
const SVG_DIR = path.join(ROOT, 'svg');
const PREVIEW_DIR = path.join(ROOT, 'preview');
const DEMO_DIR = path.join(ROOT, 'demo');
const PNG_DIR = path.join(PREVIEW_DIR, 'png');

const VIOLET = '#8B6CFF';
const GOLD = '#F0B429';
const MINT = '#3DDC97';
const SKY = '#3DB8FF';
const ROSE = '#FF6BA8';
const SWATCHES = [
  { name: 'Coral', hex: DEFAULT_COLOR, note: 'default' },
  { name: 'Violet', hex: VIOLET, note: 'user colour' },
  { name: 'Gold', hex: GOLD, note: '' },
  { name: 'Mint', hex: MINT, note: '' },
  { name: 'Sky', hex: SKY, note: '' },
  { name: 'Rose', hex: ROSE, note: '' },
];

const LYRIC_WORDS = ['Dance', 'through', 'the', 'glass', 'tonight', 'we', 'light', 'every', 'word', 'you', 'sing'];
// 11 words — add a 12th
LYRIC_WORDS.push('now');

function ensureDirs() {
  for (const d of [SVG_DIR, PREVIEW_DIR, DEMO_DIR, PNG_DIR]) {
    fs.mkdirSync(d, { recursive: true });
  }
}

function exportSvgs() {
  for (const b of bouncers) {
    fs.writeFileSync(path.join(SVG_DIR, `${b.id}.svg`), wrap(b));
  }
}

function inner(b) {
  return `<g class="lg-bouncer lg-${b.id}">${b.body}\n  </g>`;
}

function iconSvg(b, { x, y, size, color = DEFAULT_COLOR, transform = '', viewBox } = {}) {
  const vb = viewBox || '0 0 24 24';
  const g = transform ? `<g transform="${transform}">${inner(b)}</g>` : inner(b);
  return `<svg x="${x}" y="${y}" width="${size}" height="${size}" viewBox="${vb}" color="${color}" overflow="visible">${g}</svg>`;
}

const POSES = {
  default: {
    air: 'translate(12 20) scale(0.90 1.14) translate(-12 -20) translate(0 -3.4)',
    land: '',
    squash: 'translate(12 20) scale(1.26 0.70) translate(-12 -20)',
  },
  'music-note': {
    air: 'translate(12 20) rotate(-16 12 12) scale(0.94 1.10) translate(-12 -20) translate(0 -3.0)',
    land: '',
    squash: 'translate(12 20) rotate(10 12 12) scale(1.20 0.76) translate(-12 -20)',
  },
  spark: {
    air: 'translate(12 20) rotate(20 12 12) scale(1.08 1.08) translate(-12 -20) translate(0 -3.0)',
    land: '',
    squash: 'translate(12 20) rotate(-8 12 12) scale(1.18 0.78) translate(-12 -20)',
  },
  heart: {
    air: 'translate(12 20) scale(1.06 1.10) translate(-12 -20) translate(0 -2.8)',
    land: '',
    squash: 'translate(12 20) scale(1.22 0.78) translate(-12 -20)',
  },
  'glass-drop': {
    air: 'translate(12 20) scale(0.86 1.20) translate(-12 -20) translate(0 -3.2)',
    land: '',
    squash: 'translate(12 20) scale(1.34 0.64) translate(-12 -20)',
  },
  firefly: {
    air: 'translate(0 -3.8) scale(1.02 1.04)',
    land: '',
    squash: 'translate(12 20) scale(1.12 0.86) translate(-12 -20)',
  },
  comet: {
    air: 'translate(12 20) rotate(-7 12 12) scale(0.94 1.10) translate(-12 -20) translate(-0.6 -3.2)',
    land: '',
    squash: 'translate(12 20) scale(1.18 0.76) translate(-12 -20)',
  },
  'eq-pulse': {
    air: 'translate(0 -3.2) scale(1 1.06)',
    land: '',
    squash: 'translate(12 20) scale(1.16 0.72) translate(-12 -20)',
  },
  mic: {
    air: 'translate(12 20) rotate(-14 12 12) translate(-12 -20) translate(0 -3.2)',
    land: '',
    squash: 'translate(12 20) rotate(8 12 12) scale(1.16 0.78) translate(-12 -20)',
  },
  'paper-plane': {
    air: 'translate(12 20) rotate(-18 12 12) translate(-12 -20) translate(0 -3.4)',
    land: '',
    squash: 'translate(12 20) rotate(10 12 12) scale(1.12 0.82) translate(-12 -20)',
  },
  pixel: {
    air: 'translate(0 -4)',
    land: '',
    squash: 'translate(12 20) scale(1.28 0.68) translate(-12 -20)',
  },
  crescent: {
    air: 'translate(12 20) rotate(-16 12 12) translate(-12 -20) translate(0 -3.0)',
    land: '',
    squash: 'translate(12 20) rotate(12 12 12) scale(1.14 0.80) translate(-12 -20)',
  },
};

function posesFor(id) {
  return POSES[id] || POSES.default;
}

function checkerPattern(id, c1, c2, size = 6) {
  return `<pattern id="${id}" width="${size * 2}" height="${size * 2}" patternUnits="userSpaceOnUse">
    <rect width="${size * 2}" height="${size * 2}" fill="${c1}"/>
    <rect width="${size}" height="${size}" fill="${c2}"/>
    <rect x="${size}" y="${size}" width="${size}" height="${size}" fill="${c2}"/>
  </pattern>`;
}

function buildPreviewSvg() {
  const W = 2600;
  const PAD = 72;
  const headerH = 236;
  const colGap = 22;
  const rowGap = 22;
  const cols = 4;
  const cardW = (W - PAD * 2 - colGap * (cols - 1)) / cols;
  const cardH = 340;
  const sectionGap = 56;

  let y = 0;
  const parts = [];

  // background
  parts.push(`<rect width="${W}" height="4000" fill="#07141B"/>`);
  // subtle vignette dots (very light starfield / glass dust)
  parts.push(`<circle cx="240" cy="80" r="1.2" fill="#7EE7F2" opacity="0.25"/>
    <circle cx="2320" cy="120" r="1.1" fill="#7EE7F2" opacity="0.2"/>
    <circle cx="2480" cy="40" r="0.9" fill="#FF3F4C" opacity="0.25"/>`);

  // Header
  y = 56;
  parts.push(`
    <text x="${PAD}" y="${y}" font-family="Inter" font-size="13" font-weight="600" letter-spacing="2.4" fill="#5ED0E0">LYRICGLASS</text>
    <text x="${PAD}" y="${y + 52}" font-family="Outfit" font-size="54" font-weight="650" fill="#F4FFFE">Bouncers</text>
    <text x="${PAD}" y="${y + 86}" font-family="Inter" font-size="16" font-weight="400" fill="#8BB8C4">12 karaoke cursors for the lyrics overlay · Glass Orb style · default coral #FF3F4C</text>
    <text x="${PAD}" y="${y + 110}" font-family="Inter" font-size="14" font-weight="400" fill="#6A9AAB">Recolor any of them by setting color, or when inlined, color: var(--lg-bouncer). Bodies use currentColor; highlights are a white sheen plus a dark overlay.</text>
  `);
  // header orb
  parts.push(iconSvg(bouncers[0], { x: W - PAD - 96, y: 48, size: 96 }));

  y = headerH;

  // Section 1
  parts.push(`<text x="${PAD}" y="${y}" font-family="Inter" font-size="13" font-weight="600" letter-spacing="1.8" fill="#5ED0E0">THE SET</text>`);
  y += 20;

  bouncers.forEach((b, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    const x = PAD + col * (cardW + colGap);
    const cy = y + row * (cardH + rowGap);
    const poses = posesFor(b.id);

    parts.push(`
      <rect x="${x}" y="${cy}" width="${cardW}" height="${cardH}" rx="22" fill="#0C232D" stroke="#1A4A58" stroke-width="1.2"/>
      ${iconSvg(b, { x: x + 28, y: cy + 24, size: 96 })}
      <text x="${x + 140}" y="${cy + 52}" font-family="Outfit" font-size="24" font-weight="600" fill="#F4FFFE">${b.name}</text>
      ${wrapSvgText(b.description, x + 140, cy + 80, cardW - 168, 13, 17, "#8BB8C4")}
    `);

    // sizes row
    const sizesY = cy + 136;
    const sizeBoxes = [
      { label: '24', size: 24, bg: '#07141B', lx: 28 },
      { label: '16', size: 16, bg: '#07141B', lx: 28 + 78 },
      { label: '24', size: 24, bg: '#E8F4F6', lx: 28 + 78 * 2 },
      { label: '16', size: 16, bg: '#E8F4F6', lx: 28 + 78 * 3 },
    ];
    for (const s of sizeBoxes) {
      const bx = x + s.lx;
      const box = 48;
      parts.push(`<rect x="${bx}" y="${sizesY}" width="${box}" height="${box}" rx="8" fill="${s.bg}"/>`);
      const inset = (box - s.size) / 2;
      parts.push(iconSvg(b, { x: bx + inset, y: sizesY + inset, size: s.size }));
      const labelFill = s.bg === '#E8F4F6' ? '#3A6A78' : '#6A9AAB';
      parts.push(`<text x="${bx + box / 2}" y="${sizesY + box + 16}" text-anchor="middle" font-family="Inter" font-size="10" fill="${labelFill}">${s.label}${s.bg === '#E8F4F6' ? ' light' : ''}</text>`);
    }

    // poses
    const poseY = cy + 220;
    const poseEntries = [
      ['AIR', poses.air],
      ['LAND', poses.land],
      ['SQUASH', poses.squash],
    ];
    const poseSize = 56;
    const poseStart = x + 28;
    const poseGap = 18;
    poseEntries.forEach(([label, t], pi) => {
      const px = poseStart + pi * (poseSize + poseGap + 28);
      parts.push(`<rect x="${px}" y="${poseY}" width="${poseSize}" height="${poseSize}" rx="10" fill="#07141B"/>`);
      // ground shadow
      parts.push(`<ellipse cx="${px + poseSize / 2}" cy="${poseY + poseSize - 7}" rx="14" ry="2.4" fill="#041218" opacity="0.45"/>`);
      parts.push(iconSvg(b, {
        x: px + 4,
        y: poseY + 2,
        size: poseSize - 8,
        transform: t,
        viewBox: '-3 -5 30 32',
      }));
      parts.push(`<text x="${px + poseSize / 2}" y="${poseY + poseSize + 16}" text-anchor="middle" font-family="Inter" font-size="10" font-weight="600" letter-spacing="0.8" fill="#6A9AAB">${label}</text>`);
    });
    const motionX = poseStart + 3 * (poseSize + poseGap + 28) - 8;
    const motionW = x + cardW - 24 - motionX;
    parts.push(`<text x="${motionX}" y="${poseY + 8}" font-family="Inter" font-size="10" font-weight="600" letter-spacing="0.8" fill="#5ED0E0">MOTION</text>`);
    parts.push(wrapSvgText(b.motion, motionX, poseY + 24, motionW, 11, 14.5, "#8BB8C4", 6));
  });

  y += 3 * (cardH + rowGap) - rowGap + sectionGap;

  // Section 2 — dark lyrics
  const darkH = 310;
  parts.push(`<text x="${PAD}" y="${y}" font-family="Inter" font-size="13" font-weight="600" letter-spacing="1.8" fill="#5ED0E0">ON LYRICS — DARK GLASS</text>`);
  y += 18;
  const darkPanelY = y;
  parts.push(`
    <rect x="${PAD}" y="${darkPanelY}" width="${W - PAD * 2}" height="${darkH}" rx="28" fill="#0A1E26"/>
    <rect x="${PAD}" y="${darkPanelY}" width="${W - PAD * 2}" height="${darkH}" rx="28" fill="#7EE7F2" opacity="0.04"/>
    <rect x="${PAD + 1.5}" y="${darkPanelY + 1.5}" width="${W - PAD * 2 - 3}" height="${darkH - 3}" rx="26.5" fill="none" stroke="#7EE7F2" stroke-opacity="0.28" stroke-width="1.4"/>
    <path d="M${PAD + 40} ${darkPanelY + 18} C ${PAD + 200} ${darkPanelY + 6}, ${PAD + 400} ${darkPanelY + 10}, ${PAD + 560} ${darkPanelY + 22}" fill="none" stroke="#FFFFFF" stroke-opacity="0.10" stroke-width="8" stroke-linecap="round"/>
  `);

  // 24px row
  parts.push(`<text x="${PAD + 40}" y="${darkPanelY + 44}" font-family="Inter" font-size="11" font-weight="600" letter-spacing="1.2" fill="#5ED0E0">24 PX</text>`);
  const darkWordsY = darkPanelY + 118;
  layoutLyricRow(parts, bouncers, LYRIC_WORDS, {
    x: PAD + 40,
    y: darkWordsY,
    width: W - PAD * 2 - 80,
    iconSize: 24,
    fontSize: 32,
    fontWeight: 550,
    textFill: '#F4FFFE',
    activeFill: '#FFFFFF',
    color: DEFAULT_COLOR,
  });

  // 16px row
  parts.push(`<text x="${PAD + 40}" y="${darkPanelY + 188}" font-family="Inter" font-size="11" font-weight="600" letter-spacing="1.2" fill="#5ED0E0">16 PX</text>`);
  layoutLyricRow(parts, bouncers, LYRIC_WORDS, {
    x: PAD + 40,
    y: darkPanelY + 252,
    width: W - PAD * 2 - 80,
    iconSize: 16,
    fontSize: 22,
    fontWeight: 500,
    textFill: '#C5E0E6',
    activeFill: '#F4FFFE',
    color: DEFAULT_COLOR,
  });

  y += darkH + sectionGap;

  // Section 3 — light glass
  const lightH = 330;
  parts.push(`<text x="${PAD}" y="${y}" font-family="Inter" font-size="13" font-weight="600" letter-spacing="1.8" fill="#5ED0E0">ON LYRICS — LIGHT / FROSTED GLASS</text>`);
  y += 18;
  const lightBgY = y;
  // desktop-like pale teal
  parts.push(`<rect x="${PAD}" y="${lightBgY}" width="${W - PAD * 2}" height="${lightH}" rx="28" fill="#D5ECEF"/>`);
  // frosted window inset
  const winX = PAD + 36;
  const winY = lightBgY + 24;
  const winW = W - PAD * 2 - 72;
  const winH = lightH - 40;
  parts.push(`
    <rect x="${winX}" y="${winY}" width="${winW}" height="${winH}" rx="22" fill="#F4FFFE" fill-opacity="0.72"/>
    <rect x="${winX}" y="${winY}" width="${winW}" height="${winH}" rx="22" fill="#7EE7F2" fill-opacity="0.10"/>
    <rect x="${winX + 1.2}" y="${winY + 1.2}" width="${winW - 2.4}" height="${winH - 2.4}" rx="20.8" fill="none" stroke="#2EABC0" stroke-opacity="0.35" stroke-width="1.3"/>
    <path d="M${winX + 32} ${winY + 16} C ${winX + 180} ${winY + 6}, ${winX + 340} ${winY + 8}, ${winX + 480} ${winY + 18}" fill="none" stroke="#FFFFFF" stroke-opacity="0.55" stroke-width="7" stroke-linecap="round"/>
  `);

  parts.push(`<text x="${winX + 32}" y="${winY + 42}" font-family="Inter" font-size="11" font-weight="600" letter-spacing="1.2" fill="#1A86A0">24 PX</text>`);
  layoutLyricRow(parts, bouncers, LYRIC_WORDS, {
    x: winX + 32,
    y: winY + 108,
    width: winW - 64,
    iconSize: 24,
    fontSize: 32,
    fontWeight: 550,
    textFill: '#0C4F68',
    activeFill: '#08384A',
    color: DEFAULT_COLOR,
  });

  // Hebrew line with 16px bouncers over a shorter phrase, repeated/cycled
  const heWords = ['שרים', 'באור', 'הזכוכית', 'הלילה'];
  // cycle bouncers 0-3 or pick 4 distinctive ones? Use first 4? Better use 4 from across the set: orb, note, drop, plane — or just first 4. I'll spread: 0,2,5,11
  const heBouncers = [bouncers[0], bouncers[2], bouncers[4], bouncers[11]];
  parts.push(`<text x="${winX + 32}" y="${winY + 174}" font-family="Inter" font-size="11" font-weight="600" letter-spacing="1.2" fill="#1A86A0">16 PX · HEBREW</text>`);
  layoutLyricRow(parts, heBouncers, heWords, {
    x: winX + 32,
    y: winY + 240,
    width: winW - 64,
    iconSize: 16,
    fontSize: 26,
    fontWeight: 500,
    textFill: '#0C4F68',
    activeFill: '#08384A',
    color: DEFAULT_COLOR,
    rtl: true,
    fontFamily: 'IBM Plex Sans Hebrew',
  });

  y += lightH + sectionGap;

  // Section 4 — user colour
  const colorH = 280;
  parts.push(`<text x="${PAD}" y="${y}" font-family="Inter" font-size="13" font-weight="600" letter-spacing="1.8" fill="#5ED0E0">USER COLOUR</text>
    <text x="${PAD + 140}" y="${y}" font-family="Inter" font-size="13" font-weight="400" fill="#8BB8C4">All 12 in violet #8B6CFF, plus Glow Orb in six swatches to show a single fill taking any hue</text>`);
  y += 20;
  const colorPanelY = y;
  parts.push(`<rect x="${PAD}" y="${colorPanelY}" width="${W - PAD * 2}" height="${colorH}" rx="28" fill="#0C232D" stroke="#1A4A58" stroke-width="1.2"/>`);

  const iconSlot = (W - PAD * 2 - 80) / 12;
  bouncers.forEach((b, i) => {
    const cx = PAD + 40 + i * iconSlot + iconSlot / 2;
    const iy = colorPanelY + 36;
    parts.push(iconSvg(b, { x: cx - 36, y: iy, size: 72, color: VIOLET }));
    parts.push(`<text x="${cx}" y="${iy + 96}" text-anchor="middle" font-family="Inter" font-size="11" font-weight="500" fill="#C8B8FF">${b.name}</text>`);
  });

  // swatches of Glow Orb
  SWATCHES.forEach((s, i) => {
    const sx = PAD + 40 + i * 160;
    const sy = colorPanelY + 168;
    parts.push(`<rect x="${sx}" y="${sy}" width="140" height="88" rx="16" fill="#07141B"/>`);
    parts.push(iconSvg(bouncers[0], { x: sx + 16, y: sy + 16, size: 56, color: s.hex }));
    parts.push(`<text x="${sx + 80}" y="${sy + 40}" font-family="Inter" font-size="13" font-weight="600" fill="#F4FFFE">${s.name}</text>`);
    parts.push(`<text x="${sx + 80}" y="${sy + 58}" font-family="Inter" font-size="11" fill="#8BB8C4">${s.hex}</text>`);
    if (s.note) {
      parts.push(`<text x="${sx + 80}" y="${sy + 74}" font-family="Inter" font-size="10" fill="#5ED0E0">${s.note}</text>`);
    }
  });

  y += colorH + sectionGap;

  // Placeholder for 16px zoom section (Python will extend the canvas
  // if we leave a marker; easier to just set the final height now and
  // leave a labelled empty area that Python pastes into.)
  const zoomLabelY = y;
  const zoomH = 420;
  parts.push(`<text x="${PAD}" y="${zoomLabelY}" font-family="Inter" font-size="13" font-weight="600" letter-spacing="1.8" fill="#5ED0E0">16 PX STRESS TEST</text>
    <text x="${PAD + 200}" y="${zoomLabelY}" font-family="Inter" font-size="13" font-weight="400" fill="#8BB8C4">Nearest-neighbour ×8 of the true 16 px render, on dark and on light — how it will look over lyrics in the tray-adjacent size</text>`);
  // marker rect for the compositor
  parts.push(`<rect id="zoom-slot" x="${PAD}" y="${zoomLabelY + 16}" width="${W - PAD * 2}" height="${zoomH - 16}" rx="20" fill="#0A1A20"/>`);

  y = zoomLabelY + zoomH + 48;
  parts.push(`
    <text x="${PAD}" y="${y}" font-family="Inter" font-size="12" fill="#6A9AAB">LyricGlass bouncers  ·  24×24 SVG  ·  currentColor  ·  /workspace/lyricglass-bouncers/</text>
  `);

  const H = y + 40;

  const svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
     width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">
  <defs>
    ${checkerPattern('chk-dark', '#0A1A20', '#12303A', 6)}
    ${checkerPattern('chk-light', '#E8F4F6', '#F4FFFE', 6)}
  </defs>
  ${parts.join('\n')}
</svg>`;

  return { svg, W, H, zoom: { x: PAD, y: zoomLabelY + 16, w: W - PAD * 2, h: zoomH - 16 } };
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function wrapSvgText(text, x, y, maxWidth, fontSize, lineHeight, fill, maxLines = 3) {
  const pxPerChar = fontSize < 12 ? 5.7 : 6.6;
  const maxChars = Math.max(16, Math.floor(maxWidth / pxPerChar));
  const words = text.split(' ');
  const lines = [];
  let cur = '';
  for (const w of words) {
    const next = cur ? cur + ' ' + w : w;
    if (next.length > maxChars && cur) {
      lines.push(cur);
      cur = w;
    } else cur = next;
  }
  if (cur) lines.push(cur);
  const shown = lines.slice(0, maxLines);
  if (lines.length > maxLines) {
    const last = shown[shown.length - 1].replace(/\s+\S+$/, '') + '…';
    shown[shown.length - 1] = last;
  }
  return shown.map((ln, i) =>
    `<text x="${x}" y="${y + i * lineHeight}" font-family="Inter" font-size="${fontSize}" fill="${fill}">${escapeHtml(ln)}</text>`
  ).join('\n');
}


function layoutLyricRow(parts, iconList, words, opts) {
  const {
    x, y, width, iconSize, fontSize, fontWeight, textFill, activeFill, color,
    rtl = false, fontFamily = 'Inter',
  } = opts;
  // Approximate word widths and space them evenly, left-to-right or RTL.
  // We don't have exact text metrics; estimate 0.58*fontSize per character + padding.
  const estimates = words.map((w) => Math.max(iconSize + 8, w.length * fontSize * 0.62 + 16));
  const gap = 10;
  const total = estimates.reduce((a, b) => a + b, 0) + gap * (words.length - 1);
  const scale = total > width ? width / total : 1;
  let cursor = rtl ? x + width : x;
  // RTL: start at the right edge and place words in original order so the first word sits on the right.
  const order = words.map((w, i) => ({ w, i, b: iconList[i], est: estimates[i] * scale }));

  for (const item of order) {
    const slot = item.est;
    const left = rtl ? cursor - slot : cursor;
    const cx = left + slot / 2;
    // underline / word
    const textY = y + 10;
    const fill = item.i % 3 === 0 ? activeFill : textFill;
    const anchor = 'middle';
    const fontAttr = `font-family="${fontFamily}" font-size="${fontSize}" font-weight="${fontWeight}"`;
    const dir = rtl ? ' direction="rtl"' : '';
    parts.push(`<text x="${cx}" y="${textY}" text-anchor="${anchor}" ${fontAttr} fill="${fill}"${dir}>${escapeHtml(item.w)}</text>`);
    // bouncer sits just above the word
    const ix = cx - iconSize / 2;
    const gapBelowIcon = 20;
    const iy = y - fontSize - gapBelowIcon - iconSize;
    if (item.b) {
      parts.push(iconSvg(item.b, { x: ix, y: iy, size: iconSize, color }));
    }
    cursor = rtl ? cursor - slot - gap * scale : cursor + slot + gap * scale;
  }
}

function writeReadme() {
  const lines = [];
  lines.push('# LyricGlass bouncers');
  lines.push('');
  lines.push('Twelve original karaoke cursors for Ilan’s Windows lyrics overlay, LyricGlass. They hop on top of the sung word in a floating dark or frosted-glass window, in the same family as the **Glass Orb** app icon: clean vector shapes, coral by default (`#FF3F4C`), a white specular sheen, and a soft dark overlay.');
  lines.push('');
  lines.push('Each file is a 24×24 SVG, drawn to stay readable at 16 and 24 px over lyric text. There are no trademarked characters or logos, and no Spotify green or wordmark.');
  lines.push('');
  lines.push('## Colour');
  lines.push('');
  lines.push('Every bouncer is built to take a **user-chosen colour**.');
  lines.push('');
  lines.push('- The body (and glow, tail, wings, bars, and so on) uses `currentColor`.');
  lines.push('- The default is the presentation attribute `color="#FF3F4C"` on the root `<svg>`, so a file used as an `<img>` or in a resource dictionary still renders coral.');
  lines.push('- When the SVG is **inlined**, set `color` on the svg or a parent, or set `--lg-bouncer` and `color: var(--lg-bouncer)` from the host stylesheet. Do not put `var(--lg-bouncer)` inside the file itself — some renderers drop the fallback and paint the body black.');
  lines.push('- Highlights are a **fixed white sheen**. Shading is a dark `#041218` overlay at low opacity. Both work on any hue, so there is no second baked-in colour to restyle.');
  lines.push('- All twelve are fully recolorable. Firefly wings use `currentColor` at 0.75 opacity (plus a white sheen) so they stay visible on light glass, not just on a dark window.');
  lines.push('');
  lines.push('## Motion');
  lines.push('');
  lines.push('Shared hop: stretch tall at the apex, squash toward the baseline on landing, optional contact shadow that scales out in the air. Pose frames for AIR / LAND / SQUASH are on the preview sheet. Per-bouncer extras are listed below.');
  lines.push('');
  lines.push('Origin is the 24×24 viewBox; sit the sprite so the visual mass rests just above the active word. **Comet** and **Paper Plane** trail to the left (hopping right). For RTL Hebrew lyrics, flip those two with `scaleX(-1)` so the tail or nose still follows the reading direction.');
  lines.push('');
  lines.push('## The set');
  lines.push('');
  bouncers.forEach((b, i) => {
    lines.push(`### ${i + 1}. ${b.name}`);
    lines.push('');
    lines.push(`File: \`svg/${b.id}.svg\``);
    lines.push('');
    lines.push(`**Description.** ${b.description}`);
    lines.push('');
    lines.push(`**Motion.** ${b.motion}`);
    lines.push('');
    lines.push(`**Colour.** ${b.colorSupport}`);
    lines.push('');
  });
  lines.push('## Files');
  lines.push('');
  lines.push('- `svg/*.svg` — one 24×24 master per bouncer');
  lines.push('- `preview/preview-sheet.png` — all twelve at 96, 24 and 16 px, over lyric words on dark glass and on light/frosted glass, a violet user-colour row, Glow Orb in six swatches, AIR / LAND / SQUASH poses, and a 16 px ×8 nearest-neighbour stress test');
  lines.push('- `preview/png/` — standalone 16, 24 and 96 px PNGs (coral, transparent)');
  lines.push('- `demo/index.html` — offline colour-picker and hop-along-lyrics demo (open the file in a browser)');
  lines.push('- `_tools/` — `svgs.js` sources, `build.js` rebuilds everything (needs the `@resvg/resvg-js` already installed under `/workspace/lyricglass-icons/_tools/node_modules`)');
  lines.push('');
  lines.push('Rebuild:');
  lines.push('');
  lines.push('```bash');
  lines.push('node /workspace/lyricglass-bouncers/_tools/build.js');
  lines.push('```');
  lines.push('');

  fs.writeFileSync(path.join(ROOT, 'README.md'), lines.join('\n'));
}

function writeDemo() {
  const icons = bouncers.map((b) => {
    // inner svg, no xml header, color inherited
    const body = wrap(b)
      .replace(/<\?xml[^>]*>\s*/, '')
      .replace(/\scolor="[^"]+"/, '')
      .replace('width="24" height="24"', 'width="100%" height="100%"');
    return { id: b.id, name: b.name, description: b.description, motion: b.motion, svg: body };
  });

  const html = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>LyricGlass bouncers</title>
<style>
  :root {
    --lg-bouncer: ${DEFAULT_COLOR};
    --bg: #07141B;
    --panel: #0C232D;
    --stroke: #1A4A58;
    --text: #F4FFFE;
    --muted: #8BB8C4;
    --teal: #5ED0E0;
    --glass: rgba(126, 231, 242, 0.06);
  }
  * { box-sizing: border-box; }
  html, body { margin: 0; background: var(--bg); color: var(--text); font-family: Inter, "IBM Plex Sans Hebrew", sans-serif; }
  body { padding: 32px 28px 80px; }
  h1 { font-family: Outfit, Inter, sans-serif; font-weight: 650; font-size: 42px; margin: 4px 0 8px; }
  .eyebrow { color: var(--teal); letter-spacing: 0.18em; font-size: 12px; font-weight: 600; }
  .lede { color: var(--muted); max-width: 62ch; margin: 0 0 28px; }
  .stage {
    position: relative;
    background: linear-gradient(180deg, rgba(126,231,242,0.07), rgba(12,35,45,0.9));
    border: 1px solid rgba(126,231,242,0.28);
    border-radius: 22px;
    padding: 28px 32px 24px;
    margin: 0 0 18px;
    overflow: hidden;
  }
  .stage:before {
    content: "";
    position: absolute; left: 36px; top: 14px; width: 280px; height: 10px;
    border-radius: 99px; background: rgba(255,255,255,0.12);
  }
  .lyric {
    display: flex; flex-wrap: wrap; justify-content: center; gap: 6px 18px;
    padding: 36px 8px 8px; font-size: 30px; font-weight: 550; line-height: 1;
  }
  .lyric.he { direction: rtl; font-family: "IBM Plex Sans Hebrew", Inter, sans-serif; font-size: 26px; padding-top: 20px; color: #C5E0E6; }
  .word { position: relative; display: inline-block; padding-top: 34px; color: #9BB8C0; transition: color .15s; }
  .word.active { color: #F4FFFE; }
  .word .bo {
    position: absolute; top: 0; left: 50%; width: 24px; height: 24px;
    margin-left: -12px; opacity: 0; color: var(--lg-bouncer);
    pointer-events: none;
  }
  .word.active .bo { opacity: 1; animation: hop .42s cubic-bezier(.2,.8,.2,1); }
  .word .bo svg { width: 24px; height: 24px; display: block; overflow: visible; }
  @keyframes hop {
    0%   { transform: translateY(6px) scale(1.26, 0.70); }
    42%  { transform: translateY(-13px) scale(0.90, 1.14); }
    82%  { transform: translateY(0) scale(1.22, 0.74); }
    100% { transform: translateY(0) scale(1, 1); }
  }
  .bo.pixel.active, .word.active .bo.pixel { animation-timing-function: steps(4, end); }
  .toolbar { display: flex; flex-wrap: wrap; gap: 10px 16px; align-items: center; margin: 0 0 22px; }
  .swatch { width: 28px; height: 28px; border-radius: 50%; border: 2px solid transparent; cursor: pointer; padding: 0; }
  .swatch[aria-pressed="true"] { border-color: #F4FFFE; }
  .swatch:focus { outline: 2px solid var(--teal); outline-offset: 2px; }
  label.picker { color: var(--muted); font-size: 13px; display: flex; align-items: center; gap: 8px; }
  select, input[type="color"] { background: var(--panel); color: var(--text); border: 1px solid var(--stroke); border-radius: 8px; padding: 6px 8px; font: inherit; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 14px; margin-top: 10px; }
  .card {
    background: var(--panel); border: 1px solid var(--stroke); border-radius: 18px;
    padding: 16px 16px 14px; cursor: pointer; text-align: left; color: inherit;
  }
  .card[aria-pressed="true"] { border-color: var(--teal); box-shadow: 0 0 0 1px var(--teal) inset; }
  .card .thumb { width: 72px; height: 72px; color: var(--lg-bouncer); margin: 0 auto 8px; }
  .card .thumb svg { width: 72px; height: 72px; display: block; overflow: visible; animation: idle 1.1s ease-in-out infinite; transform-origin: 50% 85%; }
  .card h3 { margin: 0 0 4px; font-size: 16px; font-family: Outfit, Inter, sans-serif; text-align: center; }
  .card p { margin: 0; color: var(--muted); font-size: 12px; line-height: 1.4; }
  @keyframes idle {
    0%, 100% { transform: translateY(0) scale(1,1); }
    40% { transform: translateY(-8px) scale(0.94, 1.08); }
    75% { transform: translateY(0) scale(1.16, 0.8); }
  }
  .sizes { display: flex; gap: 8px; justify-content: center; margin: 8px 0 10px; }
  .sizes span { display: flex; align-items: center; justify-content: center; background: #07141B; border-radius: 7px; }
  .sizes .s24 { width: 40px; height: 40px; }
  .sizes .s24 svg { width: 24px; height: 24px; }
  .sizes .s16 { width: 32px; height: 32px; }
  .sizes .s16 svg { width: 16px; height: 16px; }
  .sizes .light { background: #E8F4F6; }
</style>
</head>
<body>
  <div class="eyebrow">LYRICGLASS</div>
  <h1>Bouncers</h1>
  <p class="lede">Pick a type and a colour, then watch it hop the lyric line. All twelve use <code>currentColor</code> (default coral #FF3F4C). This file is self-contained — open it in a browser, no server needed.</p>

  <div class="toolbar">
    <span style="color:var(--muted);font-size:13px;">Colour</span>
    ${SWATCHES.map((s) => `<button class="swatch" data-color="${s.hex}" title="${s.name} ${s.hex}" style="background:${s.hex}" aria-label="${s.name}"></button>`).join('\n    ')}
    <label class="picker">Custom <input id="custom" type="color" value="${DEFAULT_COLOR}"/></label>
    <label class="picker">Type
      <select id="type">
        ${bouncers.map((b) => `<option value="${b.id}">${b.name}</option>`).join('')}
      </select>
    </label>
  </div>

  <div class="stage" id="stage">
    <div class="lyric" id="lyric-en"></div>
    <div class="lyric he" id="lyric-he"></div>
  </div>

  <div class="grid" id="grid"></div>

<script>
const ICONS = ${JSON.stringify(icons)};
const EN = ${JSON.stringify(LYRIC_WORDS)};
const HE = ${JSON.stringify(['שרים', 'באור', 'הזכוכית', 'הלילה'])};
const root = document.documentElement;
let currentType = ICONS[0].id;
let currentColor = ${JSON.stringify(DEFAULT_COLOR)};

function svgFor(id) {
  const ic = ICONS.find((x) => x.id === id);
  return ic ? ic.svg : '';
}
function setColor(hex) {
  currentColor = hex;
  root.style.setProperty('--lg-bouncer', hex);
  document.querySelectorAll('.swatch').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.color === hex)));
  document.getElementById('custom').value = hex;
}
function setType(id) {
  currentType = id;
  document.getElementById('type').value = id;
  document.querySelectorAll('.card').forEach((c) => c.setAttribute('aria-pressed', String(c.dataset.id === id)));
  paintLyrics();
}
function wordEl(text, typeId) {
  const el = document.createElement('span');
  el.className = 'word';
  el.dataset.word = text;
  el.innerHTML = '<span class="bo ' + typeId + '">' + svgFor(typeId) + '</span>' + text;
  return el;
}
function paintLyrics() {
  const en = document.getElementById('lyric-en');
  const he = document.getElementById('lyric-he');
  en.innerHTML = ''; he.innerHTML = '';
  EN.forEach((w) => en.appendChild(wordEl(w, currentType)));
  HE.forEach((w) => he.appendChild(wordEl(w, currentType)));
}
function paintGrid() {
  const grid = document.getElementById('grid');
  grid.innerHTML = '';
  ICONS.forEach((ic) => {
    const btn = document.createElement('button');
    btn.className = 'card';
    btn.dataset.id = ic.id;
    btn.setAttribute('aria-pressed', String(ic.id === currentType));
    btn.innerHTML = '<div class="thumb">' + ic.svg + '</div><h3>' + ic.name + '</h3>' +
      '<div class="sizes"><span class="s24">' + ic.svg + '</span><span class="s16">' + ic.svg + '</span>' +
      '<span class="s24 light">' + ic.svg + '</span><span class="s16 light">' + ic.svg + '</span></div>' +
      '<p>' + ic.description + '</p>';
    btn.addEventListener('click', () => setType(ic.id));
    grid.appendChild(btn);
  });
}
document.querySelectorAll('.swatch').forEach((b) => b.addEventListener('click', () => setColor(b.dataset.color)));
document.getElementById('custom').addEventListener('input', (e) => setColor(e.target.value));
document.getElementById('type').addEventListener('change', (e) => setType(e.target.value));

paintGrid();
setColor(currentColor);
setType(currentType);

const allWordParents = () => [...document.querySelectorAll('#lyric-en .word, #lyric-he .word')];
let idx = 0;
setInterval(() => {
  const words = allWordParents();
  if (!words.length) return;
  words.forEach((w) => w.classList.remove('active'));
  words[idx % words.length].classList.add('active');
  idx++;
}, 460);
</script>
</body>
</html>
`;
  fs.writeFileSync(path.join(DEMO_DIR, 'index.html'), html);
}

function exportPngs() {
  for (const b of bouncers) {
    const svg = wrap(b);
    renderSvg(svg, path.join(PNG_DIR, `${b.id}-96.png`), { width: 96 });
    renderSvg(svg, path.join(PNG_DIR, `${b.id}-24.png`), { width: 24 });
    renderSvg(svg, path.join(PNG_DIR, `${b.id}-16.png`), { width: 16 });
  }
}

function main() {
  ensureDirs();
  exportSvgs();
  writeReadme();
  writeDemo();
  exportPngs();

  const { svg, W, H, zoom } = buildPreviewSvg();
  const svgPath = path.join(PREVIEW_DIR, 'preview-sheet.svg');
  fs.writeFileSync(svgPath, svg);
  const rawPng = path.join(PREVIEW_DIR, '_build', '_preview-sheet-vector.png');
  renderSvg(svg, rawPng, { width: W });
  const metaPath = path.join(PREVIEW_DIR, '_build', '_zoom-meta.json');
  fs.writeFileSync(metaPath, JSON.stringify({ W, H, zoom, ids: bouncers.map((b) => b.id), names: bouncers.map((b) => b.name) }, null, 2));
  console.log('wrote svgs, readme, demo, pngs, vector preview');
  console.log(JSON.stringify({ svgPath, rawPng, metaPath, W, H, zoom }));
}

main();
