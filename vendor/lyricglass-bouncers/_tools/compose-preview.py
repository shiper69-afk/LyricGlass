#!/usr/bin/env python3
"""Paste nearest-neighbour 16px zooms into the reserved slot on the preview sheet."""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path("/workspace/lyricglass-bouncers")
PREVIEW = ROOT / "preview"
SVG_DIR = ROOT / "svg"
RENDER = ["node", str(ROOT / "_tools" / "render-svg.js")]
FONT = "/usr/share/fonts/truetype/sand-box/google/Inter/Inter-VariableFont_opsz,wght.ttf"


def render_svg(src: Path, dest: Path, width: int) -> None:
    subprocess.check_call(RENDER + [str(src), str(dest), str(width)])


def main() -> None:
    meta = json.loads((PREVIEW / "_build" / "_zoom-meta.json").read_text())
    ids = meta["ids"]
    names = meta["names"]
    zoom_box = meta["zoom"]  # x, y, w, h in sheet pixels
    sheet_path = PREVIEW / "_build" / "_preview-sheet-vector.png"
    sheet = Image.open(sheet_path).convert("RGBA")

    # If the vector render isn't exactly W x H, scale coordinates.
    sx = sheet.width / meta["W"]
    sy = sheet.height / meta["H"]

    slot = (
        int(zoom_box["x"] * sx),
        int(zoom_box["y"] * sy),
        int(zoom_box["w"] * sx),
        int(zoom_box["h"] * sy),
    )

    font_name = ImageFont.truetype(FONT, 13)
    font_small = ImageFont.truetype(FONT, 11)

    nn = 8
    tile = 16 * nn  # 128
    cols = 6
    rows = 2
    gap_x = 24
    gap_y = 22
    label_h = 18
    pair_gap = 8  # between dark and light
    cell_w = tile
    cell_h = tile + pair_gap + tile + label_h

    grid_w = cols * cell_w + (cols - 1) * gap_x
    grid_h = rows * cell_h + (rows - 1) * gap_y
    left = slot[0] + (slot[2] - grid_w) // 2
    top = slot[1] + max(12, (slot[3] - grid_h) // 2)

    tmp = Path("/tmp/lg-zoom")
    tmp.mkdir(parents=True, exist_ok=True)

    # Dark fill already in the slot; we'll paint tiles on top.
    for i, (bid, name) in enumerate(zip(ids, names)):
        col, row = i % cols, i // cols
        png16 = tmp / f"{bid}-16.png"
        render_svg(SVG_DIR / f"{bid}.svg", png16, 16)
        src = Image.open(png16).convert("RGBA")
        if src.size != (16, 16):
            src = src.resize((16, 16), Image.Resampling.BILINEAR)
        z = src.resize((tile, tile), Image.Resampling.NEAREST)

        x = left + col * (cell_w + gap_x)
        y = top + row * (cell_h + gap_y)

        dark = Image.new("RGBA", (tile, tile), (7, 20, 27, 255))
        dark.alpha_composite(z)
        sheet.alpha_composite(dark, (x, y))

        light = Image.new("RGBA", (tile, tile), (232, 244, 246, 255))
        light.alpha_composite(z)
        sheet.alpha_composite(light, (x, y + tile + pair_gap))

        draw = ImageDraw.Draw(sheet)
        label = f"{name}"
        # center label under the pair
        bbox = draw.textbbox((0, 0), label, font=font_name)
        tw = bbox[2] - bbox[0]
        draw.text(
            (x + (tile - tw) / 2, y + tile + pair_gap + tile + 3),
            label,
            font=font_name,
            fill=(139, 184, 196, 255),
        )

    out = PREVIEW / "preview-sheet.png"
    sheet.convert("RGB").save(out, "PNG", optimize=True)
    print(f"wrote {out} {sheet.size}")


if __name__ == "__main__":
    main()
