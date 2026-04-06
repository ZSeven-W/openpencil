---
name: 'japanese-swiss-light'
tags: [swiss, minimal, zen, geometric, monospace, icon-sidebar, red-accent, data-focused]
platform: webapp
---

## Style Summary

A precision-focused light interface that fuses Swiss typographic discipline with Japanese spatial sensibility. Warm off-white backgrounds (#FAFAFA) provide a calm, paper-like canvas, while a single bold red accent (#DC2626) commands attention for status indicators and active states. A narrow 64px icon-only sidebar in solid black creates a strong vertical anchor. Typography pairs Sora with extremely tight letter-spacing for a compressed, data-dense feel, alongside IBM Plex Mono for numerical data.

Key aesthetics:

- **Swiss precision**: All corners at 0px, strict grid alignment, no decorative elements
- **Japanese whitespace**: Generous padding and deliberate emptiness between sections
- **Red focal point**: Single accent #DC2626 used only for active states and status, never decorative
- **Icon-only black sidebar**: 64px narrow black sidebar with white icons, no text labels
- **Tight letter-spacing**: Sora headings compressed -1 to -2px for a dense, authoritative feel
- **Monospace data layer**: IBM Plex Mono for all values, timestamps, and chart labels

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                  |
| --------------- | ------- | -------------------------------------- |
| Page Background | #FAFAFA | Root page background (warm off-white)  |
| Card Surface    | #FFFFFF | Cards, elevated containers             |
| Sidebar Surface | #0D0D0D | Black sidebar background               |
| Inset Surface   | #F5F5F5 | Input fields, search bars, code blocks |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #0D0D0D | Headings, metric values, primary content |
| Secondary Text | #7A7A7A | Body text, descriptions                  |
| Tertiary Text  | #B0B0B0 | Captions, timestamps, inactive labels    |
| Muted Text     | #D0D0D0 | Placeholders, disabled text              |
| Accent Text    | #DC2626 | Active indicators, live status, emphasis |
| Sidebar Text   | #FFFFFF | Sidebar icon color (active)              |

### Border Colors

| Token          | Value   | Usage                               |
| -------------- | ------- | ----------------------------------- |
| Default Border | #E8E8E8 | Card borders, table rules, dividers |
| Subtle Border  | #F0F0F0 | Light separators                    |
| Active Border  | #DC2626 | Focused inputs, active indicators   |
| Sidebar Border | #2A2A2A | Sidebar internal dividers           |

### Accent Colors

| Token          | Value     | Usage                                           |
| -------------- | --------- | ----------------------------------------------- |
| Primary Accent | #DC2626   | Active states, live indicators, primary buttons |
| Accent Glow    | #DC262666 | Status dot glow (40% opacity)                   |
| Accent Light   | #DC262610 | Subtle red background tint                      |
| Success        | #22C55E   | Positive metrics, upward trends                 |
| Success Muted  | #22C55E20 | Success indicator backgrounds                   |

## Typography

### Font Families

| Role          | Family        | Usage                                                            |
| ------------- | ------------- | ---------------------------------------------------------------- |
| UI / Headings | Sora          | Section headings, button labels, navigation, all functional text |
| Data / Mono   | IBM Plex Mono | Metric values, table data, timestamps, chart labels              |

### Type Scale

| Level   | Size | Font          | Weight | Usage                            |
| ------- | ---- | ------------- | ------ | -------------------------------- |
| Display | 40px | Sora          | 600    | Hero section title               |
| Metric  | 36px | IBM Plex Mono | 600    | Primary metric values            |
| Title 1 | 24px | Sora          | 600    | Section headings                 |
| Title 2 | 18px | Sora          | 600    | Card titles, subsection headings |
| Body    | 14px | Sora          | 400    | Descriptions, body text          |
| Label   | 12px | Sora          | 500    | Field labels, column headers     |
| Data    | 13px | IBM Plex Mono | 400    | Table cell values, data points   |
| Caption | 11px | IBM Plex Mono | 400    | Timestamps, chart axis labels    |
| Micro   | 10px | Sora          | 500    | Status badges, auxiliary labels  |
| Tiny    | 9px  | IBM Plex Mono | 400    | Footnotes, micro metadata        |

### Font Weights

| Weight   | Value | Usage                                   |
| -------- | ----- | --------------------------------------- |
| Regular  | 400   | Body text, data values                  |
| Medium   | 500   | Labels, navigation items, badges        |
| Semibold | 600   | Section titles, metric values, headings |

### Letter Spacing

- Display headings (40px): -2px (extremely tight)
- Section headings (18-24px): -1px
- Uppercase labels: +1.5px
- Body text: 0px
- Monospace data: 0px (natural spacing)

### Line Height

- Display (36-40px): 1.0
- Headings (18-24px): 1.15
- Body (14px): 1.5
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                   |
| ----- | --------------------------------------- |
| 0px   | Sidebar items (padding only)            |
| 2px   | Inline micro pairs                      |
| 4px   | Icon+text inline groups                 |
| 6px   | Status dot to label                     |
| 8px   | Compact data rows, metric change groups |
| 10px  | Between compact list items              |
| 12px  | Button groups, nav items                |
| 16px  | Table rows, form fields                 |
| 20px  | Card internal sections                  |
| 24px  | Between cards in grid                   |
| 28px  | Content area section gaps               |
| 32px  | Major section breaks                    |
| 48px  | Top-level page section spacing          |

### Padding Scale

| Value    | Usage                            |
| -------- | -------------------------------- |
| [8, 12]  | Sidebar icon buttons             |
| [10, 16] | Standard buttons                 |
| [8, 16]  | Input fields                     |
| 24px     | Card padding (standard)          |
| 28px     | Metric card padding              |
| [32, 40] | Content area padding             |
| 20px     | Sidebar section vertical padding |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 64px wide, vertical, space_between, black background
- Content area: fill_container, vertical, gap 28-48, padding [32, 40]
- Cards use stroke (#E8E8E8, thickness 1) rather than shadows

## Corner Radius

| Value | Usage      | Rationale                               |
| ----- | ---------- | --------------------------------------- |
| 0px   | Everything | Swiss geometric precision, no softening |

Design rationale: The all-zero radius philosophy aligns with Swiss International Typographic Style principles. Every element is a precise rectangle, creating an orderly grid that lets typography and color carry the visual hierarchy. The rigid geometry is softened by generous whitespace and the warm #FAFAFA background, achieving the zen-like calm of Japanese spatial design.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

layout-dashboard, bar-chart-2, users, settings, folder, file-text, trending-up, trending-down, search, bell, chevron-down, chevron-right, plus, x, edit, trash-2, filter, calendar, clock, check, home, activity, database, zap

### Icon Sizes

| Size | Usage                                            |
| ---- | ------------------------------------------------ |
| 14px | Inline metric indicators, table actions          |
| 18px | Sidebar navigation icons                         |
| 20px | Card header icons, compact action buttons        |
| 24px | Sidebar primary icons (centered in 64px sidebar) |

### Icon Color States

| State            | Color   | Usage                                     |
| ---------------- | ------- | ----------------------------------------- |
| Active           | #FFFFFF | Active sidebar icon on black background   |
| Active Content   | #DC2626 | Active state in content area              |
| Default          | #7A7A7A | Inactive content icons, secondary actions |
| Sidebar Inactive | #6A6A6A | Inactive sidebar icons on black           |
| Muted            | #B0B0B0 | Disabled icons, placeholders              |
