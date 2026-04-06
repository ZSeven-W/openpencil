---
name: 'monochrome-expressive-light'
tags: [monochrome, editorial, serif, high-contrast, elegant, magazine, uppercase, sharp-corners]
platform: webapp
---

## Style Summary

A bold editorial interface that achieves maximum visual impact using only black, white, and shades of gray. No accent color exists; hierarchy is built entirely through type size, weight, and spatial contrast. DM Serif Display headlines at 92px create dramatic focal points, while Inter body text provides clean readability. Horizontal top navigation with dot separators replaces the common sidebar pattern, giving the layout a magazine masthead quality.

Key aesthetics:

- **Pure monochrome**: No accent color. All hierarchy through black (#000000), white (#FFFFFF), and gray scale
- **Dramatic type scale**: DM Serif Display at 92px for titles, creating extreme size contrast with 14px body
- **Uppercase elements**: Section labels, navigation items, and metadata in uppercase with generous tracking
- **Dot-separated nav**: Horizontal top navigation with centered dot (·) separators between items
- **Zero radius**: All corners sharp for a print-inspired, editorial grid
- **High contrast**: True black on true white, no warm tinting or off-whites

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                  |
| --------------- | ------- | -------------------------------------- |
| Page Background | #FFFFFF | Root page background (pure white)      |
| Card Surface    | #FFFFFF | Cards with border stroke               |
| Inset Surface   | #FAFAFA | Subtle recessed areas, code blocks     |
| Dark Surface    | #000000 | Inverted hero sections, feature blocks |

### Text Colors

| Token          | Value   | Usage                          |
| -------------- | ------- | ------------------------------ |
| Primary Text   | #000000 | Headlines, primary content     |
| Secondary Text | #4A4A4A | Body text, descriptions        |
| Tertiary Text  | #7A7A7A | Captions, metadata, timestamps |
| Muted Text     | #B0B0B0 | Placeholders, disabled text    |
| Inverted Text  | #FFFFFF | Text on dark/black surfaces    |

### Border Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Default Border | #E0E0E0 | Card borders, section dividers         |
| Heavy Border   | #000000 | Navigation underlines, strong dividers |
| Subtle Border  | #F0F0F0 | Light separators between content rows  |

### Accent Colors

| Token            | Value   | Usage                                                       |
| ---------------- | ------- | ----------------------------------------------------------- |
| Primary (Black)  | #000000 | Active states, primary buttons, selected nav                |
| Secondary (Gray) | #4A4A4A | Secondary actions, hover states                             |
| Success          | #22C55E | Positive metrics (the only chromatic color, used sparingly) |
| Error            | #DC2626 | Error states (used only for functional necessity)           |

## Typography

### Font Families

| Role              | Family           | Usage                                               |
| ----------------- | ---------------- | --------------------------------------------------- |
| Display / Serif   | DM Serif Display | Hero headlines, section titles, dramatic focal text |
| Body / Functional | Inter            | Body text, labels, buttons, navigation, all UI text |

### Type Scale

| Level   | Size | Font             | Weight | Usage                         |
| ------- | ---- | ---------------- | ------ | ----------------------------- |
| Hero    | 92px | DM Serif Display | 400    | Page title, hero statement    |
| Display | 48px | DM Serif Display | 400    | Section hero headings         |
| Title 1 | 32px | DM Serif Display | 400    | Major section headings        |
| Title 2 | 24px | DM Serif Display | 400    | Subsection headings           |
| Title 3 | 18px | Inter            | 600    | Card titles, list headings    |
| Body    | 14px | Inter            | 400    | Descriptions, paragraphs      |
| Label   | 12px | Inter            | 500    | Uppercase labels, field names |
| Nav     | 13px | Inter            | 500    | Navigation items (uppercase)  |
| Caption | 11px | Inter            | 400    | Timestamps, metadata          |
| Micro   | 10px | Inter            | 500    | Badges, auxiliary data        |

### Font Weights

| Weight   | Value | Usage                                                        |
| -------- | ----- | ------------------------------------------------------------ |
| Regular  | 400   | Serif display (DM Serif Display is single-weight), body text |
| Medium   | 500   | Labels, navigation items, metadata                           |
| Semibold | 600   | Card titles, strong functional headings                      |

### Letter Spacing

- Hero/Display (48-92px): -3px (dramatic compression)
- Section headings (24-32px): -1px
- Uppercase labels: +2px (generous tracking)
- Uppercase navigation: +1.5px
- Body text: 0px

### Line Height

- Hero (92px): 0.9 (tight, overlapping baseline)
- Display (48px): 1.0
- Headings (24-32px): 1.1
- Body (14px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                  |
| ----- | -------------------------------------- |
| 4px   | Dot separator to nav text              |
| 8px   | Inline icon+text pairs, compact groups |
| 12px  | Between nav items (before/after dots)  |
| 16px  | Between table rows, form fields        |
| 20px  | Card internal sections                 |
| 24px  | Between cards, grid items              |
| 32px  | Content section gaps                   |
| 48px  | Major section separators               |
| 64px  | Top-level page section breaks          |

### Padding Scale

| Value    | Usage                         |
| -------- | ----------------------------- |
| [10, 16] | Standard buttons              |
| [8, 16]  | Input fields                  |
| [12, 24] | Navigation bar padding        |
| 24px     | Card padding                  |
| 32px     | Section padding               |
| [32, 64] | Content area generous padding |
| [48, 80] | Hero section padding          |

### Layout Pattern

- Screen width: 1440px
- Navigation: horizontal top bar, full width, height 64px, centered items
- Nav items: separated by · (middle dot), uppercase, 13px Inter 500
- Content area: full width, vertical, gap 48-64, padding [32, 64]
- No sidebar; content uses the full horizontal width

## Corner Radius

| Value | Usage      | Rationale                 |
| ----- | ---------- | ------------------------- |
| 0px   | Everything | Editorial print precision |

Design rationale: The zero-radius philosophy is essential to the magazine/editorial aesthetic. Every element is a sharp rectangle, echoing the precise trim of printed pages. Combined with the dramatic serif headlines, uppercase labels, and strict black-on-white palette, the result feels like a high-end publication laid out in a browser.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

arrow-right, arrow-up-right, chevron-down, chevron-right, plus, x, search, menu, filter, grid, list, eye, bookmark, share-2, external-link, download, mail, link, copy, type

### Icon Sizes

| Size | Usage                                     |
| ---- | ----------------------------------------- |
| 14px | Inline text indicators, navigation arrows |
| 16px | List item leading icons, button icons     |
| 20px | Section header actions, card actions      |
| 24px | Navigation menu icon, header actions      |

### Icon Color States

| State    | Color   | Usage                                |
| -------- | ------- | ------------------------------------ |
| Active   | #000000 | Selected nav item, active actions    |
| Default  | #4A4A4A | Standard UI icons, secondary actions |
| Muted    | #B0B0B0 | Disabled states, placeholder icons   |
| Inverted | #FFFFFF | Icons on black/dark surfaces         |
