---
name: slides
description: Presentation slide / deck design — 16:9 layout contracts, slide typography, one-idea-per-slide
phase: [generation]
trigger:
  keywords: [slide, slides, deck, presentation, pitch deck, keynote, ppt, 幻灯片, 演示, 演示文稿, 路演]
priority: 24
budget: 2400
category: domain
---

SLIDE / DECK DESIGN

You design slides readable in real conditions (projector, Zoom, mobile). Priority: Clarity > Readability > Hierarchy > Simplicity. Slides are visual aids, NOT documents.

## Adapt the style guide (critical, first)

The selected style guide is a brand/product palette — it is NOT slide-optimized. ALWAYS adapt it for slides: scale type up to the sizes below, widen spacing, raise contrast, simplify. If the guide's body size or contrast would hurt readability on a projector, override it. Readability beats brand fidelity every time. Pull core/accent/neutral from the guide, then enforce the slide sizes regardless of the guide's own scale.

## Style tiers — pick ONE for the whole deck, then hold it on every slide

Route the tier from the request (rules below), then use that tier's palette, type scale and element cap verbatim. When a style guide IS selected, keep its accent HUE but move the values onto the tier's roles so the contrast floors below still hold. Hex values and contrast ratios here are measured, not estimates — the listed floor pair is the one that fails first when you swap the accent, so re-measure THAT pair, never the title.

S1 WARM-WHITE BUSINESS — default. Reports, quarterly reviews, corporate updates, 汇报.

- bg `#FFFFFF` · surface `#F4F6F9` · ink `#0B1220` · muted `#5A6B85` · accent `#2F5BEA` · accent-soft `#E4EAFD` · border `#DCE2EC`
- Contrast: ink/bg 18.72 · muted/bg 5.42 · accent/bg 5.52 · muted/accent-soft 4.51 (FLOOR)
- Type: cover 104/700 · page title 64/700 · card title 34–40/600 · body 26–32/400 · KPI 140/700 · eyebrow 24/600
- ≤6 elements per slide. Cards carry the `surface` fill; the slide background stays `bg`.
- NEVER: a second accent hue; a coloured page background on anything but the closing slide.

S2 DARK PITCH — 深色 / 暗色 / dark / 科技感 / tech / 路演 / investor pitch.

- bg `#0B1220` · surface `#18263F` · ink `#F2F6FF` · muted `#93A4C4` · accent `#4D8DFF` · accent-soft `#16233D` · border `#24314D`
- Contrast: ink/bg 17.30 · muted/bg 7.44 · accent/bg 5.86 · accent/surface 4.73 (FLOOR)
- Type: cover 88–112/700 lh 1.12 · page title 64/700 · card title 38/600 · body 26–30/400 · KPI 140/700 with its unit at 44/600
- EXACTLY TWO background levels (bg + surface). ≤5 elements per slide.
- NEVER: a third grey plane — on dark, hierarchy comes from weight and the accent, not from stacking more greys. NEVER body text lighter-weight than `muted`, drop shadows, or a second accent.

S3 LIGHT LECTURE — 课件 / 教学 / 培训 / lecture / course / tutorial / workshop.

- bg `#F2EEE2` (paper, deliberately not pure white — pure white lights the whole room) · surface `#FFFFFF` · ink `#17211C` · muted `#55635A` · accent `#1B6B4C` · accent-soft `#D8E9DE` · border `#D9D2C2`
- Contrast: ink/bg 14.25 · accent/bg 5.56 · white-on-accent 6.45 · muted/accent-soft 5.01 (FLOOR)
- Type: cover 100/700 · page title 64/700 · step/objective title 30–38/600 · body 26–30/400 at lineHeight 1.5–1.6 (denser than a pitch — the audience is taking notes)
- ≤8 elements per slide. The numbered circle is this tier's signature: a filled accent circle with a white digit is the highest-contrast point on the page.
- NEVER: a procedure slide with no visible step order; body below 26.

S4 MINIMAL KEYNOTE — 极简 / 简约 / minimal / keynote / "one big idea".

- Pick ONE ground and never mix: S1's `#FFFFFF`/`#0B1220` pair, or S2's `#0B1220`/`#F2F6FF` pair. Exactly one accent, used at most once per slide.
- Type: statement 88–140/700 lh 1.12 · one supporting line 30–34/400. Nothing else.
- ≤4 elements per slide, counting the accent bar.
- NEVER: bullet lists, cards, borders, tables, icons, or a slide carrying title + subtitle + body + footer at once.

## Route the tier from the request

Scan the user's words in this order and take the FIRST hit; scan the deck's subject only if no style word appears:

1. 深色/暗色/黑色/dark/night/科技感/tech/cyber/neon → S2. 路演/pitch/投资人/investor/融资 also → S2.
2. 极简/简约/minimal/keynote/性冷淡/one big idea/less is more → S4.
3. 课件/教学/讲义/培训/lecture/course/tutorial/workshop/教程 → S3.
4. 浅色/明亮/light/白底/商务/汇报/季度/年度/report/review/corporate → S1.
5. No style word at all → S1.

An explicit brand colour in the request overrides the tier's accent (keep every other role). A tier is a whole-deck decision — never switch tiers between slides of one deck.

## Format

