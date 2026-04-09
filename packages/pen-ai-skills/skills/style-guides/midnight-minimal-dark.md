---
name: 'midnight-minimal-dark'
tags: [minimal, dark-mode, calm, quiet, blue-accent, clean, austere]
platform: webapp
---

## Style Summary

An ultra-clean dark interface that embraces restraint and calm. The deep slate background (#0F172A) provides a quiet navy-black canvas where sky blue (#38BDF8) accents appear sparingly — only for active states, links, and key indicators. Inter serves as the sole typeface at varying weights (400-600), creating hierarchy through size and weight alone rather than font switching. The result is a serene, uncluttered workspace that reduces visual noise to an absolute minimum.

Key aesthetics:

- **Single font family**: Inter throughout at weights 400-600, proving that one well-used typeface is sufficient for clear hierarchy
- **Slate palette**: Backgrounds use Tailwind slate tones (#0F172A, #1E293B) for a cool, calming blue-gray undertone
- **Sky blue restraint**: #38BDF8 used only where interaction feedback is needed — active states, links, and focus rings
- **Calm spacing**: Generous gaps (24-40px) between sections give content room to breathe
- **Medium radii**: 8-12px corners create a soft, approachable feel without being playful
- **Invisible chrome**: Borders are nearly imperceptible (#1E293B on #0F172A), letting content float freely

## Color System

### Core Backgrounds

| Token           | Value     | Usage                                          |
| --------------- | --------- | ---------------------------------------------- |
| Page Background | #0F172A   | Root page background (slate 900)               |
| Card Surface    | #1E293B   | Cards, panels, elevated containers (slate 800) |
| Sidebar Surface | #0F172A   | Navigation sidebar background                  |
| Inset Surface   | #162032   | Input fields, nested containers                |
| Hover Surface   | #1E293B80 | Subtle hover highlights                        |

### Text Colors

| Token          | Value   | Usage                                                 |
| -------------- | ------- | ----------------------------------------------------- |
| Primary Text   | #F1F5F9 | Headings, metric values, primary content (slate 100)  |
| Secondary Text | #94A3B8 | Body text, descriptions, secondary labels (slate 400) |
| Tertiary Text  | #64748B | Timestamps, captions, inactive elements (slate 500)   |
| Muted Text     | #475569 | Placeholders, disabled states (slate 600)             |
| Accent Text    | #38BDF8 | Active navigation, links, highlights (sky 400)        |

### Border Colors

| Token               | Value     | Usage                              |
| ------------------- | --------- | ---------------------------------- |
| Default Border      | #1E293B   | Card borders, dividers (slate 800) |
| Subtle Border       | #162032   | Section separators, light dividers |
| Active Border       | #38BDF8   | Focused inputs, active indicators  |
| Active Muted Border | #38BDF840 | Focus ring at 25% opacity          |

### Accent Colors

| Token          | Value     | Usage                                            |
| -------------- | --------- | ------------------------------------------------ |
| Primary Accent | #38BDF8   | Active states, links, focus indicators (sky 400) |
| Accent Subtle  | #38BDF820 | Hover backgrounds, selection tint                |
| Accent Glow    | #38BDF830 | Subtle glow on active elements                   |
| Success        | #34D399   | Positive metrics (emerald 400)                   |
| Error          | #F87171   | Error states (red 400)                           |
| Warning        | #FBBF24   | Warnings, caution (amber 400)                    |

## Typography

### Font Families

| Role       | Family | Usage                                             |
| ---------- | ------ | ------------------------------------------------- |
| Everything | Inter  | All text — headings, body, labels, data, captions |

### Type Scale

| Level      | Size | Font  | Weight | Usage                                          |
| ---------- | ---- | ----- | ------ | ---------------------------------------------- |
| Display    | 36px | Inter | 600    | Hero metric values, primary counters           |
| Title 1    | 24px | Inter | 600    | Section headings                               |
| Title 2    | 18px | Inter | 600    | Card titles, subsection headings               |
| Title 3    | 15px | Inter | 500    | List item titles, panel headers                |
| Body       | 14px | Inter | 400    | Descriptions, body text                        |
| Body Large | 16px | Inter | 400    | Intro paragraphs, prominent descriptions       |
| Label      | 12px | Inter | 500    | Field labels, navigation items, column headers |
| Caption    | 11px | Inter | 400    | Timestamps, metadata                           |
| Micro      | 10px | Inter | 500    | Status badges, auxiliary labels                |

### Font Weights

| Weight   | Value | Usage                                           |
| -------- | ----- | ----------------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions               |
| Medium   | 500   | Labels, list titles, navigation items           |
| Semibold | 600   | Display, headings, card titles — maximum weight |

### Letter Spacing

- Display (36px): -0.5px (subtle tightening)
- Section headings (24px): -0.3px
- Uppercase labels: +1px
- Body text: 0px

### Line Height

- Display (36px): 1.1
- Headings (18-24px): 1.2
- Body text (14-16px): 1.5
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                     |
| ----- | ----------------------------------------- |
| 4px   | Inline icon+text pairs                    |
| 8px   | Status indicator groups, compact sections |
| 12px  | Between list items, button groups         |
| 16px  | Card internal sections, nav item spacing  |
| 20px  | Form field groups, panel sections         |
| 24px  | Between cards in a grid                   |
| 32px  | Content area major sections               |
| 40px  | Top-level page section breaks             |

### Padding Scale

| Value    | Usage                              |
| -------- | ---------------------------------- |
| [12, 16] | Sidebar nav items                  |
| [10, 16] | Standard buttons                   |
| [8, 16]  | Input fields                       |
| 20px     | Card padding (compact)             |
| 24px     | Standard card padding              |
| [28, 32] | Content area padding               |
| [32, 40] | Content area with generous spacing |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, vertical, space_between, icon + text nav
- Content area: fill_container, vertical, gap 32-40, padding [28, 32]
- Section spacing: generous gaps let content breathe
- Sidebar indicator: 2px left border in sky blue on active item, subtle accent background

## Corner Radius

| Value | Usage                                  | Rationale                      |
| ----- | -------------------------------------- | ------------------------------ |
| 0px   | Dividers, full-width separators        | Flat structural elements       |
| 6px   | Badges, inputs, small buttons          | Compact interactive elements   |
| 8px   | Buttons, dropdowns, list items         | Standard interactive radius    |
| 10px  | Standard cards, panels                 | Primary container radius       |
| 12px  | Large cards, modals, featured sections | Maximum radius — soft and calm |

Design rationale: The 8-12px range creates a soft, approachable feel that matches the calm, minimal aesthetic. Corners are rounded enough to feel welcoming and modern without becoming playful or bubbly. The medium radii create gentle, quiet surfaces that don't compete for attention — everything is subordinate to the content.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

layout-dashboard, bar-chart-2, users, settings, folder, file-text, search, bell, chevron-right, chevron-down, plus, x, check, trending-up, trending-down, calendar, clock, download, upload, filter, eye, lock, inbox, bookmark, archive

### Icon Sizes

| Size | Usage                                 |
| ---- | ------------------------------------- |
| 14px | Inline indicators, subtle actions     |
| 18px | Sidebar navigation icons, list items  |
| 20px | Card header icons, action buttons     |
| 24px | Primary sidebar icons, header actions |

### Icon Color States

| State      | Color   | Usage                                  |
| ---------- | ------- | -------------------------------------- |
| Active     | #38BDF8 | Selected navigation, active states     |
| Default    | #94A3B8 | Inactive navigation, secondary actions |
| Muted      | #475569 | Disabled icons, placeholder states     |
| On Surface | #F1F5F9 | Icons on elevated surfaces             |
