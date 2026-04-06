---
name: 'luxury-brand-dark'
tags: [luxury, dark-mode, serif, gold-accent, elegant, premium, landing-page]
platform: webapp
---

## Style Summary

A refined, high-end brand landing page with absolute black (#0A0A0A) and gold (#C9A962) accents that evoke luxury fashion houses, premium watches, and exclusive products. Cormorant Garamond serif headlines deliver classical elegance with thin, tall letterforms, while Inter body text maintains functional legibility. Zero corner radius creates an architectural, gallery-like presentation where every element feels deliberately placed and precisely constructed.

Key aesthetics:

- **Absolute black canvas**: #0A0A0A background creates maximum contrast and drama
- **Gold accent restraint**: #C9A962 used sparingly for CTAs, dividers, and key highlights only
- **Classical serif display**: Cormorant Garamond at 300-500 weight, tall and thin for editorial elegance
- **Zero radius architecture**: All corners at 0px for a gallery, museum-like precision
- **Generous whitespace**: Oversized vertical padding between sections creates breathing room
- **Thin gold rules**: 1px gold dividers and hairline borders as primary decorative elements

## Color System

### Core Backgrounds

| Token            | Value   | Usage                             |
| ---------------- | ------- | --------------------------------- |
| Page Background  | #0A0A0A | Root page background (true black) |
| Section Alt      | #111111 | Alternating section background    |
| Card Surface     | #161616 | Product cards, feature panels     |
| Elevated Surface | #1C1C1C | Modals, dropdown menus            |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #FAFAFA | Headlines, hero copy                     |
| Secondary Text | #A3A3A3 | Body text, descriptions                  |
| Tertiary Text  | #737373 | Captions, metadata, footer text          |
| Muted Text     | #404040 | Placeholders, disabled text              |
| Gold Text      | #C9A962 | Highlighted labels, premium badges, CTAs |

### Border Colors

| Token          | Value     | Usage                                  |
| -------------- | --------- | -------------------------------------- |
| Default Border | #262626   | Card borders, structural dividers      |
| Subtle Border  | #1A1A1A   | Light separators                       |
| Gold Border    | #C9A962   | Decorative rules, active indicators    |
| Gold Hairline  | #C9A96240 | Subtle gold accents at reduced opacity |

### Accent Colors

| Token          | Value     | Usage                                         |
| -------------- | --------- | --------------------------------------------- |
| Primary Accent | #C9A962   | CTA buttons, gold highlights, premium markers |
| Accent Muted   | #C9A96220 | Subtle gold tint backgrounds                  |
| Accent Dark    | #A68B4B   | Gold hover/pressed state                      |
| On Accent      | #0A0A0A   | Text/icons on gold backgrounds (black)        |
| Error          | #B91C1C   | Error states (muted red)                      |
| Success        | #15803D   | Success states (deep green)                   |

## Typography

### Font Families

| Role              | Family             | Usage                                              |
| ----------------- | ------------------ | -------------------------------------------------- |
| Display / Serif   | Cormorant Garamond | Hero headlines, section headings, brand statements |
| Body / Functional | Inter              | Body text, navigation, buttons, descriptions       |

### Type Scale

| Level      | Size | Font               | Weight | Usage                                        |
| ---------- | ---- | ------------------ | ------ | -------------------------------------------- |
| Hero       | 80px | Cormorant Garamond | 300    | Primary hero headline (ultra-thin)           |
| Display    | 64px | Cormorant Garamond | 400    | Large section headings                       |
| Title 1    | 48px | Cormorant Garamond | 400    | Section headings                             |
| Title 2    | 32px | Cormorant Garamond | 500    | Subsection headings                          |
| Title 3    | 20px | Inter              | 400    | Card titles, feature names                   |
| Body Large | 18px | Inter              | 300    | Lead paragraphs, hero subtitle               |
| Body       | 16px | Inter              | 400    | Descriptions, copy                           |
| Label      | 13px | Inter              | 500    | Navigation (uppercase), button text          |
| Caption    | 12px | Inter              | 400    | Metadata, image captions                     |
| Overline   | 11px | Inter              | 500    | Section overlines (uppercase, gold, tracked) |

### Font Weights

| Weight  | Value | Usage                                      |
| ------- | ----- | ------------------------------------------ |
| Light   | 300   | Hero headline, lead body text              |
| Regular | 400   | Display headings, body text                |
| Medium  | 500   | Subsection headings, navigation, overlines |

### Letter Spacing

- Hero (80px): -2px
- Display (64px): -1px
- Section headings (48px): -0.5px
- Uppercase navigation: +4px (wide tracking)
- Uppercase overlines: +3px
- Body text: 0px

### Line Height

- Hero (80px): 0.95
- Display (64px): 1.0
- Headings (32-48px): 1.15
- Body (16-18px): 1.7 (generous for readability)
- Overlines (11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                       |
| ----- | ------------------------------------------- |
| 4px   | Inline icon+text pairs                      |
| 8px   | Tight element groups                        |
| 16px  | Navigation items (wide-tracked, needs room) |
| 20px  | Card content sections                       |
| 24px  | Feature list items                          |
| 32px  | Between cards in grid                       |
| 48px  | Section internal blocks                     |
| 80px  | Between page sections                       |
| 120px | Hero section to content                     |

### Padding Scale

| Value    | Usage                                  |
| -------- | -------------------------------------- |
| [12, 32] | Navigation links (generous horizontal) |
| [16, 40] | CTA buttons (wide, architectural)      |
| 32px     | Card padding (standard)                |
| 40px     | Large card padding                     |
| [120, 0] | Section vertical padding (generous)    |
| [160, 0] | Hero vertical padding                  |

### Layout Pattern

- Page width: 1440px max
- Content container: 1200px centered, padding [0, 48]
- Hero section: full-width, centered text, max-width 900px, generous vertical padding
- Feature grid: 2-column or 3-column, gap 32px
- Product showcase: full-width image + overlaid text
- Navigation: fixed top, transparent/black, height 80px, uppercase links
- Footer: minimal, centered, gold divider above
- Gold horizontal rules: 1px, used as section dividers

## Corner Radius

| Value | Usage        | Rationale                          |
| ----- | ------------ | ---------------------------------- |
| 0px   | All elements | Architectural precision throughout |

Design rationale: Zero corner radius is the signature of luxury and editorial design. Every edge is deliberately sharp, creating a gallery-like presentation that echoes the precision of fine jewelry, watchmaking, and haute couture. Rounded corners would introduce a casual, consumer-tech feel that undermines the premium positioning. The 0px radius transforms every card and button into an architectural element.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (stroke contrast with sharp corners creates intentional tension)

### Commonly Used Icons

arrow-right, arrow-up-right, chevron-right, chevron-down, menu, x, shopping-bag, heart, star, diamond, crown, award, shield, eye, play, pause, map-pin, clock, phone, mail

### Icon Sizes

| Size | Usage                              |
| ---- | ---------------------------------- |
| 16px | Inline navigation, utility actions |
| 20px | Card actions, feature indicators   |
| 24px | Header icons, mobile menu toggle   |
| 32px | Hero section decorative elements   |

### Icon Color States

| State   | Color   | Usage                               |
| ------- | ------- | ----------------------------------- |
| Gold    | #C9A962 | Premium actions, decorative accents |
| Default | #A3A3A3 | Standard icons, secondary actions   |
| Bright  | #FAFAFA | Icons on dark backgrounds           |
| Muted   | #404040 | Disabled, placeholder icons         |
| On Gold | #0A0A0A | Icons on gold backgrounds           |
