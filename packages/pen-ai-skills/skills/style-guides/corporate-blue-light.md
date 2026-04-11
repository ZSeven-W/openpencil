---
name: 'corporate-blue-light'
tags: [corporate, blue-accent, light-mode, clean, professional, sidebar, dual-font]
platform: webapp
---

## Style Summary

A polished, confidence-inspiring enterprise interface built on crisp cool-gray backgrounds (#F8FAFC) and a strong blue accent (#1D4ED8). The paired Inter weights — 600 for headings and 400 for body — deliver a single-family system that feels unified yet establishes clear hierarchy through weight contrast alone. Cards sit on pure white surfaces with subtle 1px borders, while an 80px sidebar in dark navy (#0F172A) anchors the left edge for persistent navigation. Every element radiates professionalism and reliability.

Key aesthetics:

- **Cool-gray canvas**: #F8FAFC background separates content cards cleanly without feeling sterile
- **Confident blue accent**: #1D4ED8 for primary actions, active states, and key indicators
- **Single-family typography**: Inter at two weights (600/400) keeps the interface cohesive and corporate
- **Dark sidebar**: #0F172A sidebar with white text provides strong navigation contrast
- **Moderate radii**: 6-8px corners maintain professionalism without feeling rigid or playful
- **Subtle borders**: 1px #E2E8F0 borders define structure with restrained elegance

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                            |
| --------------- | ------- | ------------------------------------------------ |
| Page Background | #F8FAFC | Root page background (cool-gray)                 |
| Card Surface    | #FFFFFF | Cards, elevated containers, modals               |
| Inset Surface   | #F1F5F9 | Input backgrounds, table stripes, recessed areas |
| Sidebar Surface | #0F172A | Sidebar navigation background (dark navy)        |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #0F172A | Headings, primary labels, metric values   |
| Secondary Text | #475569 | Body text, descriptions, secondary labels |
| Tertiary Text  | #94A3B8 | Captions, timestamps, helper text         |
| Muted Text     | #CBD5E1 | Placeholders, disabled text               |
| Accent Text    | #1D4ED8 | Active nav items, links, highlighted data |

### Border Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Default Border | #E2E8F0 | Card borders, dividers, input outlines |
| Subtle Border  | #F1F5F9 | Light separators, table row dividers   |
| Active Border  | #1D4ED8 | Focused inputs, active tab indicators  |

### Accent Colors

| Token          | Value     | Usage                                          |
| -------------- | --------- | ---------------------------------------------- |
| Primary Accent | #1D4ED8   | Primary buttons, active states, key indicators |
| Accent Hover   | #1E40AF   | Button hover, pressed states                   |
| Accent Light   | #1D4ED820 | Tinted backgrounds, selected rows              |
| Accent Muted   | #1D4ED840 | Badge fills, progress tracks                   |
| Success        | #16A34A   | Positive metrics, completion states            |
| Warning        | #D97706   | Alert indicators, pending states               |
| Error          | #DC2626   | Error states, destructive actions              |

## Typography

### Font Families

| Role              | Family | Usage                                                       |
| ----------------- | ------ | ----------------------------------------------------------- |
| Headings          | Inter  | Section headings, card titles, display metrics (weight 600) |
| Body / Functional | Inter  | Body text, labels, buttons, navigation (weight 400)         |

### Type Scale

| Level    | Size | Font  | Weight | Usage                                   |
| -------- | ---- | ----- | ------ | --------------------------------------- |
| Display  | 40px | Inter | 600    | Page titles, hero headings              |
| Metric   | 32px | Inter | 600    | Primary metric values, KPI numbers      |
| Title 1  | 24px | Inter | 600    | Section headings                        |
| Title 2  | 18px | Inter | 600    | Card titles, subsection headings        |
| Title 3  | 15px | Inter | 600    | List headings, compact card titles      |
| Body     | 14px | Inter | 400    | Descriptions, body text, table cells    |
| Label    | 12px | Inter | 500    | Field labels, column headers, nav items |
| Caption  | 11px | Inter | 400    | Timestamps, secondary metadata          |
| Overline | 11px | Inter | 600    | Section overlines (uppercase)           |
| Micro    | 10px | Inter | 500    | Badges, status labels                   |

### Font Weights

| Weight   | Value | Usage                               |
| -------- | ----- | ----------------------------------- |
| Regular  | 400   | Body text, descriptions, table data |
| Medium   | 500   | Labels, navigation items, badges    |
| Semibold | 600   | Headings, metric values, buttons    |

### Letter Spacing

- Display (40px): -1px
- Section headings (18-24px): -0.5px
- Overline / uppercase labels: +1.5px
- Body text: 0px
- Metric values: -0.5px

### Line Height

- Display (40px): 1.0
- Headings (18-24px): 1.2
- Body (14-15px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                            |
| ----- | ------------------------------------------------ |
| 4px   | Inline icon+text pairs                           |
| 6px   | Status indicator to label                        |
| 8px   | Compact element groups, button icon to text      |
| 12px  | Between list items, nav items, form fields       |
| 16px  | Card internal sections, table row height spacing |
| 20px  | Between cards in a grid                          |
| 24px  | Content section gaps                             |
| 32px  | Major section breaks                             |
| 48px  | Top-level page sections                          |

### Padding Scale

| Value    | Usage                          |
| -------- | ------------------------------ |
| [6, 12]  | Compact buttons, badges        |
| [8, 16]  | Input fields, standard buttons |
| [10, 20] | Large buttons, search bars     |
| [12, 16] | Sidebar nav items              |
| 20px     | Card padding (compact)         |
| 24px     | Card padding (standard)        |
| [32, 32] | Content area padding           |
| [16, 20] | Sidebar section padding        |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 80px expanding to 240px, vertical, dark navy (#0F172A)
- Content area: fill_container, vertical, gap 24-32, padding [32, 32]
- Cards use 1px stroke (#E2E8F0) with subtle shadow (0 1px 3px rgba(0,0,0,0.05))
- Header bar: 64px height, white background, bottom border #E2E8F0

## Corner Radius

| Value | Usage                               | Rationale                              |
| ----- | ----------------------------------- | -------------------------------------- |
| 4px   | Badges, inline tags, tooltips       | Minimal softening for small elements   |
| 6px   | Buttons, input fields, dropdowns    | Professional interactive radius        |
| 8px   | Cards, modals, sidebar sections     | Primary container radius               |
| 10px  | Large feature cards, dialog windows | Slightly softer for prominent surfaces |

Design rationale: The 4-10px radius range projects corporate reliability. Corners are softened enough to feel modern but never rounded enough to appear casual. This measured approach mirrors enterprise UI systems like Salesforce Lightning and Microsoft Fluent, where subtle radii signal quality engineering without sacrificing authority.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (clean professional stroke style)

### Commonly Used Icons

layout-dashboard, bar-chart-2, users, settings, folder, file-text, trending-up, trending-down, search, bell, chevron-down, chevron-right, plus, x, edit, trash-2, download, upload, filter, calendar, clock, check, briefcase, building, shield, lock, mail, phone

### Icon Sizes

| Size | Usage                                                      |
| ---- | ---------------------------------------------------------- |
| 14px | Inline metric indicators, table row actions                |
| 16px | Sidebar navigation icons (collapsed), button leading icons |
| 20px | Card header action icons, toolbar actions                  |
| 24px | Sidebar primary icons (expanded), page header actions      |

### Icon Color States

| State            | Color   | Usage                              |
| ---------------- | ------- | ---------------------------------- |
| Active           | #1D4ED8 | Active navigation, selected states |
| Default          | #475569 | Standard icons, secondary actions  |
| Muted            | #CBD5E1 | Disabled icons, placeholder states |
| Sidebar          | #FFFFFF | Active sidebar icons on dark navy  |
| Sidebar Inactive | #94A3B8 | Inactive sidebar icons             |
| On Accent        | #FFFFFF | Icons on blue-accent backgrounds   |
