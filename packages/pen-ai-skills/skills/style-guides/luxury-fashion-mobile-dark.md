---
name: 'luxury-fashion-mobile-dark'
tags: [luxury, dark-mode, serif, elegant, gold-accent, mobile, premium]
platform: mobile
---

## Style Summary

A refined, editorial dark mobile interface designed for luxury fashion, high-end e-commerce, and premium lifestyle applications. The deep black background (#1A1A1A) evokes the interior of a boutique showroom, creating a theatrical backdrop for product imagery and gold accents to command full attention. Cormorant Garamond brings old-world typographic elegance to display headings—its high-contrast serifs and tall ascenders channel the authority of fashion magazine mastheads. Inter provides clean, modern counterpoint for functional text, ensuring prices, sizes, and navigation remain perfectly legible. The restrained 4-8px corner radii project architectural precision and luxury craftsmanship, rejecting the roundness of casual apps in favor of the sharp, deliberate geometry of a jewelry box.

Key aesthetics:

- **Boutique darkness**: #1A1A1A background creates a showroom environment for product imagery
- **Gold accent**: #C9A962 channels luxury, exclusivity, and premium craftsmanship
- **Editorial serif display**: Cormorant Garamond at medium weight for elegant, fashion-magazine headings
- **Sharp precision**: 4-8px corners reflect the deliberate geometry of luxury packaging
- **High contrast**: White text on dark backgrounds with gold punctuation for dramatic hierarchy
- **Pill tab bar**: 100px radius bottom navigation with gold active indicator

## Color System

### Core Backgrounds

| Token            | Value   | Usage                                  |
| ---------------- | ------- | -------------------------------------- |
| Page Background  | #1A1A1A | Root screen background (boutique dark) |
| Card Surface     | #242424 | Standard cards, product tiles          |
| Elevated Surface | #2E2E2E | Elevated cards, modal sheets, drawers  |
| Subtle Surface   | #1F1F1F | Inset areas, grouped sections          |
| Tab Bar Surface  | #242424 | Bottom navigation pill fill            |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #FAFAFA | Headings, product names, primary labels   |
| Secondary Text | #A3A3A3 | Body text, descriptions                   |
| Tertiary Text  | #6B6B6B | Timestamps, captions, inactive labels     |
| Muted Text     | #404040 | Placeholders, disabled text               |
| Accent Text    | #C9A962 | Active states, prices, highlighted values |

### Border Colors

| Token          | Value     | Usage                                        |
| -------------- | --------- | -------------------------------------------- |
| Default Border | #333333   | Card borders, dividers                       |
| Subtle Border  | #2A2A2A   | Light separators                             |
| Active Border  | #C9A962   | Focused inputs, active tab indicator         |
| Gold Line      | #C9A96240 | Subtle gold accent lines, decorative borders |

### Accent Colors

| Token          | Value     | Usage                                             |
| -------------- | --------- | ------------------------------------------------- |
| Primary Accent | #C9A962   | Active states, CTA buttons, tab indicator, prices |
| Accent Light   | #C9A96215 | Tinted backgrounds, selected row highlights       |
| Accent Muted   | #C9A96220 | Subtle badge fills, tag backgrounds               |
| Accent On-Dark | #1A1A1A   | Text/icons on gold backgrounds (dark)             |
| Success        | #22C55E   | Order confirmed, in stock                         |
| Warning        | #D97706   | Low stock, limited edition                        |
| Error          | #EF4444   | Out of stock, payment failed                      |
| Premium        | #C9A962   | VIP, exclusive access (same as accent)            |

## Typography

### Font Families

| Role              | Family             | Usage                                                        |
| ----------------- | ------------------ | ------------------------------------------------------------ |
| Display / Heading | Cormorant Garamond | Brand names, product titles, hero text, editorial headings   |
| Body / Functional | Inter              | Body text, labels, buttons, navigation, prices, descriptions |

### Type Scale

| Level       | Size | Font               | Weight | Usage                                |
| ----------- | ---- | ------------------ | ------ | ------------------------------------ |
| Display     | 36px | Cormorant Garamond | 500    | Hero brand name, collection title    |
| Title 1     | 26px | Cormorant Garamond | 500    | Screen titles, designer names        |
| Title 2     | 20px | Cormorant Garamond | 500    | Section headings, category names     |
| Title 3     | 16px | Cormorant Garamond | 500    | Product names, editorial subheads    |
| Body        | 14px | Inter              | 400    | Descriptions, product details        |
| Label       | 13px | Inter              | 500    | Field labels, button text            |
| Price       | 16px | Inter              | 600    | Product prices, totals               |
| Price Small | 13px | Inter              | 500    | Original prices, secondary amounts   |
| Caption     | 12px | Inter              | 400    | Timestamps, size info, material      |
| Small       | 11px | Inter              | 500    | Badges, collection tags              |
| Micro       | 10px | Inter              | 500    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight   | Value | Usage                                               |
| -------- | ----- | --------------------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions                   |
| Medium   | 500   | Labels, buttons, all Cormorant headings, navigation |
| Semibold | 600   | Prices, emphasis within body                        |

### Letter Spacing

- Display (36px): -0.5px
- Cormorant headings (16-26px): +0.5px (open tracking for elegance)
- Uppercase labels: +2px (wide tracking for luxury feel)
- Body text: 0px
- Prices: 0px

### Line Height

- Display (36px): 1.1
- Headings (16-26px): 1.2
- Body (14px): 1.6
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                    |
| ----- | ---------------------------------------- |
| 2px   | Tight inline pairs                       |
| 4px   | Tab icon to label, price currency symbol |
| 6px   | Size chip groups, color swatch spacing   |
| 8px   | Compact card content, product details    |
| 12px  | Between list items, product grid gap     |
| 16px  | Card internal sections                   |
| 20px  | Between cards, product tile spacing      |
| 24px  | Section gaps within content              |
| 32px  | Top-level screen section breaks          |

### Padding Scale

| Value            | Usage                                    |
| ---------------- | ---------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only) |
| [12, 21, 21, 21] | Tab bar section outer padding            |
| 4px              | Tab bar pill inner padding               |
| [8, 16]          | Input fields, compact buttons            |
| [10, 20]         | Standard buttons                         |
| [14, 28]         | Large buttons, CTA add-to-bag buttons    |
| 16px             | Compact card padding                     |
| 20px             | Standard card padding, product tiles     |
| 24px             | Feature card padding, editorial sections |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Product grid: 2-column grid, 12px gap, tall product imagery (3:4 ratio)
- Editorial layout: full-bleed imagery with overlaid serif titles

