---
name: 'leadprint-vermilion-light'
tags: [social-card, card-series, cjk-type, vertical-portrait, light-mode, editorial, magazine, serif, display, warm-tones, red-accent, sharp-corners, confident]
platform: card
---

## Style Scope

This guide is self-contained and written for **Chinese social cards** — a swipeable series of fixed-canvas pages read on a phone, not a scrolling page and not an app screen. Apply its palette, rule system, spacing, and type treatments only when this exact guide is selected; do not pull dashboard, terminal, wellness, or SaaS patterns into it. Treat unnamed layout frames as structural by default: no fill, stroke, cornerRadius, or shadow unless the node is deliberately a masthead rule, a tinted column, a pull-quote block, or a table row. Structure comes from rules and column edges, not from cards with shadows.

## Style Summary

A newspaper set in lead, one colour over black. The page is `#F1EDE1` — mechanical woodpulp newsprint, greyer and cooler than a cream ivory and much cooler than rice paper. The ink is not black: `#1E1A16`, lightness 0.22 with a faint warm cast, because lead ink laid on absorbent pulp spreads a hair and never reaches true black. Those two decisions — 2% each — are the entire difference between "looks like a newspaper" and "black text on a white page".

There is exactly one chromatic colour: **vermilion**, the second-run red of an old masthead, orange-leaning and a little dirty. Newspapers of this period were black plus one, and the palette keeps that constraint literally. A second colour exists (`#324673`, an indigo stamp) and is capped at **one appearance per deck** — it is the ink the editor used to mark the proof, not part of the printing.

The signature device is **misregistration**. The cover headline is set twice: once in vermilion, offset by (3, 3), sitting underneath the black setting. That is what a two-run press does when the sheet shifts. It happens once, on the cover, and only there — used twice it stops being a press artefact and becomes a graphic effect.

Everything else is rules and columns. The masthead is a heavy rule over a hairline. Columns are separated by a 1px vertical. Tables use a tinted zebra row rather than a border. There is no shadow anywhere in the system, because ink on pulp does not cast one.

Key aesthetics:

- **Woodpulp ground**: `#F1EDE1` at chroma 0.016 — machine-made newsprint, not ivory and not rice paper
- **Ink that isn't black**: `#1E1A16`, warm-cast, the spread of lead ink into pulp
- **Black plus one**: vermilion is the only chromatic colour; a second is capped at one per deck
- **Misregistration once**: the cover headline doubled in vermilion at (3, 3), under the black
- **Rules, not cards**: heavy-over-hairline masthead, 1px column rules, tinted zebra rows
- **No shadow, no radius above 8px**: printed matter is flat and cut square
- **Old-Song display over Song body**: a hard-cut headline face over a publishing-grade serif
- **Vermilion as ink and as wash**: solid for headline runs, a tinted wash for blocks carrying black text
- **Grid visible in the outcome**: columns align hard; a hanging element is a defect

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                                            |
| --------------- | ------- | ---------------------------------------------------------------- |
| Page Background | #F1EDE1 | The newsprint sheet. The whole card; never a panel colour          |
| Card Surface    | #E5DDCC | Aged stock — pull-quote blocks, sidebars, tinted columns           |
| Tint Surface    | #D8CEBB | Table zebra rows and the deepest tint                              |
| Vermilion Wash  | #FAD5C8 | The red block that carries black text (12.69:1)                    |
| Inverted Band   | #1E1A16 | One full-bleed ink band per deck, at most                          |

### Text Colors

| Token          | Value   | Usage                                                             |
| -------------- | ------- | ----------------------------------------------------------------- |
| Primary Text   | #1E1A16 | Headlines and body copy. 14.77:1 on the sheet                     |
| Secondary Text | #544F4A | Deck lines, annotations, list sub-copy. 6.92:1                    |
| Muted Text     | #6D6863 | Sources, bylines, page numbers. 4.71:1                            |
| On Ink         | #F1EDE1 | All text on the inverted band and on solid vermilion              |

The grey ramp is warm at hue 60. A neutral grey caption on this stock reads blue by comparison and immediately looks like a screenshot of a website.

### Border Colors

