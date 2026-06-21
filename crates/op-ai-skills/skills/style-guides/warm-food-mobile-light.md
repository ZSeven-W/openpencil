---
name: 'warm-food-mobile-light'
tags: [warm-tones, light-mode, friendly, orange-accent, rounded, mobile, playful]
platform: mobile
---

## Style Scope

This guide is self-contained. Apply its palette, radius scale, spacing, shadows, and component treatments only when this exact guide is selected; do not borrow food, social, dashboard, luxury, terminal, or other guide-specific patterns into another style. Treat unnamed layout frames as structural by default: no fill, stroke, cornerRadius, or shadow unless the node is intentionally a card, input, button, badge, media mask, navigation surface, or other visible component. Avoid decorative wrapper shells around components; hierarchy should come from spacing, typography, and this guide's explicit surfaces.

## Style Summary

A polished food and restaurant mobile interface with a warm editorial feel rather than a generic delivery-app template. Use an ivory canvas (#FFFCF6), near-black espresso text (#21140F), restrained orange (#FF5A1F) for only decisive actions, and real food photography as the main visual energy. Plus Jakarta Sans keeps the UI friendly and readable, while selective 8-18px radii make cards and controls feel crafted without turning every frame into a rounded bubble. The style should feel appetizing, premium, and composed: photography, spacing, and typographic contrast do the work before colored shells do.

Key aesthetics:

- **Warm ivory canvas**: #FFFCF6 background creates hospitality warmth without a heavy cream tint
- **Restrained orange accent**: #FF5A1F only for CTAs, prices, selected filters, and tiny indicators
- **Rounded sans-serif**: Plus Jakarta Sans with rounded terminals echoes the soft, friendly radii
- **Selective radii**: 8-18px corners distinguish inputs, cards, and hero media without over-softening the page
- **Photography-led hierarchy**: Food images, crops, price typography, and whitespace create the visual hook
- **Integrated navigation**: Bottom navigation stays quiet and full-width, with orange used as an active cue

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                           |
| --------------- | ------- | ----------------------------------------------- |
| Page Background | #FFFCF6 | Root screen background (warm ivory)                                |
| Card Surface    | #FFFFFF | Product/menu cards only; not structural wrappers or carousel shells |
| Inset Surface   | #FFFFFF | Search inputs and form fields with neutral border                   |
| Accent Surface  | #FFF0E3 | Small badges, selected category chips, subtle price highlights      |
| Tab Bar Surface | #FFFCF6 | Integrated bottom navigation surface                                |

### Text Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Primary Text   | #21140F | Headings, prices, primary labels       |
| Secondary Text | #7A5B48 | Body text, descriptions, ingredients   |
| Tertiary Text  | #A8917F | Captions, timestamps, placeholders     |
| Muted Text     | #D7C6B8 | Disabled text, background labels       |
| Accent Text    | #FF5A1F | Active tabs, links, highlighted prices |

### Border Colors

| Token          | Value   | Usage                            |
| -------------- | ------- | -------------------------------- |
| Default Border | #EAD8C8 | Card borders, input outlines     |
| Subtle Border  | #F3E7DC | Light separators, section breaks |
| Active Border  | #FF5A1F | Focused inputs, active states    |

### Accent Colors

| Token          | Value     | Usage                                         |
| -------------- | --------- | --------------------------------------------- |
| Primary Accent | #FF5A1F   | Active states, primary buttons, tab indicator |
| Accent Light   | #FF5A1F14 | Tinted backgrounds, selected items            |
| Accent Muted   | #FFF0E3   | Badge backgrounds, subtle indicators          |
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
| [0, 20]          | Screen content wrapper (horizontal only) |
| [10, 0]          | Integrated bottom nav vertical padding   |
| 0-4px            | Active nav indicator or tiny badge inset |
| [8, 16]          | Input fields, search bars                |
| [10, 20]         | Standard buttons                         |
| [12, 24]         | Large buttons, order CTA buttons         |
| 16px             | Compact card padding                     |
| 20px             | Standard card padding, menu items        |
| 24px             | Feature card padding, promo banners      |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 20], vertical, gap 20-22
- Status bar: 62px, standard iOS
- Header: compact vertical rhythm; avoid oversized blank bands above search
- Search row: transparent structural frame, height 52-56, gap 10, neutral input plus optional orange filter button
- Integrated tab bar: height 58-66px, cornerRadius 0-8, padding [10, 0], full-width in page flow
- Tab items: fill_container, vertical, gap 4, center aligned
- Category rail: small chips or icon+label groups; do not mix pastel pink/blue shells with orange
- Menu cards: white product surfaces with food image top, details bottom, 14-16px radius
- Promo banners: single bold orange panel or image-led panel inside content width; no extra white rounded carousel backing

### Mobile Composition Guardrails

- Keep top rhythm compact: status bar, header, and first content block should feel connected; avoid empty 80px+ bands unless the design is intentionally image-led.
- Search should be one clean row: a neutral input plus optional filter button. Do not wrap the input in a second tinted rounded shell; the grouping frame should stay structural and transparent.
- Horizontal rails and carousel viewport frames stay transparent: no white rounded backing frame, no extra shadow. Only individual cards, images, or intentional promo panels receive fill, radius, or elevation.
- Favorite, like, bookmark, and heart actions on image cards should be icon-only overlays or very subtle translucent hit areas; do not add circular white bubbles, borders, or shadows around the heart.
- Bottom navigation is integrated into the screen flow with a quiet divider or active indicator; avoid floating capsule navigation unless a prompt explicitly asks for a floating nav pattern.
- Use 9999px radius only for true circles or tiny capsules such as avatars, status dots, and short badges; never for page sections, search shells, carousels, card action buttons, or full navigation bars.
- Food palette rule: avoid pastel pink/blue search shells or category backgrounds in this guide. Use warm neutrals, espresso text, one orange accent, and photography color.
- Product-card favorite rule: heart/bookmark icons sit directly over the image with no circular border bubble; use icon color or a faint translucent scrim only when contrast requires it.

## Corner Radius

| Value | Usage                                      | Rationale                         |
| ----- | ------------------------------------------ | --------------------------------- |
| 8px   | Tiny badges, filter chips, nav indicators  | Crisp detail radius               |
| 12px  | Buttons, search inputs, compact controls   | Primary interactive radius        |
| 14px  | Product cards, menu list rows              | App card radius                   |
| 18px  | Promo cards, large image masks             | Maximum food-app softness         |
| 9999px | Avatars, status dots, short badges only | True circles/capsules for semantic micro-elements; never navigation, search, carousels, or card action buttons |

Design rationale: Food UI should feel warm through imagery and rhythm, not through rounding every frame. Keep most structure transparent, use 12-16px for real controls and product cards, and reserve 18px for hero/promotional media. This prevents the common failure mode where search, carousel, category rail, and bottom nav all become competing rounded blobs.

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
