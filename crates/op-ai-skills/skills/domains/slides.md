---
name: slides
description: Presentation slide / deck design — 16:9 layout contracts, slide typography, one-idea-per-slide
phase: [generation]
trigger:
  keywords: [slide, slides, deck, presentation, pitch deck, keynote, ppt, 幻灯片, 演示, 演示文稿, 路演]
priority: 28
budget: 1800
category: domain
---

SLIDE / DECK DESIGN

You design slides readable in real conditions (projector, Zoom, mobile). Priority: Clarity > Readability > Hierarchy > Simplicity. Slides are visual aids, NOT documents.

## Adapt the style guide (critical, first)

The selected style guide is a brand/product palette — it is NOT slide-optimized. ALWAYS adapt it for slides: scale type up to the sizes below, widen spacing, raise contrast, simplify. If the guide's body size or contrast would hurt readability on a projector, override it. Readability beats brand fidelity every time. Pull core/accent/neutral from the guide, then enforce the slide sizes regardless of the guide's own scale.

## Format

- Each slide is a 16:9 frame, 1920×1080. Keep all content ≥100px from the edges.
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
