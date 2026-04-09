---
name: 'wellness-organic-light'
tags: [wellness, organic, light-mode, sage-green, warm-tones, rounded, friendly]
platform: webapp
---

## Style Summary

A calming, nature-inspired interface designed for wellness apps, health platforms, and mindfulness products. The soft botanical background (#FAFDF7) carries a gentle green undertone that evokes morning light through leaves, while sage green (#4D7C0F) grounds the entire palette in natural vitality. Nunito's rounded terminals deliver a warm, approachable voice at every text level — from bold section headings to gentle body copy. Generous 16-20px radii create soft, organic shapes that mirror the smooth curves of stones, petals, and natural forms.

Key aesthetics:

- **Botanical canvas**: #FAFDF7 background with a subtle green undertone, like sunlit paper
- **Sage green accent**: #4D7C0F evokes natural foliage, herbal wellness, and organic vitality
- **Rounded typography**: Nunito's circular letterforms feel nurturing and accessible
- **Organic radii**: 16-20px corners create pillow-soft, nature-inspired container shapes
- **Warm neutral tones**: Text colors with warm undertones (#2D2A26) avoid clinical coldness
- **Generous breathing room**: Extra whitespace and padding create a meditative, uncluttered feel

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                          |
| --------------- | ------- | ---------------------------------------------- |
| Page Background | #FAFDF7 | Root page background (botanical tint)          |
| Card Surface    | #FFFFFF | Cards, elevated containers                     |
| Inset Surface   | #F4F7F0 | Input backgrounds, recessed areas              |
| Warm Surface    | #FBF9F4 | Secondary containers, sidebar areas            |
| Accent Surface  | #F0F7E8 | Sage-tinted highlight sections, callout blocks |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #2D2A26 | Headings, primary content (warm charcoal) |
| Secondary Text | #5C5750 | Body text, descriptions                   |
| Tertiary Text  | #8A857C | Captions, timestamps, helper text         |
| Muted Text     | #C4BFB6 | Placeholders, disabled text               |
| Accent Text    | #4D7C0F | Active states, links, highlighted values  |

### Border Colors

| Token          | Value     | Usage                                 |
| -------------- | --------- | ------------------------------------- |
| Default Border | #E8E5DE   | Card borders, dividers (warm tint)    |
| Subtle Border  | #F0EDEA   | Light separators                      |
| Active Border  | #4D7C0F   | Focused inputs, active indicators     |
| Accent Border  | #4D7C0F40 | Soft sage borders for accent sections |

### Accent Colors

| Token          | Value     | Usage                                      |
| -------------- | --------- | ------------------------------------------ |
| Primary Accent | #4D7C0F   | Primary buttons, active states, sage green |
| Accent Hover   | #3F6B0A   | Button hover, pressed states               |
| Accent Light   | #4D7C0F15 | Tinted backgrounds, selected items         |
| Accent Muted   | #4D7C0F40 | Badge backgrounds, progress fills          |
| Success        | #15803D   | Completed goals, positive health metrics   |
| Wellness Blue  | #0E7490   | Hydration, sleep, calm-related indicators  |
| Warm Amber     | #B45309   | Energy, activity, motivation indicators    |
| Gentle Red     | #B91C1C   | Alert states, overdue reminders            |

## Typography

### Font Families

| Role              | Family | Usage                                                       |
| ----------------- | ------ | ----------------------------------------------------------- |
| Headings          | Nunito | Section headings, card titles, display metrics (weight 700) |
| Body / Functional | Nunito | Body text, labels, buttons, navigation (weight 400)         |

### Type Scale

| Level      | Size | Font   | Weight | Usage                                     |
| ---------- | ---- | ------ | ------ | ----------------------------------------- |
| Display    | 44px | Nunito | 700    | Hero heading, welcome message             |
| Metric     | 36px | Nunito | 700    | Primary wellness metric (steps, calories) |
| Title 1    | 26px | Nunito | 700    | Section headings                          |
| Title 2    | 20px | Nunito | 600    | Card titles, subsection headings          |
| Title 3    | 16px | Nunito | 600    | List headings, compact card titles        |
| Body Large | 15px | Nunito | 400    | Featured descriptions, wellness tips      |
| Body       | 14px | Nunito | 400    | Standard body text, descriptions          |
| Label      | 12px | Nunito | 600    | Field labels, column headers, tags        |
| Caption    | 11px | Nunito | 400    | Timestamps, secondary metadata            |
| Micro      | 10px | Nunito | 600    | Badges, streak counters                   |

### Font Weights

| Weight   | Value | Usage                                           |
| -------- | ----- | ----------------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions               |
| Semibold | 600   | Labels, card titles, navigation items           |
| Bold     | 700   | Display headings, section titles, metric values |

### Letter Spacing

- Display (44px): -1px
- Section headings (20-26px): -0.5px
- Uppercase labels: +1px
- Body text: 0px
- Metric values: -0.5px

### Line Height

- Display (44px): 1.0
- Headings (20-26px): 1.2
- Body (14-15px): 1.6 (generous for readability)
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                             |
| ----- | --------------------------------- |
| 4px   | Inline icon+text pairs            |
| 6px   | Status indicator to label         |
| 8px   | Compact element groups, tag rows  |
| 12px  | Between list items, form fields   |
| 16px  | Card internal sections, nav items |
| 20px  | Between cards in a grid           |
| 24px  | Content section gaps              |
| 32px  | Major section breaks              |
| 40px  | Wellness module gaps              |
| 48px  | Top-level page sections           |

### Padding Scale

| Value    | Usage                                  |
| -------- | -------------------------------------- |
| [8, 16]  | Input fields, compact buttons          |
| [10, 24] | Standard buttons                       |
| [12, 28] | Large buttons, call-to-action          |
| 20px     | Card padding (compact)                 |
| 24px     | Card padding (standard)                |
| 28px     | Feature card padding, wellness modules |
| [32, 40] | Content area padding                   |
| [24, 32] | Navigation area padding                |

### Layout Pattern

- Screen width: 1440px
- Navigation: top horizontal bar, height 72px, centered logo + text nav items
- Content area: max-width 1200px centered, vertical, gap 32-48, padding [32, 40]
- Wellness modules: full-width cards with internal horizontal layouts
- Cards use soft shadow (0 2px 12px rgba(45,42,38,0.06)) with optional sage-tinted border
- Feature sections may use accent surface (#F0F7E8) as background

## Corner Radius

| Value | Usage                                          | Rationale                        |
| ----- | ---------------------------------------------- | -------------------------------- |
| 8px   | Input fields, small badges, tags               | Minimal organic softening        |
| 12px  | Buttons, dropdowns, tooltips                   | Soft interactive radius          |
| 16px  | Standard cards, list containers, modals        | Primary organic container radius |
| 20px  | Feature cards, wellness modules, hero sections | Maximum organic softness         |
| 100px | Pill buttons, toggle switches, avatar circles  | Full capsule/circle shape        |

Design rationale: The 8-20px radius range creates the signature organic feel of wellness design. Radii at 16-20px mirror the smooth, rounded forms found in nature — river stones, seed pods, leaf edges. Unlike the aggressive 24px+ of playful styles, this range maintains sophistication while feeling nurturing and safe. The generous curves invite interaction and create a sense of comfort that aligns with wellness brand values.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (soft stroke style complements organic radii)

### Commonly Used Icons

heart, activity, sun, moon, droplets, leaf, flower, apple, brain, smile, target, flame, trending-up, calendar, clock, check-circle, star, wind, cloud, thermometer, footprints, dumbbell, bed, coffee, search, bell, settings, user, plus, x

### Icon Sizes

| Size | Usage                                        |
| ---- | -------------------------------------------- |
| 14px | Inline metric indicators, tag icons          |
| 16px | Navigation icons, list leading icons         |
| 20px | Card header actions, wellness category icons |
| 24px | Primary navigation, header actions           |
| 32px | Wellness module feature icons                |

### Icon Color States

| State     | Color   | Usage                                         |
| --------- | ------- | --------------------------------------------- |
| Active    | #4D7C0F | Selected navigation, active wellness category |
| Default   | #5C5750 | Standard icons, secondary actions             |
| Wellness  | #0E7490 | Hydration and calm-related icons              |
| Warm      | #B45309 | Energy and activity-related icons             |
| Muted     | #C4BFB6 | Disabled icons, placeholder states            |
| On Accent | #FFFFFF | Icons on sage-green backgrounds               |
