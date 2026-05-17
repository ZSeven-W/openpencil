---
name: 'nordic-frost-light'
tags: [scandinavian, light-mode, calm, soft, blue-accent, minimal, webapp, neutral]
platform: webapp
---

## Style Summary

A serene, accessibility-first interface inspired by Nordic design principles — clarity over decoration, function over flourish. The cool #F5F7FA background evokes overcast Scandinavian skies, while the muted slate-blue accent (#64748B) provides gentle guidance without demanding attention. Atkinson Hyperlegible, designed by the Braille Institute for maximum readability, serves as both heading and body font — its distinct letterforms ensure legibility at every size, embodying the Nordic commitment to universal design. Generous 8-12px radii create soft, approachable surfaces that feel like smooth river stones.

Key aesthetics:

- **Accessibility-first typography**: Atkinson Hyperlegible ensures distinct letterforms for all users
- **Muted slate accent**: #64748B provides calm, non-aggressive interactive guidance
- **Cool-gray atmosphere**: #F5F7FA background creates a quiet, fog-like canvas
- **Soft radii**: 8-12px corners produce smooth, tactile surfaces
- **Restrained contrast**: Intentionally moderate contrast ratios that still meet WCAG AA
- **Generous whitespace**: Ample breathing room between elements reduces cognitive load

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                            |
| --------------- | ------- | ------------------------------------------------ |
| Page Background | #F5F7FA | Root page background (cool fog-gray)             |
| Card Surface    | #FFFFFF | Cards, elevated containers, modals               |
| Inset Surface   | #EDF0F4 | Input backgrounds, recessed areas, table stripes |
| Hover Surface   | #E8ECF1 | Hover states, pressed backgrounds                |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #1E293B | Headings, primary labels, metric values   |
| Secondary Text | #475569 | Body text, descriptions, secondary labels |
| Tertiary Text  | #94A3B8 | Captions, timestamps, helper text         |
| Muted Text     | #CBD5E1 | Placeholders, disabled text               |
| Accent Text    | #64748B | Active nav items, links, emphasized data  |

### Border Colors

| Token          | Value     | Usage                                  |
| -------------- | --------- | -------------------------------------- |
| Default Border | #E2E8F0   | Card borders, dividers, input outlines |
| Subtle Border  | #EDF0F4   | Light separators, table row dividers   |
| Active Border  | #64748B   | Focused inputs, active tab indicators  |
| Focus Ring     | #64748B40 | Accessibility focus ring (25% opacity) |

### Accent Colors

| Token          | Value     | Usage                                          |
| -------------- | --------- | ---------------------------------------------- |
| Primary Accent | #64748B   | Primary buttons, active states, key indicators |
| Accent Hover   | #475569   | Button hover, pressed states                   |
| Accent Light   | #64748B15 | Tinted backgrounds, selected rows              |
| Accent Muted   | #64748B30 | Badge fills, progress tracks                   |
| Success        | #22C55E   | Positive metrics, completion states            |
| Warning        | #EAB308   | Alert indicators, pending states               |
| Error          | #EF4444   | Error states, destructive actions              |
| Info           | #3B82F6   | Informational callouts                         |

## Typography

### Font Families

| Role              | Family                | Usage                                                    |
| ----------------- | --------------------- | -------------------------------------------------------- |
| Headings          | Atkinson Hyperlegible | Section headings, card titles, display text (weight 700) |
| Body / Functional | Atkinson Hyperlegible | Body text, labels, buttons, navigation (weight 400)      |

### Type Scale

| Level      | Size | Font                  | Weight | Usage                                   |
| ---------- | ---- | --------------------- | ------ | --------------------------------------- |
| Display    | 36px | Atkinson Hyperlegible | 700    | Page titles, hero headings              |
| Metric     | 28px | Atkinson Hyperlegible | 700    | Primary metric values, KPI numbers      |
| Title 1    | 24px | Atkinson Hyperlegible | 700    | Section headings                        |
| Title 2    | 18px | Atkinson Hyperlegible | 700    | Card titles, subsection headings        |
| Title 3    | 15px | Atkinson Hyperlegible | 700    | List headings, compact card titles      |
| Body Large | 16px | Atkinson Hyperlegible | 400    | Lead paragraphs, featured descriptions  |
| Body       | 14px | Atkinson Hyperlegible | 400    | Standard body text, table cells         |
| Label      | 12px | Atkinson Hyperlegible | 700    | Field labels, column headers, nav items |
| Caption    | 11px | Atkinson Hyperlegible | 400    | Timestamps, secondary metadata          |
| Overline   | 11px | Atkinson Hyperlegible | 700    | Section overlines (uppercase)           |
| Micro      | 10px | Atkinson Hyperlegible | 700    | Badges, status labels                   |

