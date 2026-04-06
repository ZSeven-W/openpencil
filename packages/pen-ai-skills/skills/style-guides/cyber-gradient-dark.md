---
name: 'cyber-gradient-dark'
tags: [gradient, dark-mode, vibrant, electric, bold-typography, mesh-gradient, tech]
platform: webapp
---

## Style Summary

A bold, vibrant dark interface that channels cyberpunk energy through electric gradient accents on deep void-black surfaces. The #0A0118 background — a near-black with a hint of purple — sets the stage for violet (#8B5CF6) to cyan (#06B6D4) gradient sweeps across cards, buttons, and status indicators. Outfit headlines at weight 700 bring geometric boldness, while Inter body text keeps content readable against the high-energy palette. Generous 12-16px radii give elements a smooth, futuristic polish.

Key aesthetics:

- **Dual-accent gradients**: Violet (#8B5CF6) to cyan (#06B6D4) linear gradients on buttons, progress bars, and decorative elements
- **Deep void background**: #0A0118 (purple-tinted black) creates depth and makes gradient accents pop
- **Bold geometric headlines**: Outfit at 700 weight for impactful, chunky section titles
- **Smooth futuristic radii**: 12-16px corners give cards and containers a polished, high-tech feel
- **Glow effects**: Gradient-colored glow (blur 20, 25% opacity) behind accent elements
- **Glassmorphism hints**: Card surfaces at #13102480 (with subtle transparency for layered depth)

## Color System

### Core Backgrounds

| Token           | Value     | Usage                                    |
| --------------- | --------- | ---------------------------------------- |
| Page Background | #0A0118   | Root page background (purple-black void) |
| Card Surface    | #131024   | Cards, panels, elevated containers       |
| Sidebar Surface | #0E0B1A   | Navigation sidebar background            |
| Inset Surface   | #1A1530   | Input fields, nested containers          |
| Glass Surface   | #13102480 | Overlapping elements with transparency   |

### Text Colors

| Token              | Value   | Usage                                     |
| ------------------ | ------- | ----------------------------------------- |
| Primary Text       | #F0EEFF | Headings, metric values, primary content  |
| Secondary Text     | #A8A0C8 | Body text, descriptions, secondary labels |
| Tertiary Text      | #6B6490 | Timestamps, captions, inactive elements   |
| Muted Text         | #4A4468 | Placeholders, disabled states             |
| Accent Violet Text | #8B5CF6 | Active navigation, violet highlights      |
| Accent Cyan Text   | #06B6D4 | Secondary highlights, data emphasis       |

### Border Colors

| Token           | Value                    | Usage                              |
| --------------- | ------------------------ | ---------------------------------- |
| Default Border  | #221E38                  | Card borders, panel dividers       |
| Subtle Border   | #1A1630                  | Section separators, light dividers |
| Violet Border   | #8B5CF6                  | Focused inputs, active indicators  |
| Cyan Border     | #06B6D4                  | Secondary focus, data highlights   |
| Gradient Border | linear(#8B5CF6, #06B6D4) | Premium elements, featured cards   |

### Accent Colors

| Token          | Value     | Usage                              |
| -------------- | --------- | ---------------------------------- |
| Primary Violet | #8B5CF6   | Primary accent, active states      |
| Primary Cyan   | #06B6D4   | Secondary accent, data emphasis    |
| Gradient Start | #8B5CF6   | Violet end of gradient spectrum    |
| Gradient End   | #06B6D4   | Cyan end of gradient spectrum      |
| Violet Glow    | #8B5CF640 | Glow effects (25% opacity)         |
| Cyan Glow      | #06B6D440 | Secondary glow effects             |
| Success        | #34D399   | Positive metrics, completed states |
| Error          | #F87171   | Error states, destructive actions  |
| Warning        | #FBBF24   | Alerts, pending states             |

## Typography

### Font Families

| Role               | Family | Usage                                            |
| ------------------ | ------ | ------------------------------------------------ |
| Display / Headings | Outfit | Hero values, section headings, navigation labels |
| Body               | Inter  | Body text, descriptions, longer paragraphs       |

### Type Scale

| Level      | Size | Font   | Weight | Usage                                          |
| ---------- | ---- | ------ | ------ | ---------------------------------------------- |
| Display    | 48px | Outfit | 700    | Hero metric values, splash headlines           |
| Title 1    | 28px | Outfit | 700    | Section headings                               |
| Title 2    | 20px | Outfit | 600    | Card titles, subsection headings               |
| Title 3    | 16px | Outfit | 600    | List item titles, panel headers                |
| Body       | 14px | Inter  | 400    | Descriptions, body text                        |
| Body Large | 16px | Inter  | 400    | Prominent body text, intro paragraphs          |
| Label      | 12px | Outfit | 600    | Field labels, navigation items, column headers |
| Caption    | 11px | Inter  | 400    | Timestamps, metadata                           |
| Micro      | 10px | Outfit | 600    | Status badges, tag labels (uppercase)          |

### Font Weights

| Weight   | Value | Usage                                |
| -------- | ----- | ------------------------------------ |
| Regular  | 400   | Body text, descriptions              |
| Medium   | 500   | Secondary body, subtle emphasis      |
| Semibold | 600   | Card titles, labels, sub-headings    |
| Bold     | 700   | Display, section headings, hero text |

### Letter Spacing

- Display (48px): -2px (tight, dramatic)
- Section headings (28px): -1px
- Uppercase labels: +1.5px to +2px
- Body text: 0px

### Line Height

- Display (48px): 1.0
- Headings (20-28px): 1.15
- Body text (14-16px): 1.5
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                    |
| ----- | ---------------------------------------- |
| 4px   | Inline icon+text pairs                   |
| 8px   | Tag groups, compact metric sections      |
| 12px  | Between list items, button groups        |
| 16px  | Card internal sections, nav item spacing |
| 20px  | Panel sections, form field groups        |
| 24px  | Between cards in a grid                  |
| 32px  | Major content sections                   |
| 40px  | Top-level page section breaks            |
| 48px  | Hero section spacing                     |

### Padding Scale

| Value    | Usage                              |
| -------- | ---------------------------------- |
| [8, 16]  | Tags, compact badges               |
| [10, 20] | Standard buttons                   |
| [12, 16] | Input fields                       |
| [12, 24] | Sidebar nav items                  |
| 24px     | Card padding (standard)            |
| 28px     | Card padding (featured)            |
| [32, 40] | Content area padding               |
| [48, 40] | Content area with generous spacing |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 260px wide, vertical, space_between, icon + text nav
- Content area: fill_container, vertical, gap 32-48, padding [32, 40]
- Gradient accent bar: 4px left border with violet-to-cyan gradient on featured items
- Card grid: 2-3 columns, gap 24

## Corner Radius

| Value | Usage                                 | Rationale                        |
| ----- | ------------------------------------- | -------------------------------- |
| 0px   | Dividers, progress bar tracks         | Flat structural elements         |
| 8px   | Buttons, inputs, badges, tags         | Smooth interactive elements      |
| 12px  | Standard cards, dropdown menus        | Primary container radius         |
| 16px  | Featured cards, hero sections, modals | Large polished containers        |
| 24px  | Pill buttons, toggle switches         | Full capsule for special actions |

Design rationale: Generous radii (12-16px) create a smooth, futuristic aesthetic that complements the gradient accents. The rounded forms suggest advanced technology and polished interfaces, contrasting with the sharp edges of brutalist or industrial styles. Each radius tier creates clear visual hierarchy — larger radii signal more important or prominent containers.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

layout-dashboard, rocket, zap, sparkles, trending-up, trending-down, bar-chart-2, activity, globe, shield, lock, eye, star, heart, plus, x, chevron-right, chevron-down, search, bell, settings, users, layers, cpu, wifi, cloud

### Icon Sizes

| Size | Usage                                 |
| ---- | ------------------------------------- |
| 14px | Inline indicators, tag icons          |
| 18px | Sidebar navigation icons, list items  |
| 20px | Card header icons, action buttons     |
| 24px | Primary sidebar icons, header actions |
| 28px | Hero section feature icons            |

### Icon Color States

| State         | Color   | Usage                                  |
| ------------- | ------- | -------------------------------------- |
| Active Violet | #8B5CF6 | Selected navigation, primary active    |
| Active Cyan   | #06B6D4 | Data-related active states             |
| Default       | #A8A0C8 | Inactive navigation, secondary actions |
| Muted         | #4A4468 | Disabled icons, placeholder states     |
| On Surface    | #F0EEFF | Icons on gradient or dark surfaces     |
