---
name: 'elegant-luxury-dark'
tags: [elegant, luxury, dark-mode, gold-accent, serif, sophisticated, sidebar, refined]
platform: webapp
---

## Style Summary

A refined dark interface that draws from classical editorial and luxury brand aesthetics. Cormorant Garamond headlines bring a sense of heritage and sophistication, while Inter body text ensures readability. Gold accents (#C9A962) are used sparingly and deliberately, appearing in status indicators, active states, and decorative strokes. The wide 280px sidebar with icon-led navigation provides a spacious, unhurried feel that communicates premium quality.

Key aesthetics:

- **Serif display typography**: Cormorant Garamond at generous sizes for metric values and headings conveys timelessness
- **Gold restrained**: Single gold accent (#C9A962) used at varying opacities for hierarchy without garishness
- **Zero radius**: All corners at 0px, aligning with the classical, architectural precision
- **Wide icon-led sidebar**: 280px with icon + text navigation items, providing clear wayfinding
- **Monospace data layer**: JetBrains Mono for numerical values and timestamps adds a precision contrast to the serif display
- **Thin gold strokes**: 1px #2A2A2A borders with occasional gold (#C9A962) highlights for active regions

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                |
| --------------- | ------- | ------------------------------------ |
| Page Background | #0A0A0A | Root page background                 |
| Card Surface    | #0D0D0D | Cards, elevated containers           |
| Sidebar Surface | #0A0A0A | Navigation sidebar background        |
| Inset Surface   | #141414 | Nested containers, input backgrounds |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #FFFFFF | Headings, metric values, primary content |
| Secondary Text | #6A6A6A | Body text, descriptions, nav labels      |
| Tertiary Text  | #4A4A4A | Timestamps, captions, inactive items     |
| Muted Text     | #3A3A3A | Placeholders, disabled text              |
| Accent Text    | #C9A962 | Active navigation, status labels, links  |

### Border Colors

| Token               | Value     | Usage                                       |
| ------------------- | --------- | ------------------------------------------- |
| Default Border      | #2A2A2A   | Card borders, table rules, sidebar dividers |
| Subtle Border       | #1A1A1A   | Light separators, nested container edges    |
| Accent Border       | #C9A962   | Active sidebar indicator, focused inputs    |
| Accent Muted Border | #C9A96240 | Decorative gold stroke at 25% opacity       |
| Accent Light Border | #C9A96230 | Hover state borders, subtle highlights      |

### Accent Colors

| Token          | Value     | Usage                                        |
| -------------- | --------- | -------------------------------------------- |
| Primary Accent | #C9A962   | Active states, gold highlights, icons        |
| Accent Glow    | #C9A96266 | Status dot glow effect (40% opacity)         |
| Accent Subtle  | #C9A96220 | Background tint for active sidebar items     |
| Error          | #8B5A5A   | Destructive actions (muted to match palette) |

## Typography

### Font Families

| Role              | Family             | Usage                                                  |
| ----------------- | ------------------ | ------------------------------------------------------ |
| Display / Serif   | Cormorant Garamond | Metric values, hero headings, section titles           |
| Body / Functional | Inter              | Body text, descriptions, navigation labels, buttons    |
| Data / Mono       | JetBrains Mono     | Numerical data, timestamps, chart labels, table values |

### Type Scale

| Level   | Size | Font               | Weight | Usage                            |
| ------- | ---- | ------------------ | ------ | -------------------------------- |
| Display | 48px | Cormorant Garamond | 500    | Hero section title               |
| Metric  | 40px | Cormorant Garamond | 500    | Primary metric values            |
| Title 1 | 24px | Cormorant Garamond | 600    | Section headings                 |
| Title 2 | 18px | Inter              | 600    | Card titles, subsection headings |
| Body    | 14px | Inter              | 400    | Descriptions, body text          |
| Label   | 12px | Inter              | 500    | Field labels, navigation items   |
| Data    | 13px | JetBrains Mono     | 400    | Table cell values, chart data    |
| Caption | 11px | JetBrains Mono     | 400    | Timestamps, metadata             |
| Micro   | 10px | Inter              | 500    | Status badges, auxiliary labels  |

### Font Weights

| Weight   | Value | Usage                                     |
| -------- | ----- | ----------------------------------------- |
| Regular  | 400   | Body text, data values, descriptions      |
| Medium   | 500   | Labels, serif display metrics, navigation |
| Semibold | 600   | Section titles, card headings             |

### Letter Spacing

- Serif display (40-48px): -1px
- Uppercase labels: +1.5px
- Body text: 0px
- Monospace data: 0px

### Line Height

- Display (40-48px): 1.0
- Headings (18-24px): 1.15
- Body (14px): 1.5
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                              |
| ----- | -------------------------------------------------- |
| 0px   | Sidebar items (use padding for separation)         |
| 4px   | Inline icon+text pairs                             |
| 6px   | Status dot to label                                |
| 8px   | Metric change indicator groups                     |
| 12px  | Between sidebar nav items, button groups           |
| 16px  | Table rows, form fields                            |
| 20px  | Card internal sections (header to value to change) |
| 24px  | Between cards in a grid                            |
| 28px  | Content area section gaps                          |
| 32px  | Major section breaks                               |
| 48px  | Top-level content area gap                         |

### Padding Scale

| Value    | Usage                                 |
| -------- | ------------------------------------- |
| [12, 16] | Sidebar nav items                     |
| [10, 16] | Standard buttons                      |
| [8, 16]  | Input fields                          |
| 24px     | Content insets, card standard padding |
| 28px     | Metric card padding                   |
| [32, 48] | Content area padding                  |
| 20px     | Sidebar top/bottom section padding    |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 280px wide, vertical, space_between
- Sidebar structure: top (logo + navigation), bottom (upgrade CTA + user profile)
- Content area: fill_container, vertical, gap 28-48, padding [32, 48]
- Section gap within content: 28px

## Corner Radius

| Value | Usage      | Rationale                         |
| ----- | ---------- | --------------------------------- |
| 0px   | Everything | Classical architectural precision |

Design rationale: The all-zero radius complements the serif typography and gold accents to create a classical, editorial feel. Sharp corners evoke the clean lines of luxury print design and high-end architecture. The precision of the grid reinforces the sophisticated, curated quality of the interface.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

layout-dashboard, bar-chart-2, users, settings, folder, file-text, trending-up, trending-down, search, bell, chevron-down, chevron-right, plus, x, edit, trash-2, download, calendar, clock, check, star, award, crown, gem

### Icon Sizes

| Size | Usage                                        |
| ---- | -------------------------------------------- |
| 14px | Inline metric indicators, table actions      |
| 18px | Sidebar navigation leading icons, list items |
| 20px | Card header icons, action buttons            |
| 24px | Header actions, prominent navigation         |

### Icon Color States

| State      | Color   | Usage                                         |
| ---------- | ------- | --------------------------------------------- |
| Active     | #C9A962 | Selected sidebar, active tab, gold highlights |
| Default    | #6A6A6A | Inactive navigation, secondary actions        |
| Muted      | #4A4A4A | Disabled states, placeholder icons            |
| On Surface | #FFFFFF | Icons on dark elevated surfaces               |
