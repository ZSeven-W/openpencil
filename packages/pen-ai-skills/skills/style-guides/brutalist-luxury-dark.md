---
name: 'brutalist-luxury-dark'
tags:
  [brutalist, luxury, dark-mode, gold-accent, monospace, high-contrast, editorial, sharp-corners]
platform: webapp
---

## Style Summary

A high-contrast dark interface that merges brutalist architectural principles with luxury editorial design. The complete absence of border radius creates an uncompromising geometric grid, while gold (#C9A962) accents against pitch-black surfaces evoke opulence. Typography mixes a decorative serif logo mark with geometric sans-serif headings and monospace data, creating a layered visual hierarchy that feels both commanding and refined.

Key aesthetics:

- **Zero radius everywhere**: All corners are sharp at 0px, creating an architectural grid of perfect rectangles
- **Gold on black**: Single accent color (#C9A962) used for all highlights, active states, and decorative elements
- **Monospace data**: IBM Plex Mono for all numerical and tabular data, reinforcing precision
- **Icon-only sidebar**: Narrow 80px sidebar with gold icon indicators and no text labels
- **Thick structural borders**: Gold-tinted strokes define regions rather than shadows
- **Editorial typography**: Playfair Display for the logo mark, Space Grotesk for all functional headings

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                |
| --------------- | ------- | ------------------------------------ |
| Page Background | #0A0A0A | Root page background                 |
| Card Surface    | #0D0D0D | Cards, table rows, elevated surfaces |
| Sidebar Surface | #0A0A0A | Navigation sidebar background        |
| Inset Surface   | #141414 | Nested containers, input fields      |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #FFFFFF | Headings, metric values, active labels    |
| Secondary Text | #6A6A6A | Body text, descriptions, secondary labels |
| Tertiary Text  | #4A4A4A | Timestamps, captions, inactive elements   |
| Muted Text     | #3A3A3A | Placeholders, disabled states             |
| Accent Text    | #C9A962 | Active nav items, status labels, links    |

### Border Colors

| Token               | Value     | Usage                                    |
| ------------------- | --------- | ---------------------------------------- |
| Default Border      | #2A2A2A   | Card borders, dividers, table rules      |
| Subtle Border       | #1A1A1A   | Section separators, sidebar dividers     |
| Accent Border       | #C9A962   | Active sidebar indicator, focused inputs |
| Accent Muted Border | #C9A96240 | Decorative accent strokes at 25% opacity |

### Accent Colors

| Token          | Value     | Usage                                                               |
| -------------- | --------- | ------------------------------------------------------------------- |
| Primary Accent | #C9A962   | Active states, highlights, sidebar indicators                       |
| Accent Glow    | #C9A96266 | Status dot glow (40% opacity)                                       |
| Accent Subtle  | #C9A96230 | Accent-tinted backgrounds, hover states                             |
| Error          | #8B5A5A   | Destructive actions, error states (muted red to match gold palette) |
| Error Border   | #8B5A5A40 | Error stroke at 25% opacity                                         |

## Typography

### Font Families

| Role           | Family           | Usage                                                |
| -------------- | ---------------- | ---------------------------------------------------- |
| Logo / Display | Playfair Display | Brand mark, decorative titles                        |
| Headings / UI  | Space Grotesk    | Section headings, button labels, navigation tooltips |
| Data / Mono    | IBM Plex Mono    | Metric values, table data, timestamps, chart labels  |

### Type Scale

| Level   | Size | Font             | Weight | Usage                            |
| ------- | ---- | ---------------- | ------ | -------------------------------- |
| Logo    | 48px | Playfair Display | 700    | Brand mark in sidebar or header  |
| Display | 40px | Space Grotesk    | 600    | Hero metric values               |
| Title 1 | 24px | Space Grotesk    | 600    | Section headings                 |
| Title 2 | 18px | Space Grotesk    | 600    | Card titles, subsection headings |
| Body    | 14px | IBM Plex Mono    | 400    | Descriptions, table cell content |
| Label   | 12px | IBM Plex Mono    | 500    | Field labels, column headers     |
| Caption | 11px | IBM Plex Mono    | 400    | Timestamps, metadata             |
| Micro   | 10px | IBM Plex Mono    | 500    | Status badges, auxiliary data    |
| Tiny    | 9px  | IBM Plex Mono    | 400    | Footnotes, tertiary metadata     |

### Font Weights

| Weight   | Value | Usage                                   |
| -------- | ----- | --------------------------------------- |
| Regular  | 400   | Body text, table data, descriptions     |
| Medium   | 500   | Labels, secondary headings              |
| Semibold | 600   | Section titles, metric values, headings |
| Bold     | 700   | Logo mark, primary emphasis             |

### Letter Spacing

- Logo and display: -2px (tight, dramatic)
- Uppercase labels: +1.5px
- Monospace data: 0px (natural mono spacing)
- Headings: -1px

### Line Height

- Display metrics (40-48px): 1.0
- Headings (18-24px): 1.15
- Body text (14px): 1.5
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                   |
| ----- | --------------------------------------- |
| 0px   | Sidebar items (padding-only separation) |
| 4px   | Inline icon+text pairs                  |
| 6px   | Status indicator groups                 |
| 8px   | Metric change rows, compact lists       |
| 12px  | Button groups, form elements            |
| 16px  | Between sidebar nav items, table rows   |
| 20px  | Card internal sections                  |
| 24px  | Between cards in a grid                 |
| 28px  | Content area major sections             |
| 32px  | Top-level page sections                 |
| 48px  | Large section breaks                    |

### Padding Scale

| Value    | Usage                              |
| -------- | ---------------------------------- |
| [12, 16] | Sidebar nav items                  |
| [10, 16] | Standard buttons                   |
| [8, 16]  | Input fields                       |
| 24px     | Card padding, content insets       |
| 28px     | Metric card padding                |
| [32, 40] | Content area padding               |
| [56, 48] | Content area with generous spacing |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 80px wide, vertical, space_between, icon-only
- Content area: fill_container, vertical, gap 28-48, padding [32, 40]
- Sidebar indicator: 3px wide gold left border on active item

## Corner Radius

| Value | Usage      | Rationale                          |
| ----- | ---------- | ---------------------------------- |
| 0px   | Everything | Architectural precision throughout |

Design rationale: The brutalist philosophy demands absolute geometric purity. Every element is a perfect rectangle, creating a grid of uncompromising precision. This absence of softening forces the gold accent color and typography to carry all visual interest, producing a dramatic contrast between the rigid structure and the luxurious details. No exceptions.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (the organic curves of Lucide icons create intentional tension with the sharp rectangular containers)

### Commonly Used Icons

layout-dashboard, bar-chart-2, users, settings, folder, file-text, trending-up, trending-down, search, bell, chevron-down, chevron-right, plus, x, edit, trash-2, download, upload, filter, calendar, clock, check

### Icon Sizes

| Size | Usage                                       |
| ---- | ------------------------------------------- |
| 14px | Inline metric indicators, table row actions |
| 18px | Sidebar navigation icons, list item icons   |
| 20px | Card header action icons                    |
| 24px | Primary sidebar icons, header actions       |

### Icon Color States

| State      | Color   | Usage                                     |
| ---------- | ------- | ----------------------------------------- |
| Active     | #C9A962 | Selected sidebar item, active navigation  |
| Default    | #6A6A6A | Inactive sidebar items, secondary actions |
| Muted      | #3A3A3A | Disabled icons, placeholder states        |
| On Surface | #FFFFFF | Icons on accent-colored backgrounds       |
