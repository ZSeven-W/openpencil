#!/usr/bin/env python3
"""Bake scene-template card previews for the Scene Template Center.

`previews.sh` renders one PNG per frame plus a wide overview; those are the
authoring artefacts — full resolution, arbitrary aspect. The template center
paints every card through one fixed rect with `ImageDrawMode::Fill`, so a
tall or ultra-wide source would be cropped to an unrecognisable strip.

This bakes each source onto the card aspect with the whole design still
visible (contain, not cover), padded with the document's own background
colour so the card reads as a framed thumbnail rather than a letterboxed
photo. Output matches the prompt-center convention: 640x400 JPEG.

Usage:
    python3 scene_preview_cards.py            # write into the UI crate
    python3 scene_preview_cards.py --check    # verify without writing
"""

from __future__ import annotations

import argparse
import pathlib
import sys

from PIL import Image

REPO = pathlib.Path(__file__).resolve().parents[3]
SRC = REPO / "templates" / "step0" / "previews"
DST = REPO / "crates" / "op-editor-ui" / "assets" / "scene_template_previews"

CARD_W, CARD_H = 640, 400
JPEG_QUALITY = 82
# Keep a small margin so the design never bleeds into the card's rounded
# corners — the panel clips to a 7px radius.
INSET = 12

# (card id, source preview). The overview render is preferred wherever a
# template has several frames: what makes a multi-page template legible at
# thumbnail size is seeing that it HAS pages, not reading one of them.
# (card id, source). A string names one render; a list is tiled into a grid
# whose column count is chosen to land near the card's own aspect — a 16:9
# deck laid out 1x6 is a 10:1 strip that shrinks each slide to ~100px inside
# the card, which reads as noise rather than as slides.
CARDS = [
    ("screenshot-tutorial", "screenshot-tutorial-overview.png"),
    ("knowledge-carousel", "knowledge-carousel-overview.png"),
    ("before-after", "before-after.png"),
    ("slide-deck", [f"slide-deck-{i:02d}.png" for i in range(1, 7)]),
]

# Gap between tiles, in source pixels — scaled down with everything else.
TILE_GAP = 48


def background_colour(image: Image.Image) -> tuple[int, int, int]:
    """The document's own backdrop, sampled from the render's corners.

    Every corner agreeing means the render has a uniform backdrop and we can
    extend it. Disagreement means the design bleeds to its edges, and any
    single sample would be an arbitrary tint — a neutral card wins there.
    """
    rgb = image.convert("RGB")
    w, h = rgb.size
    corners = [
        rgb.getpixel((0, 0)),
        rgb.getpixel((w - 1, 0)),
        rgb.getpixel((0, h - 1)),
        rgb.getpixel((w - 1, h - 1)),
    ]
    if all(c == corners[0] for c in corners):
        return corners[0]
    return (245, 245, 245)


def grid_columns(tile_aspect: float, count: int) -> int:
    """Column count whose tiled aspect sits closest to the card's.

    Chosen rather than fixed so a 16:9 deck tiles 3x2 while a 3:4 carousel
    tiles wide — the goal is filling the card, not a particular shape.
    """
    card_aspect = CARD_W / CARD_H
    best, best_error = count, float("inf")
    for columns in range(1, count + 1):
        rows = -(-count // columns)
        aspect = (columns * tile_aspect) / rows
        error = abs(aspect - card_aspect)
        if error < best_error:
            best, best_error = columns, error
    return best


def tile(sources: list[pathlib.Path]) -> Image.Image:
    images = [Image.open(path).convert("RGB") for path in sources]
    width = max(i.width for i in images)
    height = max(i.height for i in images)
    columns = grid_columns(width / height, len(images))
    rows = -(-len(images) // columns)
    canvas = Image.new(
        "RGB",
        (
            columns * width + (columns - 1) * TILE_GAP,
            rows * height + (rows - 1) * TILE_GAP,
        ),
        background_colour(images[0]),
    )
    for index, image in enumerate(images):
        column, row = index % columns, index // columns
        canvas.paste(image, (column * (width + TILE_GAP), row * (height + TILE_GAP)))
    return canvas


def bake(source: pathlib.Path | list[pathlib.Path]) -> Image.Image:
    image = tile(source) if isinstance(source, list) else Image.open(source)
    canvas = Image.new("RGB", (CARD_W, CARD_H), background_colour(image))
    fitted = image.convert("RGB")
    fitted.thumbnail((CARD_W - 2 * INSET, CARD_H - 2 * INSET), Image.LANCZOS)
    canvas.paste(
        fitted,
        ((CARD_W - fitted.width) // 2, (CARD_H - fitted.height) // 2),
    )
    return canvas


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    if not args.check:
        DST.mkdir(parents=True, exist_ok=True)

    failed = False
    for card_id, source_name in CARDS:
        names = source_name if isinstance(source_name, list) else [source_name]
        sources = [SRC / name for name in names]
        missing = [path for path in sources if not path.exists()]
        if missing:
            for path in missing:
                print(f"missing source: {path}", file=sys.stderr)
            failed = True
            continue
        source = sources if isinstance(source_name, list) else sources[0]
        target = DST / f"{card_id}.jpg"
        if args.check:
            status = "ok" if target.exists() else "MISSING"
            print(f"{status}: {target.relative_to(REPO)}")
            failed = failed or status == "MISSING"
            continue
        bake(source).save(target, "JPEG", quality=JPEG_QUALITY, optimize=True)
        print(f"{target.relative_to(REPO)}  {target.stat().st_size // 1024} KB")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
