---
name: 'industrial-neon-dark'
tags: [industrial, neon, dark-mode, neon-green, monospace, tech, sharp-corners]
platform: webapp
---

## Style Summary

A high-voltage dark interface inspired by industrial control panels and factory floor HUDs. Near-black backgrounds (#0C0C0C) are punctuated by electric neon green (#39FF14) accents that recall status LEDs and machine readouts. Space Mono headings and data labels give the entire surface a utilitarian, engineered quality, while Inter body text keeps longer passages readable. Every edge is sharp — corners at 0-4px reinforce the fabricated, machined precision of industrial equipment.

Key aesthetics:

- **Neon on black**: Electric green (#39FF14) on near-black surfaces creates maximum contrast and an unmistakable industrial identity
- **Monospace-first hierarchy**: Space Mono for headings, metrics, and labels evokes machine readouts and terminal displays
- **Sharp machined corners**: 0-4px radii throughout, suggesting stamped metal and CNC-cut panels
- **Glow effects**: Neon green glow (blur 10, 30% opacity) on active elements simulates LED backlighting
- **Minimal surface variation**: Backgrounds stay within #0C0C0C to #161616, letting accent color carry all visual weight
- **Structural borders**: Thin (#1A1A1A) borders define panels; neon borders mark active zones

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                        |
| --------------- | ------- | -------------------------------------------- |
| Page Background | #0C0C0C | Root page background                         |
| Card Surface    | #121212 | Cards, panels, elevated containers           |
| Sidebar Surface | #0C0C0C | Navigation sidebar background                |
| Inset Surface   | #161616 | Input fields, nested containers, code blocks |

### Text Colors

| Token          | Value   | Usage                                               |
| -------------- | ------- | --------------------------------------------------- |
| Primary Text   | #E8E8E8 | Headings, metric values, active labels              |
| Secondary Text | #808080 | Body text, descriptions, secondary labels           |
| Tertiary Text  | #555555 | Timestamps, captions, inactive elements             |
| Muted Text     | #3A3A3A | Placeholders, disabled states                       |
| Accent Text    | #39FF14 | Active navigation, status labels, links, highlights |

### Border Colors

| Token               | Value     | Usage                                     |
| ------------------- | --------- | ----------------------------------------- |
| Default Border      | #1A1A1A   | Card borders, panel dividers, table rules |
| Subtle Border       | #141414   | Section separators, light dividers        |
| Active Border       | #39FF14   | Focused inputs, active panel indicators   |
| Active Muted Border | #39FF1440 | Decorative neon stroke at 25% opacity     |

### Accent Colors

| Token          | Value     | Usage                                     |
| -------------- | --------- | ----------------------------------------- |
| Primary Accent | #39FF14   | Active states, highlights, status LEDs    |
| Accent Glow    | #39FF144D | Glow effects (30% opacity)                |
| Accent Subtle  | #39FF1420 | Tinted backgrounds, hover states          |
| Success        | #39FF14   | Positive metrics (same as accent)         |
| Error          | #FF3333   | Error states, alerts, destructive actions |
| Warning        | #FFB800   | Caution indicators, pending states        |

## Typography

### Font Families

| Role           | Family     | Usage                                                   |
| -------------- | ---------- | ------------------------------------------------------- |
| Display / Data | Space Mono | Metric values, headings, labels, navigation, timestamps |
| Body           | Inter      | Body text, descriptions, longer paragraphs              |

### Type Scale

| Level   | Size | Font       | Weight | Usage                                          |
| ------- | ---- | ---------- | ------ | ---------------------------------------------- |
| Display | 40px | Space Mono | 700    | Hero metric values, primary counters           |
| Title 1 | 24px | Space Mono | 700    | Section headings                               |
| Title 2 | 18px | Space Mono | 600    | Card titles, subsection headings               |
| Title 3 | 15px | Space Mono | 500    | List item titles, panel headers                |
| Body    | 14px | Inter      | 400    | Descriptions, body text, paragraphs            |
| Label   | 12px | Space Mono | 500    | Field labels, navigation items, column headers |
| Caption | 11px | Space Mono | 400    | Timestamps, metadata, footnotes                |
| Micro   | 10px | Space Mono | 500    | Status badges, auxiliary data (uppercase)      |
| Tiny    | 9px  | Space Mono | 500    | Tertiary metadata, system readouts             |

### Font Weights

| Weight   | Value | Usage                                  |
| -------- | ----- | -------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions      |
| Medium   | 500   | Labels, secondary headings, navigation |
| Semibold | 600   | Card titles, subsection headings       |
| Bold     | 700   | Display metrics, section headings      |

### Letter Spacing

- Display metrics (40px): -1px
- Uppercase labels: +2px to +3px (industrial stencil feel)
- Monospace headings: -0.5px
- Body text: 0px

### Line Height

- Display metrics (40px): 1.0
- Headings (18-24px): 1.15
- Body text (14px): 1.5
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                          |
| ----- | ---------------------------------------------- |
| 0px   | Sidebar items (padding-only separation)        |
| 4px   | Inline icon+text pairs, status LED to label    |
| 8px   | Metric indicator groups, compact card sections |
| 12px  | Between list items, button groups              |
| 16px  | Card internal sections, nav item spacing       |
| 20px  | Panel sections, form field groups              |
| 24px  | Between cards in a grid                        |
| 32px  | Major content sections                         |
| 48px  | Top-level page sections                        |

### Padding Scale

| Value    | Usage                                 |
| -------- | ------------------------------------- |
| [12, 16] | Sidebar nav items                     |
| [10, 16] | Standard buttons                      |
| [8, 12]  | Input fields, compact buttons         |
| 20px     | Card padding (compact panels)         |
| 24px     | Standard card padding, content insets |
| [32, 40] | Content area padding                  |
| [48, 40] | Content area with generous spacing    |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, vertical, space_between, icon + text nav
- Content area: fill_container, vertical, gap 28-48, padding [32, 40]
- Sidebar indicator: 3px left border in neon green on active item

## Corner Radius

| Value | Usage                                     | Rationale                          |
| ----- | ----------------------------------------- | ---------------------------------- |
| 0px   | Sidebar, dividers, table rules            | Machined precision, zero tolerance |
| 2px   | Badges, status indicators, micro elements | Barely perceptible softening       |
| 4px   | Cards, buttons, inputs, panels            | Maximum radius — still industrial  |

Design rationale: The 0-4px range enforces an industrial, fabricated aesthetic. Sharp corners suggest stamped metal, CNC-milled panels, and factory equipment interfaces. The near-zero radii keep every element feeling engineered rather than decorative, letting the neon accent color provide all visual energy.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (organic curves contrast with sharp rectangular containers)

### Commonly Used Icons

activity, terminal, cpu, hard-drive, gauge, zap, power, settings, alert-triangle, check-circle, wifi, server, database, layers, monitor, refresh-cw, shield, lock, eye, bar-chart-2, trending-up, plus, x, chevron-right

### Icon Sizes

| Size | Usage                                   |
| ---- | --------------------------------------- |
| 14px | Inline indicators, table row actions    |
| 18px | Sidebar navigation icons, list items    |
| 20px | Card header icons, panel action buttons |
| 24px | Primary sidebar icons, header actions   |

### Icon Color States

| State      | Color   | Usage                                         |
| ---------- | ------- | --------------------------------------------- |
| Active     | #39FF14 | Selected navigation, active states, status on |
| Default    | #808080 | Inactive navigation, secondary actions        |
| Muted      | #3A3A3A | Disabled icons, placeholder states            |
| On Surface | #E8E8E8 | Icons on elevated surfaces                    |
