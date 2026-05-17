---
name: 'travel-warm-mobile-light'
tags: [warm-tones, light-mode, friendly, orange-accent, rounded, mobile, organic]
platform: mobile
---

## Style Summary

A warm, inviting mobile interface designed for travel, booking, and destination discovery applications. The creamy off-white background (#FFFDF7) evokes sun-bleached paper and sandy beaches, creating a wanderlust-inducing canvas that feels like flipping through a travel journal. Deep orange (#EA580C) serves as the adventurous accent—bold enough to inspire action on booking CTAs without the aggression of pure red. Outfit provides clean geometric readability with subtle warmth in its letterforms, making destination names and trip details equally inviting. Generous 16-20px corner radii create soft, postcard-like containers that invite exploration and discovery.

Key aesthetics:

- **Sun-warmed canvas**: #FFFDF7 background evokes travel journals and sun-bleached destinations
- **Adventure orange accent**: #EA580C inspires action and excitement without aggressive urgency
- **Travel-journal type**: Outfit typeface balances geometric clarity with warm, approachable character
- **Postcard radii**: 16-20px corners create soft, photo-friendly containers for destination imagery
- **Earth-toned palette**: Warm grays and amber tints reinforce the organic, exploratory mood
- **Pill tab bar**: 100px radius bottom navigation with orange active indicator

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                          |
| --------------- | ------- | ---------------------------------------------- |
| Page Background | #FFFDF7 | Root screen background (warm cream)            |
| Card Surface    | #FFFFFF | Cards, destination tiles, booking containers   |
| Inset Surface   | #FFF7ED | Input backgrounds, search bars, recessed areas |
| Subtle Surface  | #FFFBEB | Featured destinations, promotional banners     |
| Tab Bar Surface | #FFFFFF | Bottom navigation pill fill                    |

### Text Colors

| Token          | Value   | Usage                                       |
| -------------- | ------- | ------------------------------------------- |
| Primary Text   | #292014 | Headings, destination names, primary labels |
| Secondary Text | #78716C | Body text, descriptions, trip details       |
| Tertiary Text  | #A8A29E | Timestamps, captions, placeholders          |
| Muted Text     | #D6D3D1 | Disabled text, background labels            |
| Accent Text    | #EA580C | Active tabs, links, price highlights        |

### Border Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Default Border | #E7E5E4 | Card borders, input outlines, dividers |
| Subtle Border  | #F5F5F4 | Light separators, section breaks       |
| Active Border  | #EA580C | Focused inputs, active states          |

### Accent Colors

| Token          | Value     | Usage                                       |
| -------------- | --------- | ------------------------------------------- |
| Primary Accent | #EA580C   | Active states, book buttons, tab indicator  |
| Accent Light   | #EA580C15 | Tinted backgrounds, selected row highlights |
| Accent Muted   | #FFEDD5   | Badge backgrounds, deal tags                |
| Accent Warm    | #FED7AA   | Warm highlight surfaces, rating backgrounds |
| Success        | #16A34A   | Booking confirmed, available dates          |
| Warning        | #D97706   | Limited availability, price alerts          |
| Error          | #DC2626   | Booking failed, sold out, cancellation      |
| Rating         | #F59E0B   | Star ratings, review scores (amber)         |

## Typography

### Font Families

| Role              | Family | Usage                                       |
| ----------------- | ------ | ------------------------------------------- |
| Display / Heading | Outfit | Screen titles, destination names, hero text |
| Body / Functional | Outfit | Body text, labels, buttons, navigation      |

### Type Scale

| Level   | Size | Font   | Weight | Usage                                     |
| ------- | ---- | ------ | ------ | ----------------------------------------- |
| Display | 32px | Outfit | 600    | Hero destination name, trip title         |
| Title 1 | 24px | Outfit | 600    | Screen titles                             |
| Title 2 | 18px | Outfit | 600    | Section headings, category names          |
| Title 3 | 16px | Outfit | 500    | Card titles, hotel names, activity titles |
| Body    | 14px | Outfit | 400    | Descriptions, trip details, reviews       |
| Label   | 13px | Outfit | 500    | Field labels, button text, prices         |
| Caption | 12px | Outfit | 400    | Dates, secondary info, distance           |
| Small   | 11px | Outfit | 500    | Badges, deal labels, rating counts        |
| Micro   | 10px | Outfit | 500    | Tab labels (uppercase), micro badges      |

### Font Weights

| Weight   | Value | Usage                                    |
| -------- | ----- | ---------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions        |
| Medium   | 500   | Labels, buttons, card titles, navigation |
| Semibold | 600   | Section headings, screen titles, display |

### Letter Spacing

- Display (32px): -0.5px
- Section headings (16-24px): -0.3px
- Uppercase tab labels: +1px
- Body text: 0px

### Line Height

- Display (32px): 1.1
- Headings (16-24px): 1.25
- Body (14px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                       |
| ----- | ------------------------------------------- |
| 2px   | Tight inline pairs                          |
| 4px   | Tab icon to label, star rating icons        |
| 6px   | Location pin to place name                  |
| 8px   | Compact card content, amenity tags          |
| 12px  | Between list items, horizontal scroll cards |
| 16px  | Card internal sections, photo-to-details    |
| 20px  | Between cards, destination tiles            |
| 24px  | Section gaps within content                 |
| 32px  | Top-level screen section breaks             |

### Padding Scale

| Value            | Usage                                    |
| ---------------- | ---------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only) |
| [12, 21, 21, 21] | Tab bar section outer padding            |
| 4px              | Tab bar pill inner padding               |
| [8, 16]          | Input fields, search bars                |
| [10, 20]         | Standard buttons                         |
| [12, 24]         | Large buttons, booking CTA buttons       |
| 16px             | Compact card padding, amenity lists      |
| 20px             | Standard card padding                    |
| 24px             | Feature card padding, destination hero   |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Destination cards: full-width image top, details bottom, 16px radius
- Horizontal scroll: category chips or destination cards, 12px gap

## Corner Radius

| Value | Usage                                                | Rationale                               |
| ----- | ---------------------------------------------------- | --------------------------------------- |
| 8px   | Small buttons, amenity tags, filter chips            | Gentle softening for compact elements   |
| 12px  | Input fields, search bars, compact cards             | Comfortable baseline radius             |
| 16px  | Standard cards, destination tiles, booking items     | Primary container radius, postcard-like |
| 20px  | Large cards, hero destination, modal sheets          | Generous radius for featured content    |
| 100px | Tab bar pill, toggle pills, round avatars, pill tags | Full capsule shape                      |

Design rationale: Generous 16-20px radii create the soft, postcard-like containers that evoke travel photography and destination guides. The rounded forms feel organic and inviting, like smooth beach stones or rounded passport stamps, encouraging users to explore and discover. Image-heavy destination cards benefit from the softer framing that larger radii provide, making photos feel more integrated with the interface.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (organic strokes complement the warm aesthetic)

### Commonly Used Icons

home, search, heart, compass, map, map-pin, plane, luggage, hotel, calendar, clock, star, sun, cloud, camera, image, share-2, bookmark, navigation, globe, users, utensils, car, train, plus, x, chevron-right, filter

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline text indicators, rating stars                 |
| 18px | Tab bar icons, list item leading icons               |
| 20px | Card action icons, amenity icons                     |
| 24px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                |
| --------- | ------- | ------------------------------------ |
| Active    | #EA580C | Selected tab icon, active navigation |
| Rating    | #F59E0B | Star ratings, review stars (amber)   |
| Default   | #A8A29E | Inactive tabs, secondary actions     |
| Muted     | #D6D3D1 | Disabled states, placeholder icons   |
| On Accent | #FFFFFF | Icons on orange-colored backgrounds  |
