---
name: 'warm-food-mobile-light'
tags: [warm-tones, light-mode, friendly, orange-accent, rounded, mobile, playful]
platform: mobile
---

## Style Summary

A warm, appetite-inducing mobile interface designed for food delivery, restaurant, and recipe applications. The creamy background (#FFF8F0) wraps every screen in the inviting warmth of a bakery, while the vibrant orange accent (#F97316) channels the energy of fresh citrus and spice. Plus Jakarta Sans, a geometric sans-serif with rounded terminals, provides friendly readability at every level, from bold hero prices to lightweight ingredient lists. Generous 16-20px corner radii create soft, approachable containers that feel as warm and welcoming as the color palette.

Key aesthetics:

- **Creamy warm canvas**: #FFF8F0 background evokes the warmth of baked goods and natural ingredients
- **Citrus orange accent**: #F97316 as the primary accent, energetic and appetite-stimulating
- **Rounded sans-serif**: Plus Jakarta Sans with rounded terminals echoes the soft, friendly radii
- **Generous radii**: 16-20px corners make cards feel like warm, inviting menu tiles
- **Food photography friendly**: Clean white card surfaces maximize contrast for food imagery
- **Pill tab bar**: 100px radius bottom navigation with orange active indicator

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                           |
| --------------- | ------- | ----------------------------------------------- |
| Page Background | #FFF8F0 | Root screen background (warm cream)             |
| Card Surface    | #FFFFFF | Cards, menu items, elevated containers          |
| Inset Surface   | #FFF1E3 | Input backgrounds, search bars, recessed areas  |
| Accent Surface  | #FED7AA | Highlighted sections, promo banners, deal cards |
| Tab Bar Surface | #FFFFFF | Bottom navigation pill fill                     |

### Text Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Primary Text   | #1C1410 | Headings, prices, primary labels       |
| Secondary Text | #78604A | Body text, descriptions, ingredients   |
| Tertiary Text  | #B8A08A | Captions, timestamps, placeholders     |
| Muted Text     | #D4C4B4 | Disabled text, background labels       |
| Accent Text    | #F97316 | Active tabs, links, highlighted prices |

### Border Colors

| Token          | Value   | Usage                            |
| -------------- | ------- | -------------------------------- |
| Default Border | #F0DCC8 | Card borders, input outlines     |
| Subtle Border  | #F8EDE0 | Light separators, section breaks |
| Active Border  | #F97316 | Focused inputs, active states    |

### Accent Colors

| Token          | Value     | Usage                                         |
| -------------- | --------- | --------------------------------------------- |
| Primary Accent | #F97316   | Active states, primary buttons, tab indicator |
| Accent Light   | #F9731620 | Tinted backgrounds, selected items            |
| Accent Muted   | #FFEDD5   | Badge backgrounds, subtle indicators          |
| Success        | #16A34A   | Order confirmed, available items (green)      |
| Success Light  | #16A34A20 | Success badge backgrounds                     |
| Warning        | #EAB308   | Delivery delays, limited stock (warm yellow)  |
| Error          | #DC2626   | Out of stock, order issues                    |
| Rating         | #F59E0B   | Star ratings, review scores (amber)           |

## Typography

### Font Families

| Role              | Family            | Usage                                        |
| ----------------- | ----------------- | -------------------------------------------- |
| Display / Heading | Plus Jakarta Sans | Screen titles, section headings, hero prices |
| Body / Functional | Plus Jakarta Sans | Body text, labels, buttons, navigation       |

### Type Scale

| Level   | Size | Font              | Weight | Usage                                |
| ------- | ---- | ----------------- | ------ | ------------------------------------ |
| Display | 34px | Plus Jakarta Sans | 700    | Hero prices, promo values            |
| Title 1 | 24px | Plus Jakarta Sans | 700    | Screen titles                        |
| Title 2 | 18px | Plus Jakarta Sans | 600    | Section headings, category titles    |
| Title 3 | 16px | Plus Jakarta Sans | 600    | Card titles, restaurant names        |
| Body    | 14px | Plus Jakarta Sans | 400    | Descriptions, ingredients            |
| Label   | 13px | Plus Jakarta Sans | 500    | Field labels, button text            |
| Price   | 16px | Plus Jakarta Sans | 700    | Item prices, totals                  |
| Caption | 12px | Plus Jakarta Sans | 400    | Delivery times, distances, reviews   |
| Small   | 11px | Plus Jakarta Sans | 500    | Badges, deal labels                  |
| Micro   | 10px | Plus Jakarta Sans | 600    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight   | Value | Usage                               |
| -------- | ----- | ----------------------------------- |
| Regular  | 400   | Body text, descriptions, captions   |
| Medium   | 500   | Labels, buttons, navigation items   |
| Semibold | 600   | Section headings, card titles       |
| Bold     | 700   | Screen titles, prices, hero metrics |

### Letter Spacing

- Display (34px): -0.5px
- Section headings (16-24px): -0.3px
- Uppercase tab labels: +1px
- Body text: 0px
- Price values: -0.3px

### Line Height

- Display (34px): 1.0
- Headings (16-24px): 1.2
- Body (14px): 1.5
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                     |
| ----- | ----------------------------------------- |
| 2px   | Tight inline pairs, rating stars          |
| 4px   | Tab icon to label, inline icon groups     |
| 6px   | Status indicator to text, star to count   |
| 8px   | Compact card content, menu item details   |
| 12px  | Between list items, form fields           |
| 16px  | Card internal sections, search-to-content |
| 20px  | Between cards in a list, menu categories  |
| 24px  | Section gaps within content               |
| 32px  | Top-level screen section breaks           |

### Padding Scale

| Value            | Usage                                    |
| ---------------- | ---------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only) |
| [12, 21, 21, 21] | Tab bar section outer padding            |
| 4px              | Tab bar pill inner padding               |
| [8, 16]          | Input fields, search bars                |
| [10, 20]         | Standard buttons                         |
| [12, 24]         | Large buttons, order CTA buttons         |
| 16px             | Compact card padding                     |
| 20px             | Standard card padding, menu items        |
| 24px             | Feature card padding, promo banners      |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Menu cards: white surface with food image top, details bottom, 16px radius
- Promo banners: accent surface (#FED7AA) with warm gradient feel

## Corner Radius

| Value | Usage                                      | Rationale                         |
| ----- | ------------------------------------------ | --------------------------------- |
| 10px  | Small buttons, compact badges, tag chips   | Warm baseline softening           |
| 16px  | Standard cards, menu items, input fields   | Primary container radius          |
| 20px  | Large cards, feature sections, search bars | Generous friendly radius          |
| 24px  | Hero banners, promo cards, modal sheets    | Maximum warm softness             |
| 100px | Tab bar pill, toggle pills, round avatars  | Full capsule shape for navigation |

Design rationale: Generous 16-20px radii create the warm, inviting feel expected in food and hospitality apps. Every card feels like a warm plate or a rounded bread basket, reinforcing the comfort-food aesthetic. The soft corners also ensure food photography thumbnails sit inside containers with gentle, appetizing framing rather than harsh crops.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (warm, friendly strokes complement the food aesthetic)

### Commonly Used Icons

utensils, chef-hat, clock, map-pin, star, heart, shopping-cart, shopping-bag, search, filter, plus, minus, x, check, truck, bike, flame, leaf, coffee, pizza, salad, gift, percent, bell, user, chevron-right, navigation

### Icon Sizes

| Size | Usage                                        |
| ---- | -------------------------------------------- |
| 14px | Inline text indicators, rating stars         |
| 18px | Tab bar icons, list item leading icons       |
| 20px | Card action icons, menu item icons           |
| 24px | Header actions, cart icon, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                |
| --------- | ------- | ------------------------------------ |
| Active    | #F97316 | Selected tab icon, active navigation |
| Default   | #B8A08A | Inactive tabs, secondary actions     |
| Muted     | #D4C4B4 | Disabled states, placeholder icons   |
| On Accent | #FFFFFF | Icons on orange-colored backgrounds  |
