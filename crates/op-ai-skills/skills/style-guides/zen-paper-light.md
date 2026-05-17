---
name: 'zen-paper-light'
tags: [zen, minimal, light-mode, quiet, paper, serif, japanese, webapp]
platform: webapp
---

## Style Summary

A contemplative, ultra-minimal interface inspired by washi paper, ink brushwork, and the Japanese concept of "ma" (negative space as a compositional element). The warm parchment #F5F1EB background evokes handmade paper with its subtle warmth, while the stone accent (#78716C) provides muted, almost invisible guidance — like faded ink on an aged scroll. Noto Serif brings understated elegance to headings at light weights (400-500), its refined curves suggesting calligraphic discipline without ostentation. Noto Sans handles body text with quiet precision. A universal 4px radius produces barely-there corner softening — just enough to feel considered, never enough to distract.

Key aesthetics:

- **Paper-like warmth**: #F5F1EB background suggests handmade washi paper with golden undertones
- **Stone-muted accent**: #78716C is intentionally quiet — guidance through whisper, not shout
- **Light-weight serif headings**: Noto Serif at 400-500 weight creates understated, calligraphic elegance
- **Minimal radius**: 4px universal corners — barely perceptible, infinitely restrained
- **Generous "ma" space**: Abundant whitespace is treated as a design element, not empty filler
- **Monochromatic restraint**: Warm grays and stone tones with almost no chromatic color

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                          |
| --------------- | ------- | ---------------------------------------------- |
| Page Background | #F5F1EB | Root page background (warm parchment)          |
| Card Surface    | #FAF8F5 | Cards, elevated containers (lighter parchment) |
| Inset Surface   | #EEEAE3 | Input backgrounds, recessed areas              |
| Ink Surface     | #292524 | Inverted sections, footer backgrounds          |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #292524 | Headings, primary labels (ink black)     |
| Secondary Text | #57534E | Body text, descriptions                  |
| Tertiary Text  | #A8A29E | Captions, timestamps, helper text        |
| Muted Text     | #D6D3D1 | Placeholders, disabled text              |
| Accent Text    | #78716C | Active nav items, links, subtle emphasis |

### Border Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Default Border | #E7E5E4 | Card borders, dividers, input outlines |
| Subtle Border  | #EEEAE3 | Light separators, section dividers     |
| Active Border  | #78716C | Focused inputs, active tab indicators  |
| Ink Border     | #292524 | Strong dividers, section separators    |

### Accent Colors

| Token          | Value     | Usage                                      |
| -------------- | --------- | ------------------------------------------ |
| Primary Accent | #78716C   | Primary buttons, active states (stone 500) |
| Accent Hover   | #57534E   | Button hover, pressed states (stone 600)   |
| Accent Light   | #78716C12 | Tinted backgrounds, selected rows          |
| Accent Muted   | #78716C30 | Badge fills, progress tracks               |
| Ink Accent     | #292524   | Strong emphasis, inverted buttons          |
| Success        | #4D7C0F   | Positive metrics (muted olive green)       |
| Warning        | #A16207   | Alert indicators (muted amber)             |
| Error          | #991B1B   | Error states (muted red)                   |

## Typography

### Font Families

| Role              | Family     | Usage                                                        |
| ----------------- | ---------- | ------------------------------------------------------------ |
| Headings          | Noto Serif | Section headings, display text, card titles (weight 400-500) |
| Body / Functional | Noto Sans  | Body text, labels, buttons, navigation (weight 400)          |

### Type Scale

| Level      | Size | Font       | Weight | Usage                                        |
| ---------- | ---- | ---------- | ------ | -------------------------------------------- |
| Display    | 36px | Noto Serif | 400    | Page titles, hero headings (light, airy)     |
| Metric     | 28px | Noto Serif | 500    | Primary values, featured numbers             |
| Title 1    | 22px | Noto Serif | 500    | Section headings                             |
| Title 2    | 17px | Noto Serif | 500    | Card titles, subsection headings             |
| Title 3    | 14px | Noto Serif | 500    | List headings, compact card titles           |
| Body Large | 16px | Noto Sans  | 400    | Lead paragraphs, featured descriptions       |
| Body       | 14px | Noto Sans  | 400    | Standard body text, table cells              |
| Label      | 12px | Noto Sans  | 500    | Field labels, column headers, nav items      |
| Caption    | 11px | Noto Sans  | 400    | Timestamps, secondary metadata               |
| Overline   | 10px | Noto Sans  | 500    | Section overlines (uppercase, widely spaced) |
| Micro      | 10px | Noto Sans  | 500    | Badges, auxiliary labels                     |

