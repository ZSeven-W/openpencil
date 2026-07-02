---
name: 'enterprise-slate-dark'
tags: [enterprise, dark-mode, professional, sidebar, data-focused, neutral, webapp]
platform: webapp
---

## Style Summary

A commanding enterprise dark interface built for B2B SaaS platforms, admin dashboards, and professional tooling. The deep slate #1E293B background establishes authority and focus, while the confident blue accent (#3B82F6) provides clear wayfinding across dense information layouts. IBM Plex Sans at two weights (600/400) delivers the same industrial clarity trusted by enterprise systems — legible at small sizes, professional at large ones. Moderate 6-8px radii maintain a businesslike posture without feeling rigid. The result is an interface that communicates reliability, security, and operational readiness.

Key aesthetics:

- **Deep slate foundation**: #1E293B background is warmer than pure dark, reducing eye strain during extended sessions
- **Enterprise blue accent**: #3B82F6 for primary actions and navigation — universally trusted, unambiguous
- **IBM Plex authority**: Industrial sans-serif with excellent legibility at data-dense sizes
- **Professional radii**: 6-8px corners are sharp enough for seriousness, soft enough to feel modern
- **Dense information support**: Compact spacing enables high data density without visual clutter
- **Sidebar-driven navigation**: 240px sidebar with icon + text items for complex feature hierarchies

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                       |
| --------------- | ------- | ------------------------------------------- |
| Page Background | #1E293B | Root page background (slate 800)            |
| Card Surface    | #273548 | Cards, panels, elevated containers          |
| Sidebar Surface | #0F172A | Sidebar navigation background (slate 900)   |
| Inset Surface   | #1A2332 | Input fields, nested containers, table rows |
| Table Row Alt   | #222E3F | Alternating table row background            |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #F1F5F9 | Headings, metric values, primary content  |
| Secondary Text | #94A3B8 | Body text, descriptions, secondary labels |
| Tertiary Text  | #64748B | Timestamps, captions, inactive elements   |
| Muted Text     | #475569 | Placeholders, disabled states             |
| Accent Text    | #3B82F6 | Active navigation, links, data highlights |

### Border Colors

| Token          | Value     | Usage                                  |
| -------------- | --------- | -------------------------------------- |
| Default Border | #334155   | Card borders, table rules, dividers    |
| Subtle Border  | #293548   | Section separators, light dividers     |
| Active Border  | #3B82F6   | Focused inputs, active selection       |
| Focus Ring     | #3B82F640 | Accessibility focus ring (25% opacity) |

### Accent Colors

| Token          | Value     | Usage                                     |
| -------------- | --------- | ----------------------------------------- |
| Primary Accent | #3B82F6   | Primary buttons, active states (blue 500) |
| Accent Hover   | #2563EB   | Button hover, pressed states (blue 600)   |
| Accent Subtle  | #3B82F620 | Hover backgrounds, selected rows          |
| Accent Muted   | #3B82F640 | Badge fills, progress tracks              |
| Success        | #22C55E   | Positive metrics, upward trends           |
| Warning        | #F59E0B   | Alert indicators, pending states          |
| Error          | #EF4444   | Negative metrics, destructive actions     |
| Info           | #06B6D4   | Informational elements, tooltips          |

## Typography

### Font Families

| Role              | Family        | Usage                                                       |
| ----------------- | ------------- | ----------------------------------------------------------- |
| Headings          | IBM Plex Sans | Section headings, card titles, display metrics (weight 600) |
| Body / Functional | IBM Plex Sans | Body text, labels, buttons, navigation (weight 400)         |

### Type Scale

| Level    | Size | Font          | Weight | Usage                                   |
| -------- | ---- | ------------- | ------ | --------------------------------------- |
| Display  | 36px | IBM Plex Sans | 600    | Page titles, hero headings              |
| Metric   | 28px | IBM Plex Sans | 600    | Primary metric values, KPI numbers      |
| Title 1  | 22px | IBM Plex Sans | 600    | Section headings                        |
| Title 2  | 17px | IBM Plex Sans | 600    | Card titles, subsection headings        |
| Title 3  | 15px | IBM Plex Sans | 500    | List headings, compact card titles      |
| Body     | 14px | IBM Plex Sans | 400    | Descriptions, body text, table cells    |
| Label    | 12px | IBM Plex Sans | 500    | Field labels, column headers, nav items |
| Caption  | 11px | IBM Plex Sans | 400    | Timestamps, secondary metadata          |
| Overline | 11px | IBM Plex Sans | 600    | Section overlines (uppercase)           |
| Micro    | 10px | IBM Plex Sans | 500    | Badges, status labels                   |

