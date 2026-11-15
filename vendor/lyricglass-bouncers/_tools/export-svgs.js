'use strict';

const fs = require('fs');
const path = require('path');
const { bouncers, wrap } = require('./svgs');

const root = path.join(__dirname, '..');
const svgDir = path.join(root, 'svg');
fs.mkdirSync(svgDir, { recursive: true });

for (const b of bouncers) {
  const svg = wrap(b);
  const file = path.join(svgDir, `${b.id}.svg`);
  fs.writeFileSync(file, svg);
  console.log('wrote', file);
}
console.log('exported', bouncers.length, 'svgs');