### Font Weights

| Weight  | Value | Usage                                             |
| ------- | ----- | ------------------------------------------------- |
| Regular | 400   | Body text, display headings (intentionally light) |
| Medium  | 500   | Section headings, labels, buttons                 |

### Letter Spacing

- Display (36px): 0px (natural serif spacing)
- Section headings (17-22px): 0px
- Overline / uppercase labels: +3px (very wide, meditative)
- Body text: 0px
- Metric values: -0.5px

### Line Height

- Display (36px): 1.3 (generous, airy)
- Headings (17-22px): 1.3
- Body (14-16px): 1.7 (extra generous for contemplative reading)
- Captions (10-12px): 1.5

## Spacing System

### Gap Scale

| Value | Usage                                         |
| ----- | --------------------------------------------- |
| 4px   | Inline icon+text pairs                        |
| 8px   | Compact element groups                        |
| 12px  | Between list items, nav items                 |
| 16px  | Card internal sections                        |
| 24px  | Between cards in a grid, content sections     |
| 32px  | Section gaps, breathing room                  |
| 48px  | Major section breaks                          |
| 64px  | Top-level page sections (generous "ma" space) |

### Padding Scale

| Value    | Usage                                |
| -------- | ------------------------------------ |
| [8, 16]  | Compact buttons, badges              |
| [10, 20] | Standard buttons, input fields       |
| [12, 24] | Large buttons                        |
| [14, 18] | Sidebar nav items                    |
| 24px     | Card padding (compact)               |
| 32px     | Card padding (standard, generous)    |
| 40px     | Feature card padding                 |
| [40, 48] | Content area padding (ample margins) |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 220px wide, vertical, parchment (#F5F1EB) with stone-gray active indicators
- Content area: fill_container, vertical, gap 32-48, padding [40, 48]
- Cards use 1px stroke (#E7E5E4) with no shadow (paper-flat surfaces)
- Header bar: 56px height, parchment background, bottom border #E7E5E4
- Generous vertical spacing between all elements — whitespace is the primary design element
- Max content width: 960px centered (narrow for focused reading)

## Corner Radius

| Value | Usage                                                      | Rationale                      |
| ----- | ---------------------------------------------------------- | ------------------------------ |
| 2px   | Badges, inline tags, micro elements                        | Almost imperceptible softening |
| 4px   | All other elements: buttons, cards, inputs, modals, panels | Universal subtle radius        |

Design rationale: The near-universal 4px radius embodies the Zen principle of subtle, invisible craftsmanship. Corners are softened just enough that the eye registers care and intention, but never enough to create a decorative statement. Like the rounded corners of a Japanese wooden tray or a smooth stone in a garden — the softening is functional (removing harshness) rather than aesthetic (drawing attention). This restraint keeps focus on content and negative space rather than surface decoration.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (gentle, brush-like stroke quality)

### Commonly Used Icons

book-open, feather, leaf, sun, moon, cloud, droplets, wind, mountain, compass, map, search, bookmark, archive, folder, file-text, pen-line, heart, star, bell, settings, user, plus, x, check, chevron-right, chevron-down, eye, clock, calendar

### Icon Sizes

| Size | Usage                                          |
| ---- | ---------------------------------------------- |
| 14px | Inline indicators, table row actions           |
| 16px | Sidebar navigation icons, button leading icons |
| 20px | Card header action icons                       |
| 24px | Primary sidebar icons, page header actions     |

### Icon Color States

| State   | Color   | Usage                                     |
| ------- | ------- | ----------------------------------------- |
| Active  | #78716C | Active navigation, selected states        |
| Default | #A8A29E | Standard icons, secondary actions (muted) |
| Muted   | #D6D3D1 | Disabled icons, placeholder states        |
| Ink     | #292524 | Strong emphasis icons on light surfaces   |
| On Dark | #FAF8F5 | Icons on inverted/ink backgrounds         |
