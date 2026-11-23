"""Draw the Seamless Download Manager icon the way every Mocha app icon is drawn: the mark in
crust #11111B on the blue to mauve tile (corner 0.26 of the side, diagonal blend #89B4FA to
#CBA6F7), the same tile as the Mocha Steam, T3, Ableton and Yapfast icons and the tray badges.
The mark is a down arrow into a tray, thick enough to read at taskbar size.

Writes app/assets/sdm.ico (16 to 256), app/assets/icon-128.png and docs/icon-512.png."""
import io
import os
import struct

import resvg_py
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]

MARK = (
    # Arrow: shaft and head as one shape, round joins so small sizes stay solid.
    '<path d="M18 7.2 V19.6 M12.4 14.2 L18 19.8 L23.6 14.2" fill="none" stroke="#11111B" '
    'stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"/>'
    # Tray under it.
    '<path d="M9.6 21.2 V25.4 Q9.6 27.6 11.8 27.6 H24.2 Q26.4 27.6 26.4 25.4 V21.2" fill="none" '
    'stroke="#11111B" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"/>'
)


def svg(with_mark=True):
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 36">'
        '<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">'
        '<stop offset="0" stop-color="#89B4FA"/><stop offset="1" stop-color="#CBA6F7"/>'
        '</linearGradient></defs><rect width="36" height="36" rx="9.36" fill="url(#g)"/>'
        + (MARK if with_mark else "") + "</svg>"
    )


def render(size):
    png = bytes(resvg_py.svg_to_bytes(svg_string=svg(), width=size, height=size))
    return Image.open(io.BytesIO(png)).convert("RGBA")


def write_ico(images, path):
    blobs = []
    for im in images:
        b = io.BytesIO()
        im.save(b, "PNG")
        blobs.append(b.getvalue())
    head = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    for im, blob in zip(images, blobs):
        w = im.width if im.width < 256 else 0
        head += struct.pack("<BBBBHHII", w, w, 0, 0, 1, 32, len(blob), offset)
        offset += len(blob)
    with open(path, "wb") as f:
        f.write(head + b"".join(blobs))


if __name__ == "__main__":
    ims = [render(s) for s in SIZES]
    write_ico(ims, os.path.join(ROOT, "app", "assets", "sdm.ico"))
    render(128).save(os.path.join(ROOT, "app", "assets", "icon-128.png"))
    os.makedirs(os.path.join(ROOT, "docs"), exist_ok=True)
    render(512).save(os.path.join(ROOT, "docs", "icon-512.png"))
    # Proof sheet: every size, 3x zoom of the small ones, next to the T3 Mocha icon.
    sheet = Image.new("RGBA", (900, 200), (0x1E, 0x1E, 0x2E, 255))
    x = 10
    for im in ims[:6]:
        sheet.alpha_composite(im, (x, 10))
        sheet.alpha_composite(im.resize((im.width * 3, im.height * 3), Image.NEAREST), (x, 50))
        x += im.width * 3 + 10
    sheet.alpha_composite(ims[-2], (x + 10, 10))
    t3 = r"H:\Windhawk Configuration\work\2026-11-22-app-icons\T3-Mocha-128.png"
    if os.path.exists(t3):
        sheet.alpha_composite(Image.open(t3).convert("RGBA"), (x + 150, 10))
    sheet.save(os.path.join(ROOT, "docs", "icon-sheet.png"))
    print("ok")