| Token           | Value   | Usage                                                           |
| --------------- | ------- | --------------------------------------------------------------- |
| Default Border  | #B3B1AA | Column rules, hairlines, table dividers. **Never carries text**  |
| Lead Grey       | #717579 | Icon strokes and rule marks that need more weight than a hairline |

`#B3B1AA` at 2.28:1 against the sheet is a rule colour and nothing else. Any text set in it fails, so the ramp above exists to make that unnecessary — captions go to Muted Text.

### Accent Colors

| Token          | Value   | Usage                                                                    |
| -------------- | ------- | ------------------------------------------------------------------------ |
| Primary Accent | #B74A37 | Vermilion. Headline runs ≥40px, rules, numerals, the misregister layer    |
| Vermilion Wash | #FAD5C8 | The vermilion block that carries black text                              |
| Indigo Stamp   | #324673 | The proof-mark blue. **Exactly one appearance per deck**                  |

**How to use vermilion.** At 4.43:1 on the sheet it is legal at 40px and up, or at any size in weight 700. Below that it goes to black and the emphasis moves to a vermilion rule underneath the phrase. A solid vermilion block must carry **newsprint-coloured text** (4.43:1) — ink on vermilion is only 3.33:1 and fails. The house choice is the wash instead: black on `#FAD5C8` is 12.69:1, and a saturated red panel on newsprint reads as advertising.

Vermilion appears **at most twice per card**. The masthead rule counts as one of the two.

**Gradients**: none. Not on a block, not on a rule, not on the ground. Two-run letterpress has no gradients and neither does this style.

## Typography

### Font Families

| Role             | Family                                       | Range         | Usage                                                 |
| ---------------- | -------------------------------------------- | ------------- | ----------------------------------------------------- |
| Display          | Libre Caslon Text, Noto Serif SC             | 88px and up   | Mastheads, cover headlines, pull quotes               |
| Body / Interface | Archivo, Noto Serif SC                       | 64px and down | Body, deck lines, list rows, captions, page numbers   |
| Data / Numerals  | Archivo                                      | any           | Every figure, with tabular-nums on                    |

Write each family cell as one explicit fallback chain: `Libre Caslon Text, Noto Serif SC` for display and `Archivo, Noto Serif SC` for body. These families are supplied by OpenPencil. The Latin face leads so Latin words and figures use Archivo / Libre Caslon and Han characters use Noto Serif SC. If the user supplies 京华老宋体 as a custom font, it may lead the Chinese display treatment; do not require that unbundled face for the default design.

Body copy is a **serif**, not a sans. This is a newspaper: a sans body would be a magazine, which is a different object.

### Type Scale

| Level      | Size  | Font    | Weight | Tracking | Line Height | Usage                                   |
| ---------- | ----- | ------- | ------ | -------- | ----------- | --------------------------------------- |
| Display XL | 168px | Display | 700    | -0.01em  | 1.05        | Masthead-scale cover headline, 2-6 字   |
| Display L  | 120px | Display | 700    | -0.01em  | 1.10        | Cover headline, large numerals          |
| Display    | 88px  | Display | 700    | 0        | 1.15        | Interior headline, pull quote           |
| Title 1    | 64px  | Body    | 700    | 0        | 1.25        | Page title                              |
| Title 2    | 48px  | Body    | 600    | 0        | 1.30        | Section title, table header             |
| Body L     | 40px  | Body    | 400    | 0.02em   | 1.70        | **Default body copy**                   |
| Body       | 36px  | Body    | 400    | 0.02em   | 1.70        | Body floor — never below                |
| Caption    | 32px  | Body    | 400    | 0.02em   | 1.50        | Bylines, sources, page numbers          |

**At most four of these eight on one card.** Newspapers look busy but are ruthlessly few-tiered; the density comes from column structure, not from type sizes.

### Font Weights

| Weight   | Value | Usage                                                    |
| -------- | ----- | -------------------------------------------------------- |
| Regular  | 400   | All body copy and captions                                |
| Semibold | 600   | Section titles and table headers                          |
| Bold     | 700   | Page titles, display lines, and small-size vermilion runs  |

No italic on Han characters — the synthesised skew deforms the strokes. Emphasis is weight, a vermilion rule under the phrase, or a dot emphasis mark, which is the native Chinese device and the most period-correct one here.

### Line Height

