---
name: 'retro-warm-light'
tags: [warm-tones, light-mode, earth-tones, organic, cozy, rounded, webapp, friendly]
platform: webapp
---

## Style Summary

A warm, handcrafted interface evoking the amber glow of vintage paper, aged leather, and artisan workshops. The parchment-cream #FDF6EC background wraps content in a cozy warmth that feels analog and personal, while the amber-brown accent (#B45309) grounds interactions with earthy confidence. Libre Baskerville brings old-style serif character to headings — its bracketed serifs and generous x-height recall letterpress craftsmanship — while Source Sans 3 delivers crisp, modern readability for body text. Generous 12-16px radii create soft, pillow-like containers that invite touch and exploration.

Key aesthetics:

- **Parchment warmth**: #FDF6EC background creates a paper-like canvas with golden undertones
- **Amber-brown accent**: #B45309 provides earthy, natural interactive anchors
- **Serif-sans pairing**: Libre Baskerville headings + Source Sans 3 body balances heritage with clarity
- **Generous radii**: 12-16px corners create soft, organic container shapes
- **Earth-tone palette**: Warm grays and browns replace cold neutrals throughout
- **Handcrafted details**: Thicker borders and subtle textures suggest artisan quality

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                     |
| --------------- | ------- | ----------------------------------------- |
| Page Background | #FDF6EC | Root page background (warm parchment)     |
| Card Surface    | #FFFDF8 | Cards, elevated containers (warm white)   |
| Inset Surface   | #F5EDE0 | Input backgrounds, recessed areas         |
| Warm Highlight  | #FEF3C7 | Highlighted sections, callout backgrounds |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #292524 | Headings, primary labels (warm black)    |
| Secondary Text | #57534E | Body text, descriptions                  |
| Tertiary Text  | #A8A29E | Captions, timestamps, helper text        |
| Muted Text     | #D6D3D1 | Placeholders, disabled text              |
| Accent Text    | #B45309 | Active nav items, links, emphasized data |

### Border Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Default Border | #E7E0D6 | Card borders, dividers, input outlines   |
| Subtle Border  | #F0EAE0 | Light separators, table row dividers     |
| Active Border  | #B45309 | Focused inputs, active tab indicators    |
| Warm Border    | #D4A574 | Decorative borders, feature card accents |

### Accent Colors

| Token          | Value     | Usage                                        |
| -------------- | --------- | -------------------------------------------- |
| Primary Accent | #B45309   | Primary buttons, active states (amber brown) |
| Accent Hover   | #92400E   | Button hover, pressed states (darker amber)  |
| Accent Light   | #B4530915 | Tinted backgrounds, selected rows            |
| Accent Muted   | #B4530935 | Badge fills, progress tracks                 |
| Warm Secondary | #D97706   | Secondary highlights, amber                  |
| Success        | #15803D   | Positive metrics, completion states          |
| Warning        | #CA8A04   | Alert indicators, pending states             |
| Error          | #B91C1C   | Error states, destructive actions            |

## Typography

### Font Families

| Role              | Family            | Usage                                                    |
| ----------------- | ----------------- | -------------------------------------------------------- |
| Headings          | Libre Baskerville | Section headings, card titles, display text (weight 700) |
| Body / Functional | Source Sans 3     | Body text, labels, buttons, navigation (weight 400)      |

### Type Scale

| Level      | Size | Font              | Weight | Usage                                   |
| ---------- | ---- | ----------------- | ------ | --------------------------------------- |
| Display    | 40px | Libre Baskerville | 700    | Page titles, hero headings              |
| Metric     | 32px | Libre Baskerville | 700    | Primary metric values, KPI numbers      |
| Title 1    | 24px | Libre Baskerville | 700    | Section headings                        |
| Title 2    | 18px | Libre Baskerville | 700    | Card titles, subsection headings        |
| Title 3    | 15px | Libre Baskerville | 400    | List headings, compact card titles      |
| Body Large | 16px | Source Sans 3     | 400    | Lead paragraphs, featured descriptions  |
| Body       | 14px | Source Sans 3     | 400    | Standard body text, table cells         |
| Label      | 12px | Source Sans 3     | 600    | Field labels, column headers, nav items |
| Caption    | 11px | Source Sans 3     | 400    | Timestamps, secondary metadata          |
| Overline   | 11px | Source Sans 3     | 600    | Section overlines (uppercase)           |
| Micro      | 10px | Source Sans 3     | 600    | Badges, status labels                   |

