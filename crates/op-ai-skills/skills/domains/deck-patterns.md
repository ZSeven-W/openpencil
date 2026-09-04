---
name: deck-patterns
description: Slide-pattern skeletons for presentation decks — board placement, cover/data/timeline/table/points/quote/closing structures with measured numbers
phase: [generation]
trigger:
  keywords: [slide, slides, deck, presentation, pitch deck, keynote, ppt, 幻灯片, 演示, 演示文稿, 路演, 课件, 汇报]
priority: 22
budget: 2000
category: domain
---

DECK PATTERNS — SLIDE SKELETONS

Companion to the `slides` skill: that one picks the tier and the layout contract, this one is the structure to emit. Colour names below are roles from the chosen tier (`bg` / `surface` / `ink` / `muted` / `accent` / `accent-soft` / `border`). The numbers are measured from shipped decks — copy them, do not re-derive.

## Board placement

Each slide is its own top-level 1920×1080 frame and MUST carry explicit `x`/`y`; without them every board lands at the origin and the deck renders as one slide with the rest hidden underneath. 3 boards per row: `x = (i % 3) * (1920 + 120)`, `y = (i / 3) * (1080 + 360)`. The row gap is 360, not 120, because the canvas paints each frame's NAME above it at a fixed screen-space offset — at the zoom where a 3-wide deck fits the screen, 120px is ~16 screen px and every second-row label collides with the row above.

## Slide frame

```
frame(name="01 封面", x, y, width=1920, height=1080, layout="vertical",
      padding=[120,120], gap=48..56, justifyContent="start",
      alignItems="start", fill=[bg], clipContent=true)
```

Never `fit_content` on a board — the artboard is the projector. Padding 120 satisfies the ≥100 safe area on all four sides.

## Keep page titles on one line across the deck

A page-title block followed by a transparent `height="fill_container"`, `justifyContent="center"` wrapper holding the body. The title sits at the same y on every slide (repetition), while the body stays vertically centred instead of being stretched.

```
frame(slide) ├── 页头 (vertical, gap=18)  ├── text(title, 64, 700, ink, lh=1.15)
             │                            └── text(subtitle, 30, 400, muted, lh=1.45)
             └── frame(fill_container, justifyContent="center", gap=0) └── <body>
```

## Cover — three forms, pick one

- **Left-aligned + sign-off** (default): slide `justifyContent="space_between"`, children = eyebrow pill, lede column, meta row. Lede = title 100–112/700 lh 1.12 (`\n` for the intended break) + accent bar + subtitle 32–34/400 lh 1.45, gap 36. Meta row `alignItems="center"`, `justifyContent="space_between"`: speaker 28/500 ink, occasion/date 28/400 muted.
- **Centred claim**: slide `justifyContent="center"`, one column gap 32 — eyebrow, title 104/700, subtitle 34/400 muted, accent bar.
- **Oversized statement** (S4 only): title 120–140/700 and nothing but one 30–34/400 line.

Eyebrow pill: `frame(fit_content, padding=[8..10, 18..22], cornerRadius=999, fill=[accent-soft])` wrapping `text(24..26, 600, accent, letterSpacing=1.0..1.2)`. Accent bar: `rect(width=120..160, height=8..10, cornerRadius=999, fill=[accent])`. Use at most ONE pill per slide.

## Data page — the number is the slide

3 KPI cards in a row, `alignItems="stretch"`, gap 36.

```
card = frame(vertical, height="fill_container", justifyContent="center",
             padding=[56,44], cornerRadius=24, gap=18..20, fill=[surface])
  ├── row(gap=10, alignItems="end", width="fit_content")
  │     ├── text(value, 140, 700, accent, lh=1.0, width="fit_content")
  │     └── frame(fit_content, padding=[0,0,19,0]) └── text(unit, 44, 600, accent, lh=1.0)
  ├── text(label, 32, 600, ink, lh=1.25)
  └── text(note, 26, 400, muted, lh=1.45)
```

The unit's `19` bottom padding is a baseline compensation: `round((valueSize - unitSize) * 0.2)`. There is no real baseline alignment (`alignItems:"baseline"` folds to `end`), so without it a 44px unit sits below the 140px digits. Recompute it whenever either size changes. One KPI → 120–200/700 centred; two → 80–120.

## Points page — number, title, description

Three layers per item, never two: `序号 + 标题 + 说明`.

