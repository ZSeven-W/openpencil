---
name: 'gaming-electric-dark'
tags: [electric, dark-mode, neon, bold-typography, vibrant, gradient, webapp]
platform: webapp
---

## Style Summary

A high-voltage dark interface channeling the kinetic energy of esports arenas, gaming dashboards, and competitive platforms. The pure-black #0A0A0A background creates a void where electric red (#EF4444) and hot orange (#F97316) accents burn with maximum intensity. Rajdhani delivers sharp, angular headings at weight 700 — its condensed geometric letterforms suggest speed and precision — while Inter handles body text with neutral efficiency, letting the display type and color accents carry the visual charge. The 8-12px radii create sleek, aerodynamic surfaces. Gradient overlays on primary CTAs and featured panels inject motion energy into the interface.

Key aesthetics:

- **Pure-black void**: #0A0A0A background maximizes the intensity of neon accents
- **Red-to-orange energy**: #EF4444 to #F97316 gradient creates electric heat signatures
- **Angular display type**: Rajdhani's condensed, geometric letterforms suggest speed and competition
- **Gradient CTAs**: Red-to-orange linear gradients on primary buttons radiate kinetic energy
- **Sleek surfaces**: 8-12px radii create aerodynamic card shapes
- **High-contrast hierarchy**: White text on black with neon accents produces razor-sharp readability

## Color System

### Core Backgrounds

| Token            | Value     | Usage                              |
| ---------------- | --------- | ---------------------------------- |
| Page Background  | #0A0A0A   | Root page background (near-black)  |
| Card Surface     | #171717   | Cards, panels, elevated containers |
| Inset Surface    | #1C1C1C   | Input fields, nested containers    |
| Elevated Surface | #222222   | Modal overlays, dropdown menus     |
| Glow Surface     | #EF444410 | Red-tinted featured sections       |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #FAFAFA | Headings, metric values, primary content  |
| Secondary Text | #A3A3A3 | Body text, descriptions, secondary labels |
| Tertiary Text  | #737373 | Timestamps, captions, inactive elements   |
| Muted Text     | #525252 | Placeholders, disabled states             |
| Accent Text    | #EF4444 | Active states, live indicators, hot data  |
| Energy Text    | #F97316 | Secondary highlights, trending indicators |

### Border Colors

| Token          | Value     | Usage                              |
| -------------- | --------- | ---------------------------------- |
| Default Border | #262626   | Card borders, dividers             |
| Subtle Border  | #1C1C1C   | Section separators, light dividers |
| Active Border  | #EF4444   | Focused inputs, active selection   |
| Glow Border    | #EF444450 | Neon glow effect borders           |

### Accent Colors

| Token            | Value     | Usage                                       |
| ---------------- | --------- | ------------------------------------------- |
| Primary Accent   | #EF4444   | Primary buttons, active states (red 500)    |
| Accent Hover     | #DC2626   | Button hover, pressed states (red 600)      |
| Secondary Accent | #F97316   | Secondary highlights, trending (orange 500) |
| Gradient Start   | #EF4444   | Gradient button start (red)                 |
| Gradient End     | #F97316   | Gradient button end (orange)                |
| Accent Subtle    | #EF444420 | Hover backgrounds, selected rows            |
| Accent Glow      | #EF444430 | Active indicator glow effects               |
| Success          | #22C55E   | Win states, positive metrics                |
| Warning          | #FBBF24   | Caution indicators                          |
| Error            | #F43F5E   | Critical errors, elimination states         |
| Info             | #38BDF8   | Informational elements, cooldown states     |

## Typography

### Font Families

| Role              | Family   | Usage                                                       |
| ----------------- | -------- | ----------------------------------------------------------- |
| Headings          | Rajdhani | Section headings, display metrics, card titles (weight 700) |
| Body / Functional | Inter    | Body text, labels, buttons, navigation (weight 400)         |

### Type Scale

| Level    | Size | Font     | Weight | Usage                                   |
| -------- | ---- | -------- | ------ | --------------------------------------- |
| Hero     | 56px | Rajdhani | 700    | Hero headlines, tournament titles       |
| Display  | 40px | Rajdhani | 700    | Page titles, feature headings           |
| Metric   | 32px | Rajdhani | 700    | Primary metric values, scores, stats    |
| Title 1  | 24px | Rajdhani | 700    | Section headings                        |
| Title 2  | 18px | Rajdhani | 600    | Card titles, subsection headings        |
| Title 3  | 15px | Rajdhani | 600    | List headings, compact card titles      |
| Body     | 14px | Inter    | 400    | Descriptions, body text, table cells    |
| Label    | 12px | Inter    | 500    | Field labels, column headers, nav items |
| Caption  | 11px | Inter    | 400    | Timestamps, secondary metadata          |
| Overline | 11px | Rajdhani | 700    | Section overlines (uppercase)           |
| Micro    | 10px | Inter    | 600    | Badges, live indicators, status labels  |

