import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

const FONT_PACKAGES = [
  ["rubik", "rubik"],
  ["heebo", "heebo"],
  ["assistant", "assistant"],
  ["inter", "inter"],
  ["varela-round", "varela-round"],
  ["nunito", "nunito"],
  ["secular-one", "secular-one"],
  ["bebas-neue", "bebas-neue"],
  ["anton", "anton"],
  ["frank-ruhl-libre", "frank-ruhl-libre"],
  ["playfair-display", "playfair-display"],
  ["amatic-sc", "amatic-sc"],
  ["caveat", "caveat"],
  ["pacifico", "pacifico"],
  ["press-start-2p", "press-start-2p"],
  ["monoton", "monoton"],
  ["bungee", "bungee"],
];

const outRoot = path.join(root, "licenses", "fonts");
fs.mkdirSync(outRoot, { recursive: true });

for (const [, pkg] of FONT_PACKAGES) {
  const src = path.join(root, "node_modules", "@fontsource", pkg, "LICENSE");
  const destDir = path.join(outRoot, pkg);
  fs.mkdirSync(destDir, { recursive: true });
  if (fs.existsSync(src)) {
    fs.copyFileSync(src, path.join(destDir, "OFL.txt"));
  }
}

console.log(`Copied ${FONT_PACKAGES.length} font licenses to licenses/fonts/`);