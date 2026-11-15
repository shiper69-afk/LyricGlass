'use strict';

const fs = require('fs');
const { Resvg } = require('/workspace/lyricglass-icons/_tools/node_modules/@resvg/resvg-js');

const FONT_FILES = [
  '/usr/share/fonts/truetype/sand-box/google/Inter/Inter-VariableFont_opsz,wght.ttf',
  '/usr/share/fonts/truetype/sand-box/google/Outfit/Outfit-VariableFont_wght.ttf',
  '/usr/share/fonts/truetype/sand-box/google/IBM Plex Sans Hebrew/IBMPlexSansHebrew-Regular.ttf',
  '/usr/share/fonts/truetype/sand-box/google/IBM Plex Sans Hebrew/IBMPlexSansHebrew-Medium.ttf',
  '/usr/share/fonts/truetype/sand-box/google/IBM Plex Sans Hebrew/IBMPlexSansHebrew-Bold.ttf',
  '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
  '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf',
];

function renderSvg(svg, outPath, { width, height, background } = {}) {
  const opts = {
    font: {
      fontFiles: FONT_FILES,
      loadSystemFonts: true,
      defaultFontFamily: 'Inter',
    },
    shapeRendering: 2, // geometricPrecision / crisp
  };
  if (background) opts.background = background;
  if (width && height) {
    opts.fitTo = { mode: 'zoom', value: 1 };
    // If the SVG has its own size, we want to override. Wrap or set fitTo.
  }
  if (width && !height) opts.fitTo = { mode: 'width', value: width };
  else if (height && !width) opts.fitTo = { mode: 'height', value: height };
  else if (width && height) opts.fitTo = { mode: 'width', value: width };

  const r = new Resvg(svg, opts);
  const png = r.render().asPng();
  fs.writeFileSync(outPath, png);
  return outPath;
}

function renderFile(inPath, outPath, opts) {
  return renderSvg(fs.readFileSync(inPath, 'utf8'), outPath, opts);
}

if (require.main === module) {
  const [,, inPath, outPath, width] = process.argv;
  if (!inPath || !outPath) {
    console.error('usage: render-svg.js <in.svg> <out.png> [width]');
    process.exit(1);
  }
  renderFile(inPath, outPath, { width: width ? Number(width) : undefined });
}

module.exports = { renderSvg, renderFile, FONT_FILES };
