#!/usr/bin/env python3
"""Rasterise a `tmux capture-pane -e` dump to a PNG.

The headless --screenshot path only knows a crude glyph approximation, so UI
panels full of text are captured from a real terminal instead.
"""
import re
import sys

from PIL import Image, ImageDraw, ImageFont

CELL_W, CELL_H = 9, 18
FONT_CANDIDATES = [
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
]


def load_font():
    for path in FONT_CANDIDATES:
        try:
            return ImageFont.truetype(path, int(CELL_H * 0.82))
        except OSError:
            continue
    return ImageFont.load_default()


def parse(text):
    rows, row = [], []
    fg, bg = (255, 255, 255), (0, 0, 0)
    i = 0
    while i < len(text):
        c = text[i]
        if c == "\x1b":
            m = re.match(r"\x1b\[([0-9;:]*)m", text[i:])
            if m:
                params = [p for p in m.group(1).split(";") if p != ""]
                j = 0
                while j < len(params):
                    p = params[j]
                    if p in ("38", "48") and j + 4 < len(params) and params[j + 1] == "2":
                        colour = tuple(int(params[j + k]) for k in (2, 3, 4))
                        if p == "38":
                            fg = colour
                        else:
                            bg = colour
                        j += 5
                        continue
                    if p in ("0", ""):
                        fg, bg = (255, 255, 255), (0, 0, 0)
                    j += 1
                i += m.end()
                continue
            m = re.match(r"\x1b\[[0-9;?]*[A-Za-z]", text[i:])
            if m:
                i += m.end()
                continue
            i += 1
            continue
        if c == "\n":
            rows.append(row)
            row = []
        elif c != "\r":
            row.append((c, fg, bg))
        i += 1
    if row:
        rows.append(row)
    return rows


def main():
    src, dst = sys.argv[1], sys.argv[2]
    width = int(sys.argv[3]) if len(sys.argv) > 3 else 800
    with open(src, encoding="utf-8") as handle:
        rows = parse(handle.read())

    cols = max((len(r) for r in rows), default=0)
    img = Image.new("RGB", (cols * CELL_W, len(rows) * CELL_H))
    draw = ImageDraw.Draw(img)
    font = load_font()
    for y, row in enumerate(rows):
        for x, (ch, fg, bg) in enumerate(row):
            ox, oy = x * CELL_W, y * CELL_H
            if ch == "▀":
                draw.rectangle([ox, oy, ox + CELL_W - 1, oy + CELL_H // 2 - 1], fill=fg)
                draw.rectangle([ox, oy + CELL_H // 2, ox + CELL_W - 1, oy + CELL_H - 1], fill=bg)
            elif ch == "█":
                draw.rectangle([ox, oy, ox + CELL_W - 1, oy + CELL_H - 1], fill=fg)
            else:
                draw.rectangle([ox, oy, ox + CELL_W - 1, oy + CELL_H - 1], fill=bg)
                if ch.strip():
                    draw.text((ox, oy), ch, font=font, fill=fg)

    if width and img.width > width:
        height = round(img.height * width / img.width)
        img = img.resize((width, height), Image.LANCZOS)
    img.save(dst)


if __name__ == "__main__":
    main()
