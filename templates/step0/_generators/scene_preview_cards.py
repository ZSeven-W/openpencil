#!/usr/bin/env python3
"""Bake scene-template card previews for the Scene Template Center.

`previews.sh` renders one PNG per frame plus a wide overview; those are the
authoring artefacts — full resolution, arbitrary aspect. The template center
paints every card through one fixed rect with `ImageDrawMode::Fill`, so a
tall or ultra-wide source would be cropped to an unrecognisable strip.

This bakes each source onto the card aspect with the whole design still
visible (contain, not cover), padded with the document's own background
colour so the card reads as a framed thumbnail rather than a letterboxed
photo. Output is a 16:10 JPEG at 2x the largest size the gallery paints; see
CARD_W/CARD_H below for how that size is derived.

Usage:
    python3 scene_preview_cards.py            # write into the UI crate
    python3 scene_preview_cards.py --check    # verify without writing
"""

from __future__ import annotations

import argparse
import pathlib
import sys
from typing import NamedTuple

from PIL import Image


class Top(NamedTuple):
    """Take the card's aspect off the top of this render, full width."""

    name: str


REPO = pathlib.Path(__file__).resolve().parents[3]
SRC = REPO / "templates" / "step0" / "previews"
DST = REPO / "crates" / "op-editor-ui" / "assets" / "scene_template_previews"

# 2x the widest preview the gallery can paint, and no more — every byte here
# is embedded in the wasm bundle, which has a hard ceiling.
#
# The Asset Center is a full-canvas gallery now, so the card width is derived
# rather than fixed. Its maximum falls out of the column breakpoints: a
# two-column grid tops out just under 1000px of viewport, giving a ~490px card
# and a ~470px preview, so 940 device px is the worst case on a 2x display.
# The old 640px bake was sized for a 720px dialog and went visibly soft.
CARD_W, CARD_H = 1024, 640
JPEG_QUALITY = 82
# Keep a small margin so the design never bleeds into the card's rounded
# corners — the panel clips to a 9px radius.
INSET = 20

