---
name: 'ecommerce-modern-light'
tags: [clean, light-mode, modern, rounded, warm-tones, landing-page, friendly]
platform: webapp
---

## Style Summary

A warm, inviting e-commerce product page built on a clean white (#FFFFFF) foundation with vibrant orange (#F97316) as the primary accent. This style captures the energy and approachability of modern direct-to-consumer brands — friendly, confident, and conversion-optimized. Plus Jakarta Sans serves as both heading and body font, with its rounded letterforms creating a cohesive warmth that complements the generous 16px card radius. The warm tonal palette and inviting CTAs encourage browsing and purchasing without feeling pushy.

Key aesthetics:

- **Clean white base**: #FFFFFF background with #FFF7ED warm-tinted sections for visual rhythm
- **Vibrant orange accent**: #F97316 used for CTAs, price highlights, and active states — energetic without aggressive
- **Unified warm type**: Plus Jakarta Sans throughout, differentiated by weight (700 headings, 400 body)
- **Generous radii**: 16px cards, 12px buttons create a friendly, approachable container language
- **Product-first layout**: Large image areas, clear pricing hierarchy, prominent add-to-cart patterns
- **Warm neutrals**: Gray-brown undertones in text colors reinforce the warm palette identity

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                           |
| --------------- | ------- | ----------------------------------------------- |
| Page Background | #FFFFFF | Root page background (pure white)               |
| Section Alt     | #FFF7ED | Alternating section background (warm cream)     |
| Card Surface    | #FFFFFF | Product cards, feature cards on tinted sections |
| Inset Surface   | #FEF3C7 | Promotional banners, highlight blocks           |

### Text Colors

| Token          | Value   | Usage                                       |
| -------------- | ------- | ------------------------------------------- |
| Primary Text   | #1C1917 | Headlines, product names, primary content   |
| Secondary Text | #57534E | Body text, descriptions, specifications     |
| Tertiary Text  | #A8A29E | Captions, metadata, review counts           |
| Muted Text     | #D6D3D1 | Placeholders, disabled text                 |
| Accent Text    | #EA580C | Price highlights, sale badges, active links |

### Border Colors

| Token          | Value   | Usage                             |
| -------------- | ------- | --------------------------------- |
| Default Border | #E7E5E4 | Card borders, dividers            |
| Subtle Border  | #F5F5F4 | Light separators, image borders   |
| Active Border  | #F97316 | Focused inputs, selected variants |

### Accent Colors

| Token          | Value     | Usage                                        |
| -------------- | --------- | -------------------------------------------- |
| Primary Accent | #F97316   | CTA buttons, active states, price highlights |
| Accent Light   | #FFF7ED   | Tinted backgrounds, promotional badges       |
| Accent Muted   | #F9731620 | Subtle hover backgrounds                     |
| Accent Dark    | #EA580C   | Button hover/pressed state                   |
| Success        | #16A34A   | In-stock indicators, completed steps         |
| Error          | #DC2626   | Out-of-stock, error states                   |
| Warning        | #EAB308   | Low stock warnings                           |

## Typography

### Font Families

| Role              | Family            | Usage                                  |
| ----------------- | ----------------- | -------------------------------------- |
| Headings          | Plus Jakarta Sans | All headings, hero text, product names |
| Body / Functional | Plus Jakarta Sans | Body text, navigation, buttons, labels |

### Type Scale

| Level      | Size | Font              | Weight | Usage                                 |
| ---------- | ---- | ----------------- | ------ | ------------------------------------- |
| Hero       | 56px | Plus Jakarta Sans | 700    | Primary hero headline                 |
| Display    | 44px | Plus Jakarta Sans | 700    | Category section headings             |
| Title 1    | 36px | Plus Jakarta Sans | 700    | Section headings                      |
| Title 2    | 24px | Plus Jakarta Sans | 600    | Subsection headings, feature titles   |
| Title 3    | 20px | Plus Jakarta Sans | 600    | Product card titles, list headings    |
| Body Large | 18px | Plus Jakarta Sans | 400    | Hero subtitle, lead paragraphs        |
| Body       | 16px | Plus Jakarta Sans | 400    | Descriptions, product details         |
| Label      | 14px | Plus Jakarta Sans | 500    | Navigation, button text, field labels |
| Caption    | 12px | Plus Jakarta Sans | 500    | Overline labels (uppercase), badges   |
| Micro      | 11px | Plus Jakarta Sans | 400    | Fine print, footer secondary          |

### Font Weights

| Weight   | Value | Usage                                   |
| -------- | ----- | --------------------------------------- |
| Regular  | 400   | Body text, descriptions                 |
| Medium   | 500   | Labels, navigation, buttons, captions   |
| Semibold | 600   | Subsection headings, card titles        |
| Bold     | 700   | Hero headline, section headings, prices |

### Letter Spacing

- Hero (56px): -2px
- Display (44px): -1.5px
- Section headings (36px): -1px
- Subsection (24px): -0.5px
- Uppercase overlines: +2px
- Body text: 0px

### Line Height

- Hero (56px): 1.05
- Display (44px): 1.1
- Headings (24-36px): 1.2
- Body (16-18px): 1.6
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                   |
| ----- | --------------------------------------- |
| 4px   | Inline icon+text pairs, star ratings    |
| 8px   | Badge groups, variant selectors         |
| 12px  | Button groups, nav items, breadcrumbs   |
| 16px  | Card content sections, spec list items  |
| 20px  | Between feature list items, reviews     |
| 24px  | Between product cards in grid           |
| 32px  | Section internal content gaps           |
| 48px  | Between content blocks within a section |
| 64px  | Between page sections                   |
| 80px  | Major section breaks                    |

### Padding Scale

| Value    | Usage                          |
| -------- | ------------------------------ |
| [8, 16]  | Compact buttons, variant pills |
| [12, 24] | Standard buttons               |
| [14, 28] | CTA buttons (Add to Cart)      |
| 24px     | Product card padding           |
| 32px     | Feature card padding           |
| [64, 0]  | Section vertical padding       |
| [96, 0]  | Hero section vertical padding  |

### Layout Pattern

- Page width: 1280px max
- Content container: 1120px centered, padding [0, 32]
- Hero section: split layout — image left (50%), copy + CTA right (50%)
- Product grid: 3-4 column, gap 24px
- Feature grid: 3-column with icons, gap 24px
- Trust bar: logo row, single line, grayscale partner logos
- Navigation: fixed top, white, height 64px, shadow on scroll
- Footer: 4-column link grid + payment icons + bottom bar

## Corner Radius

| Value  | Usage                                      | Rationale                         |
| ------ | ------------------------------------------ | --------------------------------- |
| 6px    | Input fields, variant selectors            | Minimal functional rounding       |
| 8px    | Small badges, tag pills                    | Compact rounded elements          |
| 12px   | Buttons, dropdown menus                    | Friendly, tappable button shape   |
| 16px   | Product cards, feature cards               | Primary container radius          |
| 20px   | Hero image containers, promotional banners | Generous warmth                   |
| 9999px | Pill badges, quantity selectors            | Full capsule for compact controls |

Design rationale: The 12-16px range creates a warm, approachable feel that invites browsing. Rounded product cards feel tactile and friendly, while 12px buttons are large enough to feel tappable on mobile. The generous radii signal a consumer-friendly brand that prioritizes comfort over corporate precision.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

shopping-cart, heart, star, search, filter, chevron-right, chevron-down, truck, shield-check, refresh-cw, package, credit-card, tag, percent, plus, minus, check, x, eye, share-2, arrow-right, menu, user, map-pin

### Icon Sizes

| Size | Usage                                      |
| ---- | ------------------------------------------ |
| 16px | Inline with body text, rating stars        |
| 20px | Navigation items, button icons             |
| 24px | Card actions, feature icons                |
| 32px | Feature grid icons (in warm orange circle) |

### Icon Color States

| State     | Color   | Usage                                 |
| --------- | ------- | ------------------------------------- |
| Active    | #F97316 | Primary actions, selected, wishlisted |
| Default   | #57534E | Standard icons, secondary actions     |
| Muted     | #A8A29E | Disabled, placeholder icons           |
| On Accent | #FFFFFF | Icons on orange backgrounds           |
| Success   | #16A34A | In-stock check, delivery confirmed    |
