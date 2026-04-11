---
name: 'fintech-dark-blue-light'
tags: [fintech, blue-accent, light-mode, professional, data-focused, clean, sidebar]
platform: webapp
---

## Style Summary

A precise, data-confident interface designed for financial dashboards, portfolio trackers, and analytics platforms. The cool slate background (#F1F5F9) sets a clinical, focused tone, while dark navy (#0F172A) serves as both primary text and accent — conveying authority and trust. DM Sans delivers clean geometric headings and body text, while DM Mono handles numerical data, currency values, and timestamps with monospaced precision. Generous 12px card radii and subtle inset surfaces create a layered depth system that organizes dense financial data without visual clutter.

Key aesthetics:

- **Data-dense clarity**: DM Mono for all numerical values ensures column alignment and instant readability
- **Dark navy authority**: #0F172A as accent conveys financial trustworthiness and institutional confidence
- **Slate canvas**: #F1F5F9 background provides subtle contrast for white card surfaces
- **Sidebar navigation**: 240px sidebar with section grouping for complex financial app navigation
- **Layered surfaces**: Three background levels (slate → white → inset) create depth hierarchy
- **Metric-first layout**: Large numeric displays with trend indicators and sparkline-compatible spacing

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                             |
| --------------- | ------- | ------------------------------------------------- |
| Page Background | #F1F5F9 | Root page background (cool slate)                 |
| Card Surface    | #FFFFFF | Cards, data panels, chart containers              |
| Inset Surface   | #F8FAFC | Table backgrounds, input fields, secondary panels |
| Sidebar Surface | #FFFFFF | Sidebar background with right border              |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #0F172A | Headings, primary values, metric labels   |
| Secondary Text | #475569 | Body text, descriptions, secondary data   |
| Tertiary Text  | #94A3B8 | Timestamps, axis labels, helper text      |
| Muted Text     | #CBD5E1 | Placeholders, disabled text, empty states |
| Accent Text    | #0F172A | Active navigation, emphasized values      |

### Border Colors

| Token          | Value   | Usage                               |
| -------------- | ------- | ----------------------------------- |
| Default Border | #E2E8F0 | Card borders, table rules, dividers |
| Subtle Border  | #F1F5F9 | Light separators, inset borders     |
| Active Border  | #0F172A | Focused inputs, active indicators   |
| Sidebar Border | #E2E8F0 | Sidebar right edge border           |

### Accent Colors

| Token          | Value     | Usage                                        |
| -------------- | --------- | -------------------------------------------- |
| Primary Accent | #0F172A   | Primary buttons, active states, key controls |
| Accent Hover   | #1E293B   | Button hover, pressed states                 |
| Accent Light   | #0F172A10 | Selected row backgrounds, hover tints        |
| Positive       | #16A34A   | Gains, upward trends, positive returns       |
| Positive Light | #16A34A15 | Positive metric backgrounds                  |
| Negative       | #DC2626   | Losses, downward trends, negative returns    |
| Negative Light | #DC262615 | Negative metric backgrounds                  |
| Warning        | #D97706   | Pending states, moderate alerts              |
| Info           | #2563EB   | Informational badges, chart highlights       |

## Typography

### Font Families

| Role              | Family  | Usage                                                        |
| ----------------- | ------- | ------------------------------------------------------------ |
| Headings          | DM Sans | Section headings, card titles, navigation labels             |
| Body / Functional | DM Sans | Body text, descriptions, buttons, form labels                |
| Data / Mono       | DM Mono | Metric values, currency, percentages, timestamps, table data |

### Type Scale

| Level        | Size | Font    | Weight | Usage                                   |
| ------------ | ---- | ------- | ------ | --------------------------------------- |
| Display      | 40px | DM Sans | 700    | Page title, dashboard heading           |
| Metric Large | 36px | DM Mono | 600    | Primary portfolio value, hero metric    |
| Metric       | 28px | DM Mono | 600    | Card metric values, KPI numbers         |
| Title 1      | 22px | DM Sans | 600    | Section headings                        |
| Title 2      | 16px | DM Sans | 600    | Card titles, panel headings             |
| Body         | 14px | DM Sans | 400    | Descriptions, body text                 |
| Label        | 12px | DM Sans | 500    | Column headers, field labels, nav items |
| Data         | 13px | DM Mono | 400    | Table cell values, data points          |
| Data Small   | 11px | DM Mono | 400    | Axis labels, timestamps, footnotes      |
| Caption      | 11px | DM Sans | 400    | Helper text, captions                   |
| Overline     | 10px | DM Sans | 600    | Category labels (uppercase)             |
| Micro        | 9px  | DM Mono | 400    | Sparkline labels, micro data points     |

### Font Weights

| Weight   | Value | Usage                                        |
| -------- | ----- | -------------------------------------------- |
| Regular  | 400   | Body text, data values, descriptions         |
| Medium   | 500   | Labels, column headers, navigation           |
| Semibold | 600   | Card titles, metric values, section headings |
| Bold     | 700   | Display heading, page titles                 |

### Letter Spacing

- Display (40px): -1px
- Metric values (28-36px): -0.5px (monospaced needs less tightening)
- Section headings (16-22px): -0.3px
- Overline / uppercase labels: +1.5px
- Body text: 0px
- Monospace data: 0px

### Line Height

- Display (40px): 1.0
- Metric values (28-36px): 1.0
- Headings (16-22px): 1.2
- Body (14px): 1.5
- Data/Captions (9-13px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                        |
| ----- | -------------------------------------------- |
| 2px   | Metric value to change indicator             |
| 4px   | Inline icon+text pairs, trend arrow to value |
| 6px   | Status dot to label                          |
| 8px   | Compact data groups, metric sub-labels       |
| 12px  | Between list items, table rows, nav items    |
| 16px  | Card internal sections, form fields          |
| 20px  | Between cards in a grid, chart to legend     |
| 24px  | Content section gaps                         |
| 32px  | Major section breaks, panel gaps             |
| 48px  | Top-level page regions                       |

### Padding Scale

| Value    | Usage                                 |
| -------- | ------------------------------------- |
| [6, 12]  | Compact buttons, badges, status pills |
| [8, 16]  | Input fields, standard buttons        |
| [10, 20] | Large buttons, search bars            |
| [12, 16] | Sidebar nav items                     |
| 20px     | Card padding (compact, data-dense)    |
| 24px     | Card padding (standard)               |
| 28px     | Metric card padding                   |
| [24, 24] | Content area padding                  |
| [20, 16] | Sidebar section padding               |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, white background, right border #E2E8F0, vertical, gap 4
- Content area: fill_container, vertical, gap 24, padding [24, 24]
- Metric cards: horizontal row, fill_container children, gap 20
- Chart panels: full width, height 300-400px, padding 24
- Tables: full width, alternating row backgrounds (#F8FAFC / #FFFFFF)
- Cards use 1px stroke (#E2E8F0) with minimal shadow (0 1px 2px rgba(0,0,0,0.04))

## Corner Radius

| Value | Usage                                         | Rationale                                |
| ----- | --------------------------------------------- | ---------------------------------------- |
| 4px   | Badges, status pills, inline tags             | Subtle softening for small data elements |
| 6px   | Buttons, input fields, dropdowns, table cells | Functional interactive radius            |
| 8px   | Buttons (large), tooltips, popovers           | Intermediate interactive radius          |
| 12px  | Cards, panels, chart containers, modals       | Primary container radius                 |

Design rationale: The 4-12px radius range balances approachability with financial seriousness. Cards at 12px feel modern and refined without the casual softness of higher values. Smaller interactive elements at 6px maintain sharp precision appropriate for data-heavy interfaces. This range mirrors fintech leaders like Stripe Dashboard and Robinhood, where subtle rounding signals modernity while data density demands crisp geometry.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (clean data-friendly stroke style)

### Commonly Used Icons

layout-dashboard, bar-chart-2, line-chart, pie-chart, trending-up, trending-down, wallet, credit-card, banknote, arrow-up-right, arrow-down-right, search, bell, settings, filter, download, calendar, clock, check-circle, alert-triangle, info, eye, eye-off, lock, shield, refresh-cw, external-link, copy

### Icon Sizes

| Size | Usage                                          |
| ---- | ---------------------------------------------- |
| 12px | Inline trend arrows, micro indicators          |
| 14px | Table row actions, metric change icons         |
| 16px | Sidebar navigation icons, button leading icons |
| 20px | Card header actions, toolbar icons             |
| 24px | Page header actions, primary navigation        |

### Icon Color States

| State     | Color   | Usage                                  |
| --------- | ------- | -------------------------------------- |
| Active    | #0F172A | Active navigation, selected states     |
| Default   | #475569 | Standard icons, secondary actions      |
| Positive  | #16A34A | Upward trend arrows, gain indicators   |
| Negative  | #DC2626 | Downward trend arrows, loss indicators |
| Muted     | #CBD5E1 | Disabled icons, placeholder states     |
| On Accent | #FFFFFF | Icons on dark navy backgrounds         |
