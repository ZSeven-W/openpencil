---
name: 'noir-elegant-dark'
tags: [elegant, dark-mode, serif, luxury, gold-accent, sophisticated, refined]
platform: webapp
---

## Style Summary

A cinematic dark interface inspired by film noir aesthetics and old-world luxury. Deep charcoal (#121212) surfaces serve as the backdrop for restrained gold (#D4AF37) accents that reference gilded typography, brass fixtures, and vintage cinema titles. Cormorant headlines bring classical serif elegance with dramatic contrast between thick and thin strokes, while Lato body text provides clean, modern readability. Sharp corners (0-4px) reinforce the architectural precision of art deco and golden age design.

Key aesthetics:

- **Film noir palette**: Deep charcoal (#121212) with warm gold (#D4AF37) accents evoking vintage cinema and brass details
- **Classical serif display**: Cormorant for headings and metric values — high contrast, elegant letterforms
- **Gold restraint**: Accent used sparingly at varying opacities (100%, 40%, 20%) to create depth without garishness
- **Art deco precision**: 0-4px radii and thin gold strokes suggest architectural ornament and period typography
- **Warm neutrals**: Text colors lean slightly warm (#F5F0E8, #B8B0A0) rather than pure gray, complementing the gold
- **Decorative dividers**: Thin gold rules and 1px strokes create elegant separation between content zones

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                 |
| --------------- | ------- | ------------------------------------- |
| Page Background | #121212 | Root page background                  |
| Card Surface    | #181818 | Cards, panels, elevated containers    |
| Sidebar Surface | #121212 | Navigation sidebar background         |
| Inset Surface   | #1E1E1E | Input fields, nested containers       |
| Overlay Surface | #0E0E0E | Modal backdrops, dropdown backgrounds |

### Text Colors

| Token          | Value   | Usage                                                    |
| -------------- | ------- | -------------------------------------------------------- |
| Primary Text   | #F5F0E8 | Headings, metric values, primary content (warm white)    |
| Secondary Text | #B8B0A0 | Body text, descriptions, secondary labels (warm gray)    |
| Tertiary Text  | #7A7468 | Timestamps, captions, inactive elements                  |
| Muted Text     | #4A4640 | Placeholders, disabled states                            |
| Accent Text    | #D4AF37 | Active navigation, status labels, links, gold highlights |

### Border Colors

| Token               | Value     | Usage                                               |
| ------------------- | --------- | --------------------------------------------------- |
| Default Border      | #2A2620   | Card borders, dividers (warm-tinted)                |
| Subtle Border       | #1E1C18   | Section separators, light dividers                  |
| Accent Border       | #D4AF37   | Active indicators, focused inputs, decorative rules |
| Accent Muted Border | #D4AF3740 | Decorative gold stroke at 25% opacity               |
| Accent Light Border | #D4AF3720 | Subtle hover highlights                             |

### Accent Colors

| Token          | Value     | Usage                                              |
| -------------- | --------- | -------------------------------------------------- |
| Primary Accent | #D4AF37   | Active states, gold highlights, primary accent     |
| Accent Glow    | #D4AF3766 | Warm glow effects (40% opacity)                    |
| Accent Subtle  | #D4AF3720 | Tinted backgrounds, hover states                   |
| Success        | #7BAE6E   | Positive metrics (muted green, period-appropriate) |
| Error          | #C4534A   | Error states (muted red, matches warm palette)     |
| Warning        | #D4AF37   | Warnings use gold (natural caution color)          |

## Typography

### Font Families

| Role                | Family    | Usage                                               |
| ------------------- | --------- | --------------------------------------------------- |
| Display / Headlines | Cormorant | Hero values, section headings, feature titles       |
| Body / Functional   | Lato      | Body text, descriptions, navigation labels, buttons |

### Type Scale

| Level      | Size | Font      | Weight | Usage                                          |
| ---------- | ---- | --------- | ------ | ---------------------------------------------- |
| Display    | 48px | Cormorant | 600    | Hero metric values, cinematic headlines        |
| Title 1    | 28px | Cormorant | 600    | Section headings                               |
| Title 2    | 20px | Cormorant | 600    | Card titles, subsection headings               |
| Title 3    | 16px | Lato      | 600    | List item titles, panel headers                |
| Body       | 14px | Lato      | 400    | Descriptions, body text                        |
| Body Large | 16px | Lato      | 400    | Intro paragraphs, prominent descriptions       |
| Label      | 12px | Lato      | 500    | Field labels, navigation items, column headers |
| Caption    | 11px | Lato      | 400    | Timestamps, metadata                           |
| Micro      | 10px | Lato      | 600    | Status badges, auxiliary labels (uppercase)    |

### Font Weights

| Weight   | Value | Usage                                   |
| -------- | ----- | --------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions       |
| Medium   | 500   | Labels, navigation items                |
| Semibold | 600   | All headings (Cormorant), card titles   |
| Bold     | 700   | Primary emphasis, rare — used sparingly |

### Letter Spacing

- Display (48px): -1.5px (tight, dramatic serif)
- Section headings (28px): -1px
- Uppercase labels: +1.5px to +2px (art deco stencil feel)
- Body text: 0px

### Line Height

- Display (48px): 1.0
- Headings (20-28px): 1.15
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
| 20px  | Panel sections, form field groups         |
| 24px  | Between cards in a grid                   |
| 28px  | Content area major sections               |
| 32px  | Top-level page sections                   |
| 48px  | Large section breaks, hero spacing        |

### Padding Scale

| Value    | Usage                   |
| -------- | ----------------------- |
| [12, 16] | Sidebar nav items       |
| [10, 20] | Standard buttons        |
| [8, 16]  | Input fields            |
| 24px     | Card padding (standard) |
| 28px     | Feature card padding    |
| [32, 48] | Content area padding    |
| [48, 48] | Hero section padding    |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 260px wide, vertical, space_between, icon + text nav
- Content area: fill_container, vertical, gap 28-48, padding [32, 48]
- Decorative gold rule: 1px horizontal divider in #D4AF37 between major sections
- Sidebar indicator: 3px left border in gold on active item

## Corner Radius

| Value | Usage                               | Rationale                           |
| ----- | ----------------------------------- | ----------------------------------- |
| 0px   | Dividers, sidebar, decorative rules | Classical precision                 |
| 2px   | Badges, micro indicators            | Barely perceptible softening        |
| 4px   | Cards, buttons, inputs, panels      | Maximum radius — art deco restraint |

Design rationale: The 0-4px range maintains the architectural precision of art deco and film noir aesthetics. Sharp corners suggest engraved metal plates, gilded frames, and typeset letterpress blocks. The near-zero radii let the Cormorant serif typography and gold accents carry all decorative weight, keeping the structural elements severe and elegant.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

layout-dashboard, bar-chart-2, users, settings, crown, award, star, gem, briefcase, calendar, clock, trending-up, trending-down, search, bell, chevron-right, chevron-down, plus, x, edit, eye, lock, heart, bookmark, filter, download

### Icon Sizes

| Size | Usage                                 |
| ---- | ------------------------------------- |
| 14px | Inline indicators, table row actions  |
| 18px | Sidebar navigation icons, list items  |
| 20px | Card header icons, action buttons     |
| 24px | Primary sidebar icons, header actions |

### Icon Color States

| State      | Color   | Usage                                  |
| ---------- | ------- | -------------------------------------- |
| Active     | #D4AF37 | Selected navigation, gold highlights   |
| Default    | #B8B0A0 | Inactive navigation, secondary actions |
| Muted      | #4A4640 | Disabled icons, placeholder states     |
| On Surface | #F5F0E8 | Icons on elevated surfaces             |
