---
name: 'portfolio-minimal-light'
tags: [minimal, light-mode, quiet, monochrome, elegant, landing-page, whitespace]
platform: webapp
---

## Style Summary

A quiet, restrained portfolio page built on a near-white (#FAFAFA) canvas with near-black (#171717) as the singular accent. This style channels the discipline of Swiss design and the serenity of gallery spaces — every element earns its place through necessity, not decoration. Sora headlines at 500-600 weight bring geometric precision with a subtle warmth, while Inter body text provides invisible, efficient readability. The monochrome palette and generous whitespace create a stage where the work itself is the hero.

Key aesthetics:

- **Near-white canvas**: #FAFAFA base with #FFFFFF for card surfaces, creating subtle depth without color
- **Monochrome restraint**: #171717 near-black for all emphasis — no color accent, letting portfolio work provide color
- **Geometric display type**: Sora at 500-600 weight, clean and precise without being cold
- **Minimal radii**: 4-8px for subtle softening, never decorative or playful
- **Generous whitespace**: Oversized vertical margins between sections create breathing room and focus
- **Grid-first layout**: Strong grid structure with consistent column widths and gutters

## Color System

### Core Backgrounds

| Token           | Value   | Usage                             |
| --------------- | ------- | --------------------------------- |
| Page Background | #FAFAFA | Root page background (warm white) |
| Section Alt     | #F5F5F5 | Alternating section background    |
| Card Surface    | #FFFFFF | Project cards, image containers   |
| Inset Surface   | #F0F0F0 | Code blocks, secondary panels     |

### Text Colors

| Token          | Value   | Usage                                      |
| -------------- | ------- | ------------------------------------------ |
| Primary Text   | #171717 | Headlines, project titles, primary content |
| Secondary Text | #525252 | Body text, project descriptions            |
| Tertiary Text  | #A3A3A3 | Captions, dates, metadata                  |
| Muted Text     | #D4D4D4 | Placeholders, disabled text                |
| Link Text      | #171717 | Links (underlined, no color distinction)   |

### Border Colors

| Token          | Value   | Usage                        |
| -------------- | ------- | ---------------------------- |
| Default Border | #E5E5E5 | Card borders, dividers       |
| Subtle Border  | #F0F0F0 | Light separators             |
| Active Border  | #171717 | Focused inputs, hover states |

### Accent Colors

| Token          | Value     | Usage                                   |
| -------------- | --------- | --------------------------------------- |
| Primary Accent | #171717   | CTA buttons, active states, emphasis    |
| Accent Light   | #F5F5F5   | Hover backgrounds, selected states      |
| Accent Muted   | #17171710 | Subtle hover tints                      |
| On Accent      | #FFFFFF   | Text on black backgrounds               |
| Error          | #DC2626   | Error states (only color in the system) |

## Typography

### Font Families

| Role              | Family | Usage                                         |
| ----------------- | ------ | --------------------------------------------- |
| Headings          | Sora   | Hero headlines, section titles, project names |
| Body / Functional | Inter  | Body text, navigation, buttons, descriptions  |

### Type Scale

| Level      | Size | Font  | Weight | Usage                                 |
| ---------- | ---- | ----- | ------ | ------------------------------------- |
| Hero       | 64px | Sora  | 500    | Primary hero headline                 |
| Display    | 48px | Sora  | 500    | Large section headings                |
| Title 1    | 36px | Sora  | 500    | Section headings                      |
| Title 2    | 24px | Sora  | 600    | Project titles, subsection headings   |
| Title 3    | 18px | Sora  | 600    | Card titles, list headings            |
| Body Large | 18px | Inter | 400    | Hero subtitle, lead paragraphs        |
| Body       | 16px | Inter | 400    | Descriptions, project copy            |
| Label      | 14px | Inter | 500    | Navigation, button text               |
| Caption    | 12px | Inter | 400    | Date labels, image captions, metadata |
| Micro      | 11px | Inter | 400    | Fine print, footer secondary          |

### Font Weights

| Weight   | Value | Usage                                    |
| -------- | ----- | ---------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions        |
| Medium   | 500   | Hero headlines, section headings, labels |
| Semibold | 600   | Project titles, subsection emphasis      |

### Letter Spacing

- Hero (64px): -2.5px
- Display (48px): -1.5px
- Section headings (36px): -1px
- Project titles (24px): -0.5px
- Uppercase labels: +3px (wide tracking for quiet emphasis)
- Body text: 0px

### Line Height

- Hero (64px): 1.0
- Display (48px): 1.05
- Headings (24-36px): 1.2
- Body (16-18px): 1.7 (generous for readability)
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                   |
| ----- | --------------------------------------- |
| 4px   | Inline icon+text pairs                  |
| 8px   | Tag groups, tight clusters              |
| 12px  | Navigation items                        |
| 16px  | Card content sections                   |
| 24px  | Between project cards in grid           |
| 32px  | Section internal content gaps           |
| 48px  | Between content blocks within a section |
| 80px  | Between page sections                   |
| 120px | Hero to first content section           |

### Padding Scale

| Value    | Usage                             |
| -------- | --------------------------------- |
| [8, 16]  | Compact buttons, tag pills        |
| [12, 24] | Standard buttons                  |
| [14, 32] | CTA buttons (generous horizontal) |
| 24px     | Card padding (compact)            |
| 32px     | Card padding (standard)           |
| [80, 0]  | Section vertical padding          |
| [120, 0] | Hero section vertical padding     |

### Layout Pattern

- Page width: 1200px max
- Content container: 1040px centered, padding [0, 40]
- Hero section: left-aligned text, max-width 700px, minimal elements
- Project grid: 2-column, gap 24px, large landscape images
- Project detail: full-width image + below description
- About section: 2-column — text left, portrait right
- Navigation: fixed top, minimal, height 64px, no shadow
- Footer: single line, centered, minimal links

## Corner Radius

| Value | Usage                       | Rationale                            |
| ----- | --------------------------- | ------------------------------------ |
| 0px   | Images, project thumbnails  | Sharp edges for gallery presentation |
| 4px   | Buttons, input fields, tags | Barely perceptible softening         |
| 8px   | Cards, containers           | Subtle rounding without personality  |

Design rationale: Minimal radii (4-8px) keep the design quiet and architectural. The near-zero approach lets the portfolio work speak without the interface competing for attention. Images use 0px to maintain photographic integrity, while buttons at 4px have just enough softness to feel interactive without being playful.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

arrow-right, arrow-up-right, external-link, chevron-right, chevron-down, menu, x, mail, github, twitter, linkedin, dribbble, figma, instagram, download, copy, eye, grid, list, calendar

### Icon Sizes

| Size | Usage                               |
| ---- | ----------------------------------- |
| 16px | Inline with body text, social links |
| 20px | Navigation items, button arrows     |
| 24px | Section icons, card actions         |

### Icon Color States

| State   | Color   | Usage                             |
| ------- | ------- | --------------------------------- |
| Active  | #171717 | Primary actions, hovered links    |
| Default | #525252 | Standard icons, secondary actions |
| Muted   | #A3A3A3 | Disabled, decorative icons        |
| On Dark | #FFFFFF | Icons on black backgrounds        |