- **Numbered circle**: `frame(width=size, height=size, cornerRadius=size/2, layout="horizontal", alignItems="center", justifyContent="center", fill=[accent])` + `text(digit, round(size*0.46), 700, "#FFFFFF", lh=1.0)`. size 52 inline, 64 as a card leading. Planned-but-not-done → `fill=[surface]`, ink `accent`, `stroke(accent, 3)`.
- **Bullet dot**: `rect(14×14, cornerRadius=7, fill=[accent])` inside a `fit_content` column with `padding=[15,0,0,0]` — the lift is `round(fontSize * lineHeight / 2 - dotSize / 2)`, which drops the dot onto the first line's optical centre instead of its top edge.
- Card variant: each item is `padding=[36..48, 36..44], cornerRadius=20, fill=[surface]`, item gap 24, text column gap 12. Title 34–40/600 lh 1.2–1.25, description 27–28/400 lh 1.5–1.55.
- 3 items per row when horizontal (`alignItems="stretch"`, gap 36, each `height="fill_container"`), 3–4 stacked when vertical.

## Timeline

A row of equal columns with **gap 0** — the axis is assembled from each column's own segment, and any column gap breaks it into disconnected pieces. Breathing room comes from a right inset on the text, never from the row gap.

```
column(gap=28)
  ├── 节点日期  frame(padding=[0,56,0,0]) └── text(date, 28, 600, accent, lh=1.2)
  ├── 轴段      row(gap=0, alignItems="center")
  │               ├── rect(28×28, cornerRadius=14, fill=[accent])          -- done
  │               │   (planned: fill=[bg] + stroke(accent, 4))
  │               └── rect(width="fill_container", height=4, fill=[border])
  └── 节点文案  frame(padding=[0,56,0,0], gap=10)
                  ├── text(title, 34, 600, ink, lh=1.25)
                  └── text(desc, 26, 400, muted, lh=1.5)
```

Never put padding on the axis row itself. 4–5 nodes maximum.

## Comparison table

Rules only, via per-side stroke — a full box turns the table into a grid and hides the row relationships.

- Header row: `padding=[26,32]`, `stroke={thickness:{bottom:3}, fill:[accent]}`, column headings 34/700 accent.
- Body row: same padding, `stroke={thickness:{bottom:2}, fill:[border]}`. **The last row carries no stroke** — a trailing rule dangles under nothing.
- Row-label column: fixed `width=260`, 27/600 muted. Content columns fill, 28/400 ink lh 1.5. Row gap 40 between cells, 0 between rows.
- Wrapper: `frame(vertical, gap=0, cornerRadius=20, fill=[surface], clipContent=true)`. The header needs a blank label cell (`text(" ")`) so its columns line up with the body.

## Chart placeholder

`frame(horizontal, gap=28, alignItems="end", padding=[40,40], cornerRadius=24, fill=[surface], height="fill_container")` holding 5 `rect(width="fill_container", cornerRadius=12)` bars at heights 180/250/220/330/420. Exactly ONE bar — the one the takeaway is about — uses `accent`, the rest `accent-soft`. Pair it with a `width=520` notes column of 2–3 title 30/600 + body 26/400 pairs: the insight goes in words, never left for the audience to find.

### Bar chart geometry

- **Bottom baseline** — bars align to same bottom; set `alignItems="end"` so heights float upward.
- **Height ∝ value** — if value B is 2× value A, so is height B.
- **Flex layout** — use `frame(horizontal, alignItems="end")` for auto-distribution; never `layout="none"` with manual x/y.
- **Axis at bottom** — x-axis sits below bars, not above.
- **No empty frames** — frame + fill color alone is a defect; every placeholder needs visible content.

## Closing / CTA

`justifyContent="space_between"`: a lede column (accent bar, headline 88–96/700 lh 1.15, one 30–32/400 muted line) and a contact card `row(padding=[48,56], cornerRadius=24, fill=[surface], justifyContent="space_between", alignItems="center")`. Left = name 32/600 over a nested column of 30/500 accent + 30/400 muted lines (nesting keeps each level's sizes internally consistent). Right = a `200×200` `frame` placeholder, `cornerRadius=16`, `fill=[accent-soft]`, `stroke(accent, 2)`, with a centred 26/600 label inside.

Placeholders for a QR code, a photo or a chart must be `frame`, never `rectangle`: a rectangle does not render its children, so the label inside a rectangle placeholder is invisible.