### Font Weights

| Weight   | Value | Usage                                       |
| -------- | ----- | ------------------------------------------- |
| Regular  | 400   | Body text, descriptions, serif subheadings  |
| Semibold | 600   | Labels, buttons, navigation items           |
| Bold     | 700   | Serif headings, display text, metric values |

### Letter Spacing

- Display (40px): -1px
- Section headings (18-24px): -0.5px
- Overline / uppercase labels: +1.5px
- Body text: 0px
- Serif headings: 0px (serifs provide natural rhythm)

### Line Height

- Display (40px): 1.1
- Headings (18-24px): 1.25
- Body (14-16px): 1.6 (generous for readability)
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                               |
| ----- | ----------------------------------- |
| 4px   | Inline icon+text pairs              |
| 8px   | Compact element groups, tag spacing |
| 12px  | Between list items, nav items       |
| 16px  | Card internal sections, form fields |
| 20px  | Between cards in a grid             |
| 24px  | Content section gaps                |
| 32px  | Major section breaks                |
| 48px  | Top-level page sections             |

### Padding Scale

| Value    | Usage                          |
| -------- | ------------------------------ |
| [8, 16]  | Compact buttons, badges        |
| [10, 20] | Standard buttons, input fields |
| [12, 24] | Large buttons, search bars     |
| [14, 18] | Sidebar nav items              |
| 20px     | Card padding (compact)         |
| 24px     | Card padding (standard)        |
| 32px     | Feature card padding           |
| [32, 36] | Content area padding           |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, vertical, warm-white (#FFFDF8) with amber-brown active indicators
- Content area: fill_container, vertical, gap 24-32, padding [32, 36]
- Cards use 1px stroke (#E7E0D6) with warm shadow (0 2px 8px rgba(120,80,30,0.06))
- Header bar: 64px height, warm-white background, bottom border #E7E0D6
- Feature sections may use #FEF3C7 warm highlight backgrounds

## Corner Radius

| Value | Usage                                          | Rationale                                |
| ----- | ---------------------------------------------- | ---------------------------------------- |
| 6px   | Badges, inline tags, tooltips                  | Gentle softening for small elements      |
| 10px  | Input fields, dropdowns                        | Softened interactive radius              |
| 12px  | Buttons, standard cards, modals                | Primary interactive and container radius |
| 16px  | Featured cards, large panels, image containers | Generous organic softness                |
| 24px  | Pill buttons, toggle switches                  | Full capsule shape for special CTAs      |

Design rationale: The 12-16px range creates surfaces that feel handmade and organic — like rounded pottery or smoothed wood. These generous radii reject the sharp, clinical precision of corporate UI in favor of warmth and approachability. Combined with the parchment background and earth tones, the soft corners suggest an artisan workshop where every element has been shaped by hand rather than machine.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (complements the warm, organic aesthetic)

### Commonly Used Icons

home, book-open, bookmark, coffee, pen-line, image, heart, star, map-pin, compass, sun, leaf, gift, shopping-bag, archive, folder, file-text, search, bell, user, settings, plus, x, check, edit, calendar, clock, tag, scissors, feather

### Icon Sizes

| Size | Usage                                          |
| ---- | ---------------------------------------------- |
| 14px | Inline indicators, table row actions           |
| 16px | Sidebar navigation icons, button leading icons |
| 20px | Card header action icons, toolbar actions      |
| 24px | Primary sidebar icons, page header actions     |

### Icon Color States

| State     | Color   | Usage                                   |
| --------- | ------- | --------------------------------------- |
| Active    | #B45309 | Active navigation, selected states      |
| Default   | #57534E | Standard icons, secondary actions       |
| Muted     | #D6D3D1 | Disabled icons, placeholder states      |
| On Accent | #FFFFFF | Icons on amber-brown accent backgrounds |
| Warm      | #D97706 | Special highlight icons, amber accent   |