### Font Weights

| Weight   | Value | Usage                                     |
| -------- | ----- | ----------------------------------------- |
| Regular  | 400   | Body text, descriptions (Inter)           |
| Medium   | 500   | Labels, navigation items (Inter)          |
| Semibold | 600   | Sub-headings, card titles (both families) |
| Bold     | 700   | Display headlines, metrics (Rajdhani)     |

### Letter Spacing

- Hero (56px): -2px (tight, aggressive)
- Display (40px): -1.5px
- Section headings (18-24px): -0.5px
- Overline / uppercase labels: +2px (wide, technical)
- Body text: 0px
- Metric values: -1px

### Line Height

- Hero (56px): 0.9 (ultra-tight for impact)
- Display (40px): 1.0
- Headings (18-24px): 1.15
- Body (14px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                      |
| ----- | ------------------------------------------ |
| 4px   | Inline icon+text pairs, stat label spacing |
| 8px   | Compact element groups, score components   |
| 12px  | Between list items, nav items              |
| 16px  | Card internal sections, form fields        |
| 20px  | Between cards in a grid, panel sections    |
| 24px  | Content section gaps                       |
| 32px  | Major section breaks                       |
| 48px  | Top-level page sections                    |

### Padding Scale

| Value    | Usage                              |
| -------- | ---------------------------------- |
| [6, 14]  | Compact badges, status chips       |
| [10, 18] | Standard buttons, input fields     |
| [12, 24] | Large buttons, primary CTAs        |
| [14, 20] | Sidebar nav items                  |
| 16px     | Card padding (compact, stat cards) |
| 20px     | Card padding (standard)            |
| 24px     | Feature card padding               |
| [24, 32] | Content area padding               |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, vertical, near-black (#0A0A0A) with red active indicators and glow effects
- Content area: fill_container, vertical, gap 20-24, padding [24, 32]
- Cards use 1px stroke (#262626) with no shadow (flat dark surfaces, glow accents instead)
- Header bar: 60px height, #0A0A0A background, bottom border #262626
- Featured panels may use gradient borders (red-to-orange) or subtle glow (#EF444410 background)
- Stat grids: 4-column, gap 16, compact metric cards

## Corner Radius

| Value | Usage                                       | Rationale                                 |
| ----- | ------------------------------------------- | ----------------------------------------- |
| 4px   | Badges, status chips, inline indicators     | Sharp, compact data markers               |
| 6px   | Input fields, dropdowns, tooltips           | Tight interactive radius                  |
| 8px   | Buttons, standard cards, modals             | Primary interactive radius                |
| 12px  | Featured cards, large panels, hero sections | Sleek container emphasis                  |
| 100px | Avatar circles, rank badges                 | Full circle for profile and rank elements |

Design rationale: The 8-12px range creates sleek, aerodynamic surfaces that suggest speed without becoming soft or friendly. The slightly rounded corners evoke racing car panels and gaming hardware — engineered curves rather than organic shapes. Combined with the pure-black background and electric accent colors, these radii produce an interface that feels like a high-performance cockpit rather than a casual consumer app.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

trophy, target, flame, zap, swords, shield, crown, star, trending-up, trending-down, activity, timer, play, pause, volume-2, users, settings, search, bell, chevron-right, chevron-down, plus, x, check, eye, bar-chart, medal, gamepad-2, signal, wifi

### Icon Sizes

| Size | Usage                                    |
| ---- | ---------------------------------------- |
| 14px | Inline stat indicators, table actions    |
| 16px | Sidebar navigation, button leading icons |
| 20px | Card header icons, toolbar actions       |
| 24px | Primary sidebar icons, header actions    |
| 32px | Feature section accent icons, hero icons |

### Icon Color States

| State      | Color   | Usage                              |
| ---------- | ------- | ---------------------------------- |
| Active     | #EF4444 | Active navigation, live indicators |
| Energy     | #F97316 | Trending states, secondary active  |
| Default    | #A3A3A3 | Standard icons, secondary actions  |
| Muted      | #525252 | Disabled icons, placeholder states |
| On Surface | #FAFAFA | Icons on elevated surfaces         |
| On Accent  | #FFFFFF | Icons on red/gradient backgrounds  |
