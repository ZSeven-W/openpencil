---
name: 'agency-editorial-light'
tags: [editorial, serif, light-mode, bold-typography, magazine, creative, landing-page]
platform: webapp
---

## Style Summary

A bold, magazine-inspired creative agency landing page with a near-white (#FAFAFA) background and near-black (#1A1A1A) as the primary accent. This style borrows from editorial design and print magazines — oversized serif headlines, dramatic whitespace, and stark contrast create a layout that feels curated and intentional. DM Serif Display provides commanding headlines with high-contrast thick/thin strokes, while DM Sans keeps functional text clean and geometric.

Key aesthetics:

- **Near-white canvas**: #FAFAFA background with pure white (#FFFFFF) cards for minimal contrast layering
- **Near-black accent**: #1A1A1A as the primary color for CTAs, headers, and key elements — monochrome authority
- **Editorial serif**: DM Serif Display at 400 weight with dramatic thick/thin contrast for headline impact
- **Minimal radii**: 0-4px corners maintain the editorial, print-inspired sharpness
- **Oversized typography**: Hero headlines at 80-96px dominate the viewport, demanding attention
- **Asymmetric layouts**: Text-heavy sections with large type balanced against generous negative space

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                 |
| --------------- | ------- | ------------------------------------- |
| Page Background | #FAFAFA | Root page background (warm off-white) |
| Section Alt     | #F5F5F5 | Alternating section background        |
| Card Surface    | #FFFFFF | Cards, featured project panels        |
| Dark Section    | #1A1A1A | Inverted dark sections (CTA, footer)  |

### Text Colors

| Token          | Value   | Usage                       |
| -------------- | ------- | --------------------------- |
| Primary Text   | #1A1A1A | Headlines, primary content  |
| Secondary Text | #525252 | Body text, descriptions     |
| Tertiary Text  | #A3A3A3 | Captions, metadata, dates   |
| Muted Text     | #D4D4D4 | Placeholders, disabled text |
| Inverted Text  | #FAFAFA | Text on dark sections       |

### Border Colors

| Token          | Value   | Usage                                   |
| -------------- | ------- | --------------------------------------- |
| Default Border | #E5E5E5 | Card borders, dividers                  |
| Strong Border  | #1A1A1A | Bold decorative rules, section dividers |
| Subtle Border  | #F0F0F0 | Light separators                        |

### Accent Colors

| Token           | Value     | Usage                               |
| --------------- | --------- | ----------------------------------- |
| Primary Accent  | #1A1A1A   | CTA buttons, links, primary actions |
| Accent Inverted | #FAFAFA   | Buttons on dark sections            |
| Highlight       | #1A1A1A10 | Subtle hover backgrounds            |
| Error           | #B91C1C   | Error states                        |
| Success         | #15803D   | Success states                      |

## Typography

### Font Families

| Role              | Family           | Usage                                         |
| ----------------- | ---------------- | --------------------------------------------- |
| Display / Serif   | DM Serif Display | Hero headlines, section headings, pull quotes |
| Body / Functional | DM Sans          | Body text, navigation, buttons, labels        |

### Type Scale

| Level      | Size | Font             | Weight | Usage                                  |
| ---------- | ---- | ---------------- | ------ | -------------------------------------- |
| Hero       | 96px | DM Serif Display | 400    | Primary hero headline (massive)        |
| Display    | 72px | DM Serif Display | 400    | Large section headings                 |
| Title 1    | 48px | DM Serif Display | 400    | Section headings                       |
| Title 2    | 32px | DM Serif Display | 400    | Subsection headings, project titles    |
| Title 3    | 20px | DM Sans          | 500    | Card titles, list headings             |
| Body Large | 18px | DM Sans          | 400    | Lead paragraphs, hero subtitle         |
| Body       | 16px | DM Sans          | 400    | Descriptions, copy                     |
| Label      | 13px | DM Sans          | 500    | Navigation (uppercase), button text    |
| Caption    | 12px | DM Sans          | 400    | Dates, metadata, project categories    |
| Overline   | 11px | DM Sans          | 500    | Section overlines (uppercase, tracked) |

### Font Weights

| Weight  | Value | Usage                             |
| ------- | ----- | --------------------------------- |
| Regular | 400   | Serif headlines, body text        |
| Medium  | 500   | Sans headings, labels, navigation |

### Letter Spacing

- Hero (96px): -4px (extreme compression for impact)
- Display (72px): -3px
- Section headings (48px): -2px
- Subsection (32px): -1px
- Uppercase navigation: +3px (wide editorial tracking)
- Uppercase overlines: +4px
- Body text: 0px

### Line Height

- Hero (96px): 0.9 (tight stacking for multi-line impact)
- Display (72px): 0.95
- Headings (32-48px): 1.1
- Body (16-18px): 1.65 (generous editorial leading)
- Overlines (11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                          |
| ----- | ------------------------------ |
| 4px   | Inline icon+text pairs         |
| 8px   | Tight element groups           |
| 12px  | Navigation items               |
| 16px  | Card content sections          |
| 24px  | Between cards, feature entries |
| 32px  | Section internal content       |
| 48px  | Major content blocks           |
| 80px  | Between page sections          |
| 120px | Hero to first content section  |

### Padding Scale

| Value    | Usage                                    |
| -------- | ---------------------------------------- |
| [12, 28] | Navigation links (generous horizontal)   |
| [16, 36] | CTA buttons (wide, editorial proportion) |
| 32px     | Card padding (standard)                  |
| 40px     | Feature card padding                     |
| 48px     | Large section padding                    |
| [100, 0] | Section vertical padding (generous)      |
| [140, 0] | Hero vertical padding (dramatic)         |

### Layout Pattern

- Page width: 1440px max
- Content container: 1200px centered, padding [0, 48]
- Hero section: left-aligned or centered text, full viewport height, oversized type
- Project grid: 2-column masonry or full-width stacked panels
- About section: asymmetric 60/40 split (text/image)
- Services: numbered list with large serif numbers + description
- Navigation: fixed top, transparent, height 72px, uppercase tracking
- Footer: dark (#1A1A1A) full-width section, minimal centered content
- Decorative rules: 2px #1A1A1A horizontal lines between sections

## Corner Radius

| Value | Usage                                   | Rationale                    |
| ----- | --------------------------------------- | ---------------------------- |
| 0px   | Buttons, hero sections, dark sections   | Sharp editorial precision    |
| 2px   | Input fields, form elements             | Barely perceptible softening |
| 4px   | Cards, image containers, dropdown menus | Maximum allowed radius       |

Design rationale: Near-zero radii (0-4px) are essential to the editorial and print-magazine aesthetic. Sharp corners evoke newspaper columns, gallery walls, and magazine layouts where precision equals intentionality. The 4px maximum on cards prevents a harsh brutalist feel while maintaining the geometric rigor of editorial design.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

arrow-right, arrow-up-right, chevron-right, chevron-down, menu, x, external-link, play, pause, eye, image, layout, grid, pen-tool, camera, film, mail, phone, map-pin, instagram, twitter, dribbble

### Icon Sizes

| Size | Usage                                     |
| ---- | ----------------------------------------- |
| 16px | Inline navigation arrows, utility actions |
| 20px | Card actions, external link indicators    |
| 24px | Header icons, mobile menu toggle          |
| 32px | CTA arrow icons, section decorative       |

### Icon Color States

| State     | Color   | Usage                               |
| --------- | ------- | ----------------------------------- |
| Default   | #1A1A1A | Standard icons on light backgrounds |
| Secondary | #525252 | Secondary actions, metadata icons   |
| Muted     | #A3A3A3 | Disabled, placeholder               |
| Inverted  | #FAFAFA | Icons on dark backgrounds           |