## Corner Radius

| Value | Usage                                       | Rationale                                       |
| ----- | ------------------------------------------- | ----------------------------------------------- |
| 4px   | Small buttons, tag chips, size selectors    | Sharp precision for compact elements            |
| 6px   | Input fields, search bars, compact badges   | Subtle refinement                               |
| 8px   | Standard cards, product tiles, containers   | Primary container radius, jewelry-box precision |
| 12px  | Large cards, modal sheets, image containers | Maximum standard radius for larger surfaces     |
| 100px | Tab bar pill, toggle pills, round buttons   | Full capsule shape for navigation               |

Design rationale: Minimal 4-8px radii project the architectural precision and deliberate craftsmanship that luxury brands demand. Sharp corners evoke the geometry of jewelry boxes, perfume bottles, and high-end packaging—forms that communicate quality through restraint. The narrow range (4-12px) rejects the casual roundness of consumer apps, positioning the interface as a curated boutique experience where every angle is intentional.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (refined strokes with elegant proportions)

### Commonly Used Icons

home, search, heart, shopping-bag, user, menu, x, chevron-right, chevron-left, share-2, bookmark, eye, ruler, palette, tag, truck, package, credit-card, star, filter, sliders, grid-2x2, list, camera, bell, arrow-right

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline text indicators, small badges                 |
| 18px | Tab bar icons, list item leading icons               |
| 20px | Card action icons, product action bar                |
| 24px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                       |
| --------- | ------- | ------------------------------------------- |
| Active    | #C9A962 | Selected tab, active states, gold highlight |
| Default   | #6B6B6B | Inactive tabs, secondary actions            |
| Muted     | #404040 | Disabled states, placeholder icons          |
| On Accent | #1A1A1A | Icons on gold-colored backgrounds (dark)    |