### Font Weights

| Weight   | Value | Usage                                      |
| -------- | ----- | ------------------------------------------ |
| Regular  | 400   | Body text, descriptions, table data        |
| Medium   | 500   | Labels, navigation items, compact headings |
| Semibold | 600   | Section titles, metric values, buttons     |

### Letter Spacing

- Display (36px): -0.75px
- Section headings (17-22px): -0.5px
- Overline / uppercase labels: +1.5px
- Body text: 0px
- Metric values: -0.5px

### Line Height

- Display (36px): 1.1
- Headings (17-22px): 1.2
- Body (14px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                       |
| ----- | ------------------------------------------- |
| 4px   | Inline icon+text pairs, status indicators   |
| 8px   | Compact element groups, button icon to text |
| 12px  | Between list items, nav items, table rows   |
| 16px  | Card internal sections, form fields         |
| 20px  | Panel sections, filter bar elements         |
| 24px  | Between cards in a grid                     |
| 28px  | Content area sections                       |
| 32px  | Top-level page sections                     |

### Padding Scale

| Value    | Usage                                 |
| -------- | ------------------------------------- |
| [6, 12]  | Compact buttons, badges, status chips |
| [8, 16]  | Standard buttons, input fields        |
| [10, 20] | Large buttons, search bars            |
| [12, 16] | Sidebar nav items                     |
| 16px     | Card padding (compact)                |
| 20px     | Card padding (standard)               |
| 24px     | Large card padding, panel sections    |
| [24, 32] | Content area padding                  |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, vertical, dark slate (#0F172A) with blue active indicators and section headers
- Content area: fill_container, vertical, gap 24-28, padding [24, 32]
- Cards use 1px stroke (#334155) with no shadow (flat dark surfaces)
- Header bar: 56px height, slate (#1E293B), bottom border #334155
- Table sections: full-width cards with compact 12px row spacing
- Metric grids: 3-4 columns, gap 20

## Corner Radius

| Value | Usage                                   | Rationale                              |
| ----- | --------------------------------------- | -------------------------------------- |
| 2px   | Inline badges, micro indicators         | Barely perceptible softening           |
| 4px   | Table cells, compact elements, tooltips | Minimal data element radius            |
| 6px   | Buttons, input fields, dropdowns, chips | Professional interactive radius        |
| 8px   | Cards, modals, panels, sidebar sections | Primary container radius               |
| 10px  | Featured panels, dialog windows         | Slightly softer for prominent surfaces |

Design rationale: The 6-8px system projects enterprise seriousness without feeling dated or rigid. Cards at 8px have just enough softening to feel contemporary, while buttons at 6px maintain a tight, functional character. This restrained approach mirrors enterprise platforms like Datadog, Grafana, and Salesforce — systems where the interface must feel like a reliable tool rather than a consumer product. The consistency of the radius scale reinforces the organized, systematic quality that enterprise users expect.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (clean professional stroke)

### Commonly Used Icons

layout-dashboard, bar-chart-2, users, settings, database, server, shield, lock, key, folder, file-text, search, bell, filter, chevron-right, chevron-down, plus, x, check, edit, trash-2, download, upload, refresh-cw, globe, building, briefcase, clock, calendar, activity

### Icon Sizes

| Size | Usage                                          |
| ---- | ---------------------------------------------- |
| 14px | Inline indicators, table row actions           |
| 16px | Sidebar navigation icons, button leading icons |
| 18px | List item icons, compact toolbar               |
| 20px | Card header action icons, toolbar actions      |
| 24px | Primary sidebar icons, page header actions     |

### Icon Color States

| State            | Color   | Usage                              |
| ---------------- | ------- | ---------------------------------- |
| Active           | #3B82F6 | Active navigation, selected states |
| Default          | #94A3B8 | Standard icons, secondary actions  |
| Muted            | #475569 | Disabled icons, placeholder states |
| On Surface       | #F1F5F9 | Icons on elevated card surfaces    |
| Sidebar Active   | #3B82F6 | Active sidebar icons               |
| Sidebar Inactive | #64748B | Inactive sidebar icons             |
