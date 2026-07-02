---
name: 'scandinavian-minimal-light'
tags: [scandinavian, minimal, organic, sage-green, warm-tones, soft-shadows, rounded, cozy]
platform: webapp
---

## Style Summary

A warm, inviting interface inspired by Scandinavian design principles of organic simplicity, natural materials, and cozy functionality. The parchment-toned background (#F7F6F3) replaces cold whites with a warm paper feel, while sage green (#7C9070) accents evoke natural foliage. Fraunces, a variable serif with optical sizing, brings warmth and personality to display text, complemented by Plus Jakarta Sans for clean functional text and Space Mono for data precision. Soft organic radii (10-16px) and subtle shadows complete the hygge aesthetic.

Key aesthetics:

- **Parchment warmth**: #F7F6F3 background gives every screen a natural, paper-like quality
- **Sage green accent**: #7C9070 as the single natural accent, used for active states and highlights
- **Organic radii**: 10-16px corners create soft, rounded shapes without being overly playful
- **Display serif**: Fraunces variable font with optical sizing for warm, characterful headings
- **Horizontal top nav**: Clean navigation bar instead of sidebar, preserving horizontal content flow
- **Soft shadows**: Subtle warm-toned shadows instead of hard borders for depth

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                 |
| --------------- | ------- | ------------------------------------- |
| Page Background | #F7F6F3 | Root page background (parchment)      |
| Card Surface    | #FFFFFF | Cards, elevated containers            |
| Inset Surface   | #F0EFEC | Input backgrounds, recessed areas     |
| Warm Surface    | #EDECE8 | Secondary surfaces, sidebar (if used) |

### Text Colors

| Token          | Value   | Usage                                       |
| -------------- | ------- | ------------------------------------------- |
| Primary Text   | #1A1A18 | Headings, primary content (warm black)      |
| Secondary Text | #5C5C58 | Body text, descriptions                     |
| Tertiary Text  | #8A8A85 | Captions, timestamps                        |
| Muted Text     | #B5B5B0 | Placeholders, disabled text                 |
| Accent Text    | #7C9070 | Active nav items, links, highlighted values |

### Border Colors

| Token          | Value   | Usage                              |
| -------------- | ------- | ---------------------------------- |
| Default Border | #E5E4E0 | Card borders, dividers (warm tint) |
| Subtle Border  | #EDECE8 | Light separators                   |
| Active Border  | #7C9070 | Focused inputs, active indicators  |

### Accent Colors

| Token          | Value     | Usage                                      |
| -------------- | --------- | ------------------------------------------ |
| Primary Accent | #7C9070   | Active states, primary buttons, sage green |
| Accent Light   | #7C907020 | Tinted backgrounds, hover states           |
| Accent Muted   | #7C907050 | Badge backgrounds, subtle indicators       |
| Success        | #4A8C5C   | Positive metrics (deeper green)            |
| Warning        | #C4943A   | Alert states (warm amber)                  |
| Error          | #B85C5C   | Error states (muted warm red)              |

## Typography

### Font Families

| Role              | Family            | Usage                                               |
| ----------------- | ----------------- | --------------------------------------------------- |
| Display / Serif   | Fraunces          | Hero headings, section titles, display metrics      |
| Body / Functional | Plus Jakarta Sans | Body text, labels, buttons, navigation              |
| Data / Mono       | Space Mono        | Metric values, timestamps, chart data, table values |

### Type Scale

| Level   | Size | Font              | Weight | Usage                              |
| ------- | ---- | ----------------- | ------ | ---------------------------------- |
| Display | 48px | Fraunces          | 600    | Hero section heading               |
| Metric  | 36px | Space Mono        | 600    | Primary metric values              |
| Title 1 | 28px | Fraunces          | 600    | Section headings                   |
| Title 2 | 20px | Fraunces          | 500    | Card titles, subsection headings   |
| Title 3 | 16px | Plus Jakarta Sans | 600    | List headings, compact card titles |
| Body    | 14px | Plus Jakarta Sans | 400    | Descriptions, body text            |
| Label   | 12px | Plus Jakarta Sans | 500    | Field labels, column headers       |
| Data    | 13px | Space Mono        | 400    | Table values, timestamps           |
| Caption | 11px | Plus Jakarta Sans | 400    | Captions, metadata                 |
| Micro   | 10px | Plus Jakarta Sans | 500    | Badges, auxiliary labels           |

### Font Weights

| Weight   | Value | Usage                                     |
| -------- | ----- | ----------------------------------------- |
| Regular  | 400   | Body text, data values, descriptions      |
| Medium   | 500   | Labels, navigation items, serif subtitles |
| Semibold | 600   | Section titles, metric values, headings   |

### Letter Spacing

- Display (48px): -1.5px
- Section headings (20-28px): -0.5px
- Uppercase labels: +1px
- Body text: 0px
- Monospace data: 0px

### Line Height

- Display (48px): 1.0
- Headings (20-28px): 1.2
- Body (14px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                        |
| ----- | -------------------------------------------- |
| 4px   | Inline icon+text pairs                       |
| 6px   | Status indicators to labels                  |
| 8px   | Compact element groups                       |
| 12px  | Between list items, nav items, button groups |
| 16px  | Card internal sections, form fields          |
| 20px  | Between cards in a grid                      |
| 24px  | Content section gaps                         |
| 32px  | Major section breaks                         |
| 48px  | Top-level page sections                      |

### Padding Scale

| Value    | Usage                             |
| -------- | --------------------------------- |
| [8, 16]  | Input fields, compact buttons     |
| [10, 20] | Standard buttons                  |
| [12, 24] | Navigation bar horizontal padding |
| 24px     | Card padding (standard)           |
| 28px     | Metric card padding               |
| 32px     | Content area padding              |
| [32, 48] | Content area generous padding     |
| [16, 32] | Navigation bar padding            |

### Layout Pattern

- Screen width: 1440px
- Navigation: horizontal top bar, height 64px, full width
- Content area: max-width 1200px centered, vertical, gap 32-48, padding [32, 48]
- Cards use soft shadow (0 2px 8px rgba(0,0,0,0.06)) instead of hard borders
- Optional light border (#E5E4E0) on cards in addition to shadow

## Corner Radius

| Value | Usage                                          | Rationale                |
| ----- | ---------------------------------------------- | ------------------------ |
| 10px  | Input fields, small buttons, badges            | Gentle organic softness  |
| 12px  | Standard cards, list containers, dropdowns     | Primary container radius |
| 14px  | Large cards, feature sections                  | Comfortable rounding     |
| 16px  | Hero cards, modal sheets, prominent containers | Maximum organic radius   |

Design rationale: Organic radii in the 10-16px range create a warm, approachable feel that mirrors natural forms. Unlike the pillow-soft 20px+ of playful styles, this range is restrained and mature, evoking the smooth curves of Scandinavian wooden furniture and ceramic vessels. The consistent range keeps the design cohesive without becoming childish.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (the organic stroke style complements the rounded corners and warm palette)

### Commonly Used Icons

home, bar-chart-2, users, settings, folder, file-text, trending-up, trending-down, search, bell, chevron-down, chevron-right, plus, x, edit, leaf, sun, coffee, heart, calendar, clock, check, grid, bookmark, map-pin

### Icon Sizes

| Size | Usage                                    |
| ---- | ---------------------------------------- |
| 14px | Inline metric indicators, table actions  |
| 16px | Navigation bar icons, list leading icons |
| 20px | Card header action icons                 |
| 24px | Primary navigation, header actions       |

### Icon Color States

| State     | Color   | Usage                              |
| --------- | ------- | ---------------------------------- |
| Active    | #7C9070 | Selected nav item, active states   |
| Default   | #5C5C58 | Standard icons, secondary actions  |
| Muted     | #B5B5B0 | Disabled icons, placeholder states |
| On Accent | #FFFFFF | Icons on sage-green backgrounds    |