### Font Weights

| Weight  | Value | Usage                               |
| ------- | ----- | ----------------------------------- |
| Regular | 400   | Body text, descriptions, captions   |
| Bold    | 700   | Headings, labels, buttons, emphasis |

### Letter Spacing

- Display (36px): -0.5px
- Section headings (18-24px): -0.25px
- Overline / uppercase labels: +1.5px
- Body text: 0px
- Metric values: -0.5px

### Line Height

- Display (36px): 1.1
- Headings (18-24px): 1.25
- Body (14-16px): 1.6 (generous for readability)
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                     |
| ----- | ----------------------------------------- |
| 4px   | Inline icon+text pairs                    |
| 8px   | Compact element groups, status indicators |
| 12px  | Between list items, nav items             |
| 16px  | Card internal sections, form fields       |
| 20px  | Between cards in a grid                   |
| 24px  | Content section gaps                      |
| 32px  | Major section breaks                      |
| 48px  | Top-level page sections                   |

### Padding Scale

| Value    | Usage                          |
| -------- | ------------------------------ |
| [8, 14]  | Compact buttons, badges        |
| [10, 18] | Standard buttons, input fields |
| [12, 24] | Large buttons, search bars     |
| [12, 16] | Sidebar nav items              |
| 20px     | Card padding (compact)         |
| 24px     | Card padding (standard)        |
| 32px     | Large container padding        |
| [32, 36] | Content area padding           |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, vertical, white background with slate-blue active indicators
- Content area: fill_container, vertical, gap 24-32, padding [32, 36]
- Cards use 1px stroke (#E2E8F0) with very subtle shadow (0 1px 2px rgba(0,0,0,0.04))
- Header bar: 60px height, white background, bottom border #E2E8F0
- Generous whitespace between sections creates breathing room

## Corner Radius

| Value | Usage                                        | Rationale                              |
| ----- | -------------------------------------------- | -------------------------------------- |
| 4px   | Badges, inline tags, tooltips                | Minimal softening for compact elements |
| 8px   | Buttons, input fields, dropdowns             | Smooth interactive radius              |
| 10px  | Cards, modals, sidebar sections              | Primary container radius               |
| 12px  | Featured cards, dialog windows, large panels | Softer container emphasis              |

Design rationale: The 8-12px range produces surfaces that feel like smooth, natural objects — river stones or wooden blocks in a Scandinavian home. Corners are soft enough to convey warmth and approachability but never so round that they become playful or informal. This measured softness aligns with the Nordic design principle of "lagom" — just the right amount, never excessive.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (matches the soft radius system)

### Commonly Used Icons

home, layout-dashboard, bar-chart, users, settings, folder, file-text, search, bell, chevron-right, chevron-down, plus, x, check, edit, trash-2, download, mail, calendar, clock, bookmark, heart, star, filter, arrow-right, sun, cloud, map-pin, compass

### Icon Sizes

| Size | Usage                                          |
| ---- | ---------------------------------------------- |
| 14px | Inline indicators, table row actions           |
| 16px | Sidebar navigation icons, button leading icons |
| 20px | Card header action icons, toolbar actions      |
| 24px | Primary sidebar icons, page header actions     |

### Icon Color States

| State     | Color   | Usage                                  |
| --------- | ------- | -------------------------------------- |
| Active    | #64748B | Active navigation, selected states     |
| Default   | #475569 | Standard icons, secondary actions      |
| Muted     | #CBD5E1 | Disabled icons, placeholder states     |
| On Accent | #FFFFFF | Icons on slate-blue accent backgrounds |
| Hover     | #1E293B | Icon hover state, darkened             |