- Display (88-168px): 1.05-1.15
- Titles (48-64px): 1.25-1.30
- Body (36-40px): **1.70** — Han glyphs fill the em box, so Chinese body needs about 0.2 more leading than the Latin default
- Captions: 1.50

### Letter Spacing

Body 0.02em; titles 0; display -0.01em. Latin all-caps labels take +0.10em, the only positive tracking in the system. Never apply Latin display tracking of -0.05em to Han text.

### Line Length

Body lines cap at **22 Han characters**. At 40px on a 920px column that is the natural break; at 36px the column must narrow to 10 of 12 columns to hold the cap. Runaway line length does more damage to readability than any type choice.

## Spacing System

### Gap Scale

| Value | Usage                                              |
| ----- | -------------------------------------------------- |
| 8px   | The baseline unit                                  |
| 16px  | Byline to rule, icon to label                      |
| 24px  | Between a list number and its text                 |
| 32px  | Between paragraphs (1.5 × the 40px body line)      |
| 48px  | Between a headline and its deck line               |
| 72px  | Between blocks within a card                       |
| 120px | Between the masthead block and the content block   |

### Padding Scale

| Value     | Usage                                                     |
| --------- | --------------------------------------------------------- |
| [12, 20]  | Vermilion tags and corner marks                           |
| 32px      | Table cells                                               |
| 40px      | Pull-quote and sidebar blocks                             |
| 56px      | The inverted ink band                                     |
| [96, 80]  | The card — 96 top, 80 sides                               |
| 128px     | Card bottom, larger than the top so feed chrome doesn't sit on the last line |

### Layout Pattern

- Canvas 1080 × 1440 (3:4). Content column 920: 12 columns of 62 with 16 gutters
- Masthead: a 3px rule over a 1px rule, full content width, with the headline between them
- Cover: masthead, then the doubled misregistered headline, then a deck line, then one rule
- Columns: two-column body is allowed only at Body 36 in an 11-column measure; otherwise single column
- Tables: header row in Card Surface, zebra rows in Tint Surface, no vertical rules
- Inverted band: at most one per deck, full-bleed, 0 radius
- Page number: bottom-right, Caption size, Muted Text, no frame

## Corner Radius

| Value | Usage                                              | Rationale                                 |
| ----- | -------------------------------------------------- | ----------------------------------------- |
| 0px   | Rules, bands, table cells, tinted columns           | Printed matter is cut, not rounded        |
| 4px   | Button radius, small vermilion tags                 | The barest softening on an interactive tag |
| 8px   | Card radius — pull-quote blocks and sidebars        | The maximum in this system                 |

Nothing exceeds 8px and nothing is a capsule. A rounded card in this style reads as a web component pasted onto a newspaper.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, 2px stroke, square caps

Icons are near-absent. A newspaper marks things with rules, numerals, and dingbats, and so does this style. Use an icon only in a list marker or a source line.

### Commonly Used Icons

arrow-right, arrow-up-right, chevron-right, minus, square, quote, hash, bookmark, external-link, corner-down-right

### Icon Sizes

| Size | Usage                          |
| ---- | ------------------------------ |
| 32px | Beside Caption text            |
| 40px | Beside Body text, list markers |
| 56px | Section markers                |

### Icon Color States

| State     | Color   | Usage                                  |
| --------- | ------- | -------------------------------------- |
| Primary   | #1E1A16 | Default on the sheet                   |
| Muted     | #6D6863 | Decorative and inactive marks          |
| Accent    | #B74A37 | The single marker carrying emphasis    |
| On Ink    | #F1EDE1 | On the inverted band                   |

## Anti-Patterns

- **No aging effects.** No coffee stains, no torn edges, no curling paper, no halftone dot screens. The stock's character is already in its colour; simulated damage is costume.
- **No brushwork or seals.** No ink wash, no dry-brush, no calligraphic stroke, no red seal stamps. This is a machine-set lead page: the letterforms are hard-edged and the red is a second press run, not a chop.
- **Misregistration exactly once**, on the cover only. A second use makes it a decoration rather than an artefact.
- **`#B3B1AA` never carries text.** It is a rule colour at 2.28:1.
- **No shadows.** Ink on pulp casts none, and the moment one appears the whole system reads as a web page.
- **Indigo Stamp more than once per deck** voids the "black plus one" premise the style is built on.
- **No gradient anywhere.**
