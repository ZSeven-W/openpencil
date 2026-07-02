---
name: 'terminal-minimal-dark'
tags: [terminal, dark-mode, monospace, minimal, cyan-accent, tech, developer, crisp]
platform: mobile
---

## Style Summary

A developer-focused dark interface inspired by terminal aesthetics and command-line tooling. The design pairs a deep navy-black background with sharp cyan accents to evoke a modern IDE or system monitor. Every element feels precise and data-driven, with monospace typography lending authenticity and a sense of technical competence.

Key aesthetics:

- **Terminal palette**: Deep navy (#0A0F1C) with cyan (#22D3EE) highlights that reference terminal cursor and selection colors
- **Monospace-first**: JetBrains Mono as the primary display and data font, giving all numbers and labels a code-native feel
- **Compact density**: Tight spacing and small radii (3-12px) keep information dense without clutter
- **Glow indicators**: Status dots and active states use subtle cyan glow effects (blur 8, 40% opacity)
- **Minimal chrome**: Borders are thin (#1E293B) and surfaces barely separate from the background
- **Pill tab bar**: Bottom navigation uses a 100px radius pill shape with icon+label layout

## Color System

### Core Backgrounds

| Token           | Value   | Usage                             |
| --------------- | ------- | --------------------------------- |
| Page Background | #0A0F1C | Root screen background            |
| Card Surface    | #111827 | Cards, input fields, list items   |
| Inset Surface   | #0F172A | Nested containers, code blocks    |
| Tab Bar Surface | #111827 | Bottom navigation pill background |

### Text Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Primary Text   | #F1F5F9 | Headings, metric values, active labels |
| Secondary Text | #94A3B8 | Body text, descriptions                |
| Tertiary Text  | #64748B | Timestamps, captions, inactive labels  |
| Muted Text     | #475569 | Placeholders, disabled states          |
| Accent Text    | #22D3EE | Active nav items, links, status labels |

### Border Colors

| Token          | Value   | Usage                                |
| -------------- | ------- | ------------------------------------ |
| Default Border | #1E293B | Card borders, dividers               |
| Subtle Border  | #162032 | Section separators                   |
| Active Border  | #22D3EE | Focused inputs, active tab indicator |

### Accent Colors

| Token          | Value     | Usage                                 |
| -------------- | --------- | ------------------------------------- |
| Primary Accent | #22D3EE   | Active states, highlights, links      |
| Accent Muted   | #22D3EE40 | Glow effects, badge backgrounds       |
| Success        | #22C55E   | Positive metrics, completed states    |
| Error          | #EF4444   | Error indicators, destructive actions |
| Warning        | #F59E0B   | Alerts, pending states                |

## Typography

### Font Families

| Role           | Family         | Usage                                                  |
| -------------- | -------------- | ------------------------------------------------------ |
| Display / Data | JetBrains Mono | Metric values, headings, timestamps, navigation labels |
| Body           | Inter          | Descriptions, body paragraphs, secondary labels        |

### Type Scale

| Level   | Size | Font           | Weight | Usage                                 |
| ------- | ---- | -------------- | ------ | ------------------------------------- |
| Display | 36px | JetBrains Mono | 700    | Hero metric values                    |
| Title 1 | 24px | JetBrains Mono | 600    | Section headings                      |
| Title 2 | 17px | JetBrains Mono | 600    | Card titles, subsection headings      |
| Title 3 | 15px | JetBrains Mono | 500    | List item titles                      |
| Body    | 14px | Inter          | 400    | Descriptions, body text               |
| Label   | 12px | JetBrains Mono | 500    | Navigation labels, field labels       |
| Caption | 11px | JetBrains Mono | 400    | Timestamps, metadata                  |
| Micro   | 10px | JetBrains Mono | 500    | Status badges, tab labels (uppercase) |
| Tiny    | 9px  | JetBrains Mono | 500    | Auxiliary data, footnotes             |

### Font Weights

| Weight   | Value | Usage                                  |
| -------- | ----- | -------------------------------------- |
| Regular  | 400   | Body text, descriptions                |
| Medium   | 500   | Labels, secondary headings, navigation |
| Semibold | 600   | Section titles, card headings          |
| Bold     | 700   | Hero metrics, primary headings         |

### Letter Spacing

- Uppercase labels: +1px to +2px
- Display metrics: -1px
- Default text: 0px
- Tab bar labels: +1px

### Line Height

- Display metrics (36px): 1.0
- Headings (17-24px): 1.2
- Body text (14px): 1.5
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                                 |
| ----- | ----------------------------------------------------- |
| 2px   | Inline icon+text micro pairs                          |
| 4px   | Tab icon to label, tight element groups               |
| 6px   | Status dot to label                                   |
| 8px   | Metric change indicator groups, compact card sections |
| 12px  | Between list items, button groups                     |
| 16px  | Card internal sections, form field groups             |
| 20px  | Major card sections (header, value, change)           |
| 24px  | Between cards in a grid                               |
| 32px  | Screen section gaps                                   |

### Padding Scale

| Value            | Usage                                    |
| ---------------- | ---------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only) |
| [12, 21, 21, 21] | Tab bar section outer padding            |
| 4px              | Tab bar pill inner padding               |
| [8, 12]          | Input fields, small buttons              |
| [10, 16]         | Standard buttons                         |
| 16px             | Card padding (compact)                   |
| 20px             | Card padding (standard)                  |
| 24px             | Large card padding, section padding      |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24-32
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned

## Corner Radius

| Value | Usage                           | Rationale                    |
| ----- | ------------------------------- | ---------------------------- |
| 0px   | Dividers, full-width separators | Flat technical precision     |
| 3px   | Badges, micro indicators        | Barely perceptible softening |
| 6px   | Input fields, small buttons     | Compact technical feel       |
| 8px   | Standard cards, list items      | Primary container radius     |
| 12px  | Large cards, modal sheets       | Comfortable but compact      |
| 100px | Tab bar pill, toggle pills      | Full capsule shape           |

Design rationale: Compact radii (3-12px) maintain a crisp, technical aesthetic. The sharp corners suggest precision engineering, while the slight rounding prevents a harsh brutalist feel. The 100px pill for the tab bar provides a strong contrast and clear affordance for the primary navigation element.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

home, search, bell, settings, user, calendar, check-circle, trending-up, trending-down, clock, activity, target, plus, x, chevron-right, chevron-down, bar-chart-2, zap, terminal, code

### Icon Sizes

| Size | Usage                                      |
| ---- | ------------------------------------------ |
| 14px | Inline with text, metric change indicators |
| 18px | Tab bar icons, list item leading icons     |
| 20px | Card header icons, action buttons          |
| 24px | Primary navigation, header actions         |

### Icon Color States

| State         | Color   | Usage                            |
| ------------- | ------- | -------------------------------- |
| Active        | #22D3EE | Selected tab, active navigation  |
| Default       | #94A3B8 | Inactive tabs, secondary actions |
| Muted         | #475569 | Disabled, placeholder icons      |
| On Background | #F1F5F9 | Icons on accent-colored surfaces |