# (card id, source preview). The overview render is preferred wherever a
# template has several frames: what makes a multi-page template legible at
# thumbnail size is seeing that it HAS pages, not reading one of them.
# (card id, source). A string names one render; a list is tiled into a grid
# whose column count is chosen to land near the card's own aspect — a 16:9
# deck laid out 1x6 is a 10:1 strip that shrinks each slide to ~100px inside
# the card, which reads as noise rather than as slides. `Top(name)` takes the
# card's own aspect off the top of a very tall render instead of fitting the
# whole thing: a 1:4.5 marketing page contained inside a 16:10 card is a
# 140px-wide sliver, where its masthead and hero at full width are exactly
# what makes it recognisable.
#
# 多板模板一律走帧列表，**不要**拿 `<id>-overview.png` 当源：overview 是整画布
# 导出，板与板之间的间隙和最后一行缺的格子都是画布底色，缩进卡片里就是几道黑
# 条。帧列表这条路由 `tile()` 自己按文档底色铺画布（`background_colour` 从首帧
# 四角取），缺的格子跟着一起变成文档底色，所以永远不会露黑。
CARDS = [
    ("screenshot-tutorial",
     [f"screenshot-tutorial-{index:02d}.png" for index in range(1, 6)]),
    ("knowledge-carousel",
     [f"knowledge-carousel-{index:02d}.png" for index in range(1, 6)]),
    ("before-after", "before-after.png"),
    ("slide-deck", [f"slide-deck-{i:02d}.png" for i in range(1, 7)]),
    ("knowledge-card-vertical", "knowledge-card-vertical.png"),
    ("knowledge-card-square", "knowledge-card-square.png"),
    ("pitch-deck-dark", [f"pitch-deck-dark-{i:02d}.png" for i in range(1, 7)]),
    ("lecture-deck-light",
     [f"lecture-deck-light-{i:02d}.png" for i in range(1, 7)]),
    ("minimal-keynote", [f"minimal-keynote-{i:02d}.png" for i in range(1, 10)]),
    ("gradient-tech", [f"gradient-tech-{i:02d}.png" for i in range(1, 7)]),
    ("saas-landing-orange", Top("saas-landing-orange.png")),
    ("product-landing-light", Top("product-landing-light.png")),
    ("punch-quote-card", "punch-quote-card.png"),
    ("journal-checklist-card", "journal-checklist-card.png"),
    # The infographics are 1:2.5 and 1:3 — contained in a 16:10 card each
    # would be a ~250px sliver. `Top` keeps the masthead and the first
    # section at full width, which is what makes a long-form graphic
    # recognisable as one.
    ("data-report-infographic", Top("data-report-infographic.png")),
    ("steps-flow-infographic", Top("steps-flow-infographic.png")),
    ("pitfall-list-infographic", Top("pitfall-list-infographic.png")),
    ("spine-culture-card", "spine-culture-card.png"),
    ("metric-single-card", "metric-single-card.png"),
    ("quote-frame-card", "quote-frame-card.png"),
    ("daily-sign-card", "daily-sign-card.png"),
    ("price-tier-card", "price-tier-card.png"),
    ("notice-board-card", "notice-board-card.png"),
    ("milestone-timeline-infographic", Top("milestone-timeline-infographic.png")),
    ("concept-contrast-infographic", Top("concept-contrast-infographic.png")),
    ("ranking-board-infographic", Top("ranking-board-infographic.png")),
    ("faq-thread-infographic", Top("faq-thread-infographic.png")),
    ("data-story-infographic", Top("data-story-infographic.png")),
    ("challenge-tracker-infographic", Top("challenge-tracker-infographic.png")),
    ("ecosystem-map-infographic", Top("ecosystem-map-infographic.png")),
    ("do-dont-comparison", "do-dont-comparison.png"),
    ("myth-truth-comparison", Top("myth-truth-comparison.png")),
    ("pricing-tiers-comparison", "pricing-tiers-comparison.png"),
    ("scenario-guide-comparison", Top("scenario-guide-comparison.png")),
    ("spec-table-comparison", Top("spec-table-comparison.png")),
    ("three-way-comparison", Top("three-way-comparison.png")),
    ("time-shift-comparison", "time-shift-comparison.png"),
    ("tradeoff-scale-comparison", "tradeoff-scale-comparison.png"),
    ("version-diff-comparison", "version-diff-comparison.png"),
    ("app-onboarding-triptych", "app-onboarding-triptych.png"),
    ("diy-blueprint-guide", Top("diy-blueprint-guide.png")),
    ("photo-composition-tutorial",
     [f"photo-composition-tutorial-{index:02d}.png" for index in range(1, 6)]),
    ("recipe-four-step", "recipe-four-step.png"),
    ("skincare-routine-cards",
     [f"skincare-routine-cards-{index:02d}.png" for index in range(1, 7)]),
    ("software-step-tutorial", "software-step-tutorial.png"),
    ("storage-makeover-steps",
     [f"storage-makeover-steps-{index:02d}.png" for index in range(1, 7)]),
    ("weekly-report-lesson", Top("weekly-report-lesson.png")),
    ("workout-breakdown-guide", Top("workout-breakdown-guide.png")),
    ("bookreview-silk-carousel",
     [f"bookreview-silk-carousel-{index:02d}.png" for index in range(1, 6)]),
    ("cityguide-film-carousel",
     [f"cityguide-film-carousel-{index:02d}.png" for index in range(1, 8)]),
    ("datareport-grid-carousel",
     [f"datareport-grid-carousel-{index:02d}.png" for index in range(1, 7)]),
    ("opinion-longform-carousel",
     [f"opinion-longform-carousel-{index:02d}.png" for index in range(1, 7)]),
    ("qa-chalkboard-carousel",
     [f"qa-chalkboard-carousel-{index:02d}.png" for index in range(1, 7)]),
    ("story-night-carousel",
     [f"story-night-carousel-{index:02d}.png" for index in range(1, 8)]),
    ("toolkit-notebook-carousel",
     [f"toolkit-notebook-carousel-{index:02d}.png" for index in range(1, 7)]),
    ("tutorial-journal-carousel",
     [f"tutorial-journal-carousel-{index:02d}.png" for index in range(1, 7)]),
    ("yearreview-mineral-carousel",
     [f"yearreview-mineral-carousel-{index:02d}.png" for index in range(1, 9)]),
    ("event-poster-deck",
     [f"event-poster-deck-{i:02d}.png" for i in range(1, 7)]),
    ("sounding-navy-deck",
     [f"sounding-navy-deck-{i:02d}.png" for i in range(1, 8)]),
    ("tidemark-slate-deck",
     [f"tidemark-slate-deck-{i:02d}.png" for i in range(1, 8)]),
    ("banxin-rule-deck",
     [f"banxin-rule-deck-{i:02d}.png" for i in range(1, 8)]),
    ("gridpaper-graphite-deck",
     [f"gridpaper-graphite-deck-{i:02d}.png" for i in range(1, 9)]),
    ("dossier-linen-deck",
     [f"dossier-linen-deck-{i:02d}.png" for i in range(1, 9)]),
    ("ledger-tick-deck",
     [f"ledger-tick-deck-{i:02d}.png" for i in range(1, 8)]),
    ("brand-concept-sheet", "brand-concept-sheet.png"),
    ("logo-qa-board", "logo-qa-board.png"),
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


def crop_to_card_top(source: pathlib.Path) -> Image.Image:
    """Full-width band off the top of a render, already at the card's aspect.

    Returned pre-cropped so the shared fit path below only ever scales it —
    the band is the whole picture, so it fills the card edge to edge with no
    letterboxing to pad.
    """
    image = Image.open(source).convert("RGB")
    band = min(image.height, round(image.width * CARD_H / CARD_W))
    return image.crop((0, 0, image.width, band))


def bake(source: pathlib.Path | list[pathlib.Path] | Top) -> Image.Image:
    if isinstance(source, Top):
        image = crop_to_card_top(SRC / source.name)
    elif isinstance(source, list):
        image = tile(source)
    else:
        image = Image.open(source)
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
    parser.add_argument("card_ids", nargs="*")
    args = parser.parse_args()

    known_ids = {card_id for card_id, _ in CARDS}
    unknown_ids = sorted(set(args.card_ids) - known_ids)
    if unknown_ids:
        parser.error(f"unknown card id(s): {', '.join(unknown_ids)}")
    requested = set(args.card_ids)
    cards = [card for card in CARDS if not requested or card[0] in requested]

    if not args.check:
        DST.mkdir(parents=True, exist_ok=True)

    failed = False
    for card_id, source_name in cards:
        if isinstance(source_name, Top):
            names = [source_name.name]
        elif isinstance(source_name, list):
            names = source_name
        else:
            names = [source_name]
        sources = [SRC / name for name in names]
        missing = [path for path in sources if not path.exists()]
        if missing:
            for path in missing:
                print(f"missing source: {path}", file=sys.stderr)
            failed = True
            continue
        if isinstance(source_name, Top):
            source = source_name
        elif isinstance(source_name, list):
            source = sources
        else:
            source = sources[0]
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
