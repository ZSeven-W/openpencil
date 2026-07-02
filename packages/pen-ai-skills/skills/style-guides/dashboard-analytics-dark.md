---
name: 'dashboard-analytics-dark'
tags: [data-focused, dark-mode, dashboard, monospace, cyan-accent, sidebar, tech]
platform: webapp
---

## Style Summary

A data-driven dark interface optimized for analytics dashboards, monitoring panels, and metric-heavy applications. The #111827 background provides a neutral dark canvas where cyan (#06B6D4) accents highlight active data points, chart elements, and interactive controls. IBM Plex Sans delivers clean, professional headings and body text, while IBM Plex Mono handles all numerical data, table values, and chart labels with precision alignment. The three-font system (Sans headings, Sans body, Mono data) creates a clear separation between navigational, descriptive, and quantitative content.

Key aesthetics:

- **Three-tier IBM Plex**: Sans for headings, Sans for body, Mono for data — each role clearly delineated
- **Cyan data accent**: #06B6D4 highlights active metrics, chart lines, and interactive elements
- **Chart-friendly palette**: Six distinct data colors for multi-series charts (cyan, violet, amber, emerald, rose, orange)
- **8px system**: Consistent 8px radius for cards and 6px for buttons creates a cohesive, professional surface
- **Dense information layout**: Compact spacing enables high data density without clutter
- **Sidebar navigation**: 240px sidebar with section headers and icon + text navigation items

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                       |
| --------------- | ------- | ------------------------------------------- |
| Page Background | #111827 | Root page background (gray 900)             |
| Card Surface    | #1F2937 | Cards, panels, chart containers (gray 800)  |
| Sidebar Surface | #111827 | Navigation sidebar background               |
| Inset Surface   | #1A2332 | Input fields, nested containers, table rows |
| Table Row Alt   | #19222F | Alternating table row background            |

### Text Colors

| Token          | Value   | Usage                                                |
| -------------- | ------- | ---------------------------------------------------- |
| Primary Text   | #F9FAFB | Headings, metric values, primary content (gray 50)   |
| Secondary Text | #9CA3AF | Body text, descriptions, secondary labels (gray 400) |
| Tertiary Text  | #6B7280 | Timestamps, captions, inactive elements (gray 500)   |
| Muted Text     | #4B5563 | Placeholders, disabled states (gray 600)             |
| Accent Text    | #06B6D4 | Active navigation, data highlights (cyan 500)        |

### Border Colors

| Token               | Value     | Usage                                         |
| ------------------- | --------- | --------------------------------------------- |
| Default Border      | #1F2937   | Card borders, table rules (gray 800)          |
| Subtle Border       | #1A2332   | Section separators, light dividers            |
| Active Border       | #06B6D4   | Focused inputs, active chart selection        |
| Active Muted Border | #06B6D440 | Focus ring, selection highlight (25% opacity) |

### Accent Colors

| Token          | Value     | Usage                                         |
| -------------- | --------- | --------------------------------------------- |
| Primary Accent | #06B6D4   | Active states, primary data series (cyan 500) |
| Accent Subtle  | #06B6D420 | Hover backgrounds, chart area fills           |
| Accent Glow    | #06B6D430 | Active indicator glow                         |
| Success        | #10B981   | Positive metrics, upward trends (emerald 500) |
| Error          | #EF4444   | Negative metrics, alerts (red 500)            |
| Warning        | #F59E0B   | Caution indicators (amber 500)                |

### Chart Data Colors

| Series   | Value   | Usage                       |
| -------- | ------- | --------------------------- |
| Series 1 | #06B6D4 | Primary data series (cyan)  |
| Series 2 | #8B5CF6 | Secondary series (violet)   |
| Series 3 | #F59E0B | Tertiary series (amber)     |
| Series 4 | #10B981 | Quaternary series (emerald) |
| Series 5 | #F43F5E | Quinary series (rose)       |
| Series 6 | #F97316 | Senary series (orange)      |

## Typography

### Font Families

| Role        | Family        | Usage                                               |
| ----------- | ------------- | --------------------------------------------------- |
| Headings    | IBM Plex Sans | Section headings, card titles, navigation labels    |
| Body        | IBM Plex Sans | Body text, descriptions, paragraphs                 |
| Data / Mono | IBM Plex Mono | Metric values, table data, chart labels, timestamps |

### Type Scale