- Each slide is a 16:9 frame, 1920×1080. Keep all content ≥100px from the edges.
- MARGIN FLOOR: slide root horizontal padding ≥64px (1080-wide card roots ≥48px); no text may touch the canvas edge — a full-bleed background image is the only exception.
- Multiple slides = multiple sibling frames laid left-to-right on the canvas.

## Core rules

- ONE idea per slide. The title states the takeaway, not the topic.
- If content doesn't fit at the required sizes: SPLIT or REMOVE. Never shrink fonts to fit.
- Short phrases, not sentences. No paragraphs. Details go to notes/appendix.
- Consistency > creativity. Reduce cognitive load. Apply CRAP (Contrast, Repetition, Alignment, Proximity).

## Typography (non-negotiable)

- Max 2 font families. Use WEIGHT for hierarchy, not many sizes.
- Body ≥24px (prefer 28–32). Titles ≥40px. Key numbers can be 80–200px.
- Line-height 1.1–1.2. High contrast always. Avoid ALL CAPS except small labels.

## Color

- 2–3 core colors + neutrals (from the selected style guide). Accent only for emphasis. Body text neutral. High contrast text/bg mandatory.
- When the document carries the semantic palette, prefer refs over hex: slide bg `$color-bg-deep` (or `$color-surface` for light decks), titles `$color-text-primary`, body/sub `$color-text-body`, labels/meta/muted-before `$color-text-muted`, the one emphasis/after/KPI accent `$color-accent`. Full-bleed hero overlays use `$color-scrim`. Otherwise use guide hex.

## Visuals & data

- Visuals support meaning, not decoration. Charts > text for data. One insight per chart; highlight the key datapoint; no chart-junk. Icons consistent size/style.

## Layout contracts (pick one per slide; `intent — grid — content@size`)

- L01 Cover — center stack — Title 48–64 bold + Subtitle 28–32 + Meta 20–24
- L02 Bold cover — left block — Title 56–72 (≤2 lines) + Subtitle 28, left margin ~120, logo bottom-right
- L03 Section break — center — Label 24 muted + Title 48–56 (only these two)
- L04 Key statement — center — Statement 36–48 (≤2 lines) + optional attribution 24
- L05 Concept+visual — 2col 50/50 — left Title 36–40 + Body 24–28 (≤4 lines), right image, gap ≥40
- L06 Concept+visual (mirror) — 2col — image left, text right
- L07 Three pillars — 3col — each: visual + Label 28 + Desc 20 (≤2 lines), equal width
- L08 Compare two — 2col — each: Heading 28–32 + 2–4 points @24
- L09 Single KPI — center stack — Label 24 muted + Number 120–200 + Context 24–28 (number is hero)
- L10 Two KPIs — 2col — Number 80–120 + Label 24, equal weight
- L11 Three KPIs — 3col — Number 64–80 + Label 24, same baseline
- L12 Quote — center stack — Quote 28–36 (≤3 lines) + Attribution 20–24
- L13 Process — row of 3–5 steps — icon/number + Label 28 + Desc 20 (1 line), equal spacing
- L14 Hero image — full bleed — overlay Title 40–56 + Subtitle 24–28, dark overlay for contrast
- L15 Matrix — 2×2 — each: Heading 28 + Desc 20, equal cards
- L16 Icon row — row of 3–4 — icon + Label 28 + Desc 20 (1–2 lines), same icon size
- L17 Data + insight — stack — chart ~60% height + Insight 24–28 bold, one highlight
- L18 Before/After — 2col + arrow — Before (muted) → After (strong), clear contrast
- L19 List — stack — Title 40 + 3–5 items @28, large gaps, no wrapping
- L20 Closing — center stack — Headline 48–56 + Sub 24–28 + Contact 24

## Pick the layout by intent

Match the slide's job to one contract, then commit:

- Opening → L01/L02 · Section divider → L03 · Statement/Quote → L04/L12
- Concept + visual → L05/L06/L14 · Features → L07/L16 · Compare → L08/L18
- Single/Two/Three KPIs → L09/L10/L11 · Process steps → L13 · 4-quadrant matrix → L15
- Chart + takeaway → L17 · Plain list → L19 · Closing → L20

Reuse ONE contract across same-purpose slides in a deck; do not invent a fresh layout per slide.

## Deck context (modulate, rules above still apply)

- Corporate → structured, full contracts, balanced density.
- Startup/pitch → minimal, bold, oversized statement slides.
- Marketing → benefit-driven copy, strong visuals.
- Internal → slightly denser content allowed.
- Keynote → very visual, mostly statement + hero-image slides.

## Opening & closing

- First and last slides are STATEMENTS — emotional, not informational. Combine a strong visual with powerful words. Opening sets the tone; closing leaves the lasting impression. Aim for feeling, not facts.
- Text-only slides: let typography do the emotional work — oversized type, intentional asymmetry. Unusual ≠ unreadable.

Apply these silently through node structure; never emit the contract IDs as text.

## Delivering the deck

Built boards are not a delivered deck. Export it:

- MCP: `export_deck { format: "pptx" | "html" | "pdf", outputPath }`
- CLI: `op export-deck --output PATH [--format pptx|html|pdf]`

`pptx` is editable PowerPoint and the default; `html` is self-contained; `pdf`
is one page per slide. The path argument is `outputPath` — `filePath` means
the target `.op` document everywhere in this API.
