#!/usr/bin/env python3
"""Convert an image to the PaperDOS 1-bit packed framebuffer format.

Output layout: 800×480 pixels, 1 bit per pixel, MSB first.
  - Byte 0 bit 7 = pixel (0, 0)  [top-left]
  - Byte 0 bit 0 = pixel (7, 0)
  - Byte 100 bit 7 = pixel (0, 1)  [second row]
  - 1 = white, 0 = black
  - Total: 100 bytes/row × 480 rows = 48,000 bytes

Usage:
    uv run pdimage.py [--rotate {none,cw90,ccw90,180}] <input> <output.bin>
"""

import argparse
from pathlib import Path
from PIL import Image

PANEL_WIDTH = 800
PANEL_HEIGHT = 480
ROW_BYTES = PANEL_WIDTH // 8
FRAME_BYTES = ROW_BYTES * PANEL_HEIGHT  # 48,000


def apply_rotation(img: Image.Image, rotate: str) -> Image.Image:
    if rotate == "none":
        return img
    if rotate == "cw90":
        return img.rotate(-90, expand=True)
    if rotate == "ccw90":
        return img.rotate(90, expand=True)
    if rotate == "180":
        return img.rotate(180, expand=True)
    raise ValueError(f"unsupported rotation: {rotate}")


def convert(src: Path, dst: Path, rotate: str) -> None:
    img = Image.open(src).convert("RGB")
    img = apply_rotation(img, rotate)

    # Resize to panel dimensions with high-quality Lanczos filter.
    img = img.resize((PANEL_WIDTH, PANEL_HEIGHT), Image.Resampling.LANCZOS)

    # Convert to grayscale, then apply Floyd-Steinberg dithering to 1-bit.
    # PIL's "1" mode uses Floyd-Steinberg by default when converting from "L".
    img = img.convert("L")
    img = img.convert("1", dither=Image.Dither.FLOYDSTEINBERG)

    # Pack into our MSB-first 1bpp format.
    # PIL "1" images store 1 pixel per byte internally; we repack 8 per byte.
    buf = bytearray(FRAME_BYTES)
    pixels = img.load()
    if pixels is None:
        raise RuntimeError("failed to load image pixels")
    for y in range(PANEL_HEIGHT):
        for x in range(PANEL_WIDTH):
            px = pixels[x, y]  # 0 (black) or 255 (white) in PIL "1" mode
            if px:  # white → bit set
                byte_idx = y * ROW_BYTES + x // 8
                bit_mask = 0x80 >> (x % 8)
                buf[byte_idx] |= bit_mask

    dst.write_bytes(buf)
    print(f"Written {len(buf):,} bytes → {dst}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--rotate", choices=["none", "cw90", "ccw90", "180"], default="none"
    )
    parser.add_argument("input")
    parser.add_argument("output")
    args = parser.parse_args()
    convert(Path(args.input), Path(args.output), args.rotate)