| Level      | Size | Font          | Weight | Usage                                   |
| ---------- | ---- | ------------- | ------ | --------------------------------------- |
| Display    | 36px | IBM Plex Mono | 600    | Hero metric values, KPI numbers         |
| Title 1    | 24px | IBM Plex Sans | 600    | Section headings                        |
| Title 2    | 18px | IBM Plex Sans | 600    | Card titles, subsection headings        |
| Title 3    | 15px | IBM Plex Sans | 500    | List item titles, panel headers         |
| Body       | 14px | IBM Plex Sans | 400    | Descriptions, body text                 |
| Data Large | 20px | IBM Plex Mono | 600    | Card metric values, chart totals        |
| Data       | 14px | IBM Plex Mono | 400    | Table cell values, data points          |
| Label      | 12px | IBM Plex Sans | 500    | Field labels, column headers, nav items |
| Caption    | 11px | IBM Plex Mono | 400    | Timestamps, chart axis labels           |
| Micro      | 10px | IBM Plex Sans | 500    | Status badges, tag labels (uppercase)   |

### Font Weights

| Weight   | Value | Usage                                        |
| -------- | ----- | -------------------------------------------- |
| Regular  | 400   | Body text, data values, descriptions         |
| Medium   | 500   | Labels, navigation items, secondary headings |
| Semibold | 600   | Section titles, metric values, card headings |

### Letter Spacing

- Display metrics (36px): -1px
- Section headings (24px): -0.5px
- Uppercase labels: +1px to +1.5px
- Monospace data: 0px (natural mono spacing)
- Body text: 0px

### Line Height

- Display metrics (36px): 1.0
- Headings (18-24px): 1.2
- Body text (14px): 1.5
- Data / captions (10-14px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                       |
| ----- | ------------------------------------------- |
| 4px   | Inline icon+text pairs, metric unit spacing |
| 8px   | Chart legend items, compact metric groups   |
| 12px  | Between table rows, button groups           |
| 16px  | Card internal sections, nav item spacing    |
| 20px  | Panel sections, filter bar elements         |
| 24px  | Between cards in a grid                     |
| 28px  | Content area major sections                 |
| 32px  | Top-level page sections                     |

### Padding Scale

| Value    | Usage                                |
| -------- | ------------------------------------ |
| [8, 12]  | Table cells, compact elements        |
| [10, 16] | Standard buttons, filter chips       |
| [12, 16] | Sidebar nav items, input fields      |
| 16px     | Card padding (compact, metric cards) |
| 20px     | Standard card padding                |
| 24px     | Large card padding, chart containers |
| [24, 32] | Content area padding                 |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, vertical, space_between, icon + text nav with section headers
- Content area: fill_container, vertical, gap 24-32, padding [24, 32]
- Metric grid: 3-4 columns, gap 24, compact metric cards
- Chart row: 2 columns, gap 24, equal-width chart containers
- Table section: full-width card with 12px row padding

## Corner Radius

| Value | Usage                                    | Rationale                  |
| ----- | ---------------------------------------- | -------------------------- |
| 0px   | Table rules, dividers, progress bars     | Flat data elements         |
| 4px   | Badges, status dots, mini indicators     | Compact data markers       |
| 6px   | Buttons, inputs, filter chips, dropdowns | Interactive element radius |
| 8px   | Cards, chart containers, panels, modals  | Primary container radius   |
| 12px  | Featured cards, summary panels           | Large container emphasis   |

Design rationale: The 6-8px system creates a professional, tool-like interface appropriate for data-heavy applications. Cards at 8px feel like distinct panels in a dashboard layout without becoming too soft or decorative. Buttons and inputs at 6px maintain a slightly tighter, more functional character. The consistency of the radius system reinforces the organized, systematic quality expected in analytics tools.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

layout-dashboard, bar-chart-2, line-chart, pie-chart, trending-up, trending-down, activity, target, users, settings, filter, search, bell, calendar, clock, download, upload, refresh-cw, chevron-right, chevron-down, plus, x, check, eye, database, server, layers, table

### Icon Sizes

| Size | Usage                                   |
| ---- | --------------------------------------- |
| 14px | Inline metric indicators, table actions |
| 16px | Chart legend icons, filter chip icons   |
| 18px | Sidebar navigation icons, list items    |
| 20px | Card header icons, action buttons       |
| 24px | Primary sidebar icons, header actions   |

### Icon Color States

| State      | Color   | Usage                                  |
| ---------- | ------- | -------------------------------------- |
| Active     | #06B6D4 | Selected navigation, active filters    |
| Default    | #9CA3AF | Inactive navigation, secondary actions |
| Muted      | #4B5563 | Disabled icons, placeholder states     |
| On Surface | #F9FAFB | Icons on elevated surfaces             |
