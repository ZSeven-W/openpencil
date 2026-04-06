---
name: 'wellness-green-mobile-light'
tags: [wellness, sage-green, light-mode, organic, calm, mobile, soft-corners]
platform: mobile
---

## Style Summary

A serene, nature-inspired mobile interface designed for health, wellness, and mindfulness applications. The soft green-tinted background (#F7FDF4) evokes morning light through foliage, while the green accent (#16A34A) channels growth and vitality. Outfit, a clean geometric sans-serif with subtle humanist touches, provides warmth and approachability at every typographic level. Organic 12-20px corner radii create comfortable, natural-feeling containers, and the overall palette draws from earth tones and botanical greens for a calming, grounded experience.

Key aesthetics:

- **Nature-tinted canvas**: #F7FDF4 background evokes the soft light of a garden or greenhouse
- **Growth green accent**: #16A34A as the primary accent, representing vitality and healthy progress
- **Humanist geometry**: Outfit typeface balances geometric clarity with warm, approachable letterforms
- **Organic radii**: 12-20px corners feel natural and comfortable, like smooth river stones
- **Earth tones**: Warm grays and muted greens for secondary text and borders
- **Pill tab bar**: 100px radius bottom navigation with green active indicator

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                          |
| --------------- | ------- | ---------------------------------------------- |
| Page Background | #F7FDF4 | Root screen background (soft green tint)       |
| Card Surface    | #FFFFFF | Cards, elevated containers                     |
| Inset Surface   | #F0F9EC | Input backgrounds, search bars, recessed areas |
| Accent Surface  | #DCFCE7 | Highlighted sections, progress indicators      |
| Tab Bar Surface | #FFFFFF | Bottom navigation pill fill                    |

### Text Colors

| Token          | Value   | Usage                                   |
| -------------- | ------- | --------------------------------------- |
| Primary Text   | #14241A | Headings, metric values, primary labels |
| Secondary Text | #4B6356 | Body text, descriptions                 |
| Tertiary Text  | #8BA898 | Captions, timestamps, placeholders      |
| Muted Text     | #C2D4CA | Disabled text, background labels        |
| Accent Text    | #16A34A | Active tabs, links, highlighted values  |

### Border Colors

| Token          | Value   | Usage                            |
| -------------- | ------- | -------------------------------- |
| Default Border | #D4E8DA | Card borders, input outlines     |
| Subtle Border  | #E8F5EB | Light separators, section breaks |
| Active Border  | #16A34A | Focused inputs, active states    |

### Accent Colors

| Token          | Value     | Usage                                              |
| -------------- | --------- | -------------------------------------------------- |
| Primary Accent | #16A34A   | Active states, primary buttons, tab indicator      |
| Accent Light   | #16A34A15 | Tinted backgrounds, selected row highlights        |
| Accent Muted   | #BBF7D0   | Badge backgrounds, subtle indicators               |
| Success        | #16A34A   | Positive metrics, completed goals (same as accent) |
| Warning        | #CA8A04   | Alert indicators, hydration reminders (warm amber) |
| Error          | #DC2626   | Error states, missed goals (muted red)             |
| Calm           | #6366F1   | Meditation, sleep tracking (soft indigo)           |

## Typography

### Font Families

| Role              | Family | Usage                                         |
| ----------------- | ------ | --------------------------------------------- |
| Display / Heading | Outfit | Screen titles, section headings, hero metrics |
| Body / Functional | Outfit | Body text, labels, buttons, navigation        |

### Type Scale

| Level   | Size | Font   | Weight | Usage                                |
| ------- | ---- | ------ | ------ | ------------------------------------ |
| Display | 34px | Outfit | 700    | Hero metric values, daily score      |
| Title 1 | 24px | Outfit | 600    | Screen titles                        |
| Title 2 | 18px | Outfit | 600    | Section headings                     |
| Title 3 | 16px | Outfit | 500    | Card titles, list headings           |
| Body    | 14px | Outfit | 400    | Body text, descriptions              |
| Label   | 13px | Outfit | 500    | Field labels, button text            |
| Caption | 12px | Outfit | 400    | Timestamps, secondary info           |
| Small   | 11px | Outfit | 500    | Badges, auxiliary labels             |
| Micro   | 10px | Outfit | 500    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight   | Value | Usage                                    |
| -------- | ----- | ---------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions        |
| Medium   | 500   | Labels, buttons, card titles, navigation |
| Semibold | 600   | Section headings, screen titles          |
| Bold     | 700   | Hero metrics, display headings           |

### Letter Spacing

- Display (34px): -0.5px
- Section headings (16-24px): -0.3px
- Uppercase tab labels: +1px
- Body text: 0px

### Line Height

- Display (34px): 1.0
- Headings (16-24px): 1.2
- Body (14px): 1.5
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                     |
| ----- | ----------------------------------------- |
| 2px   | Tight inline pairs                        |
| 4px   | Tab icon to label, inline icon groups     |
| 6px   | Status indicator to text                  |
| 8px   | Compact card content, activity entries    |
| 12px  | Between list items, form fields           |
| 16px  | Card internal sections, search-to-content |
| 20px  | Between cards in a list                   |
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
| [12, 24]         | Large buttons, CTA buttons               |
| 16px             | Compact card padding                     |
| 20px             | Standard card padding                    |
| 24px             | Feature card padding, large sections     |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Progress cards: accent surface (#DCFCE7) with green progress bars

## Corner Radius

| Value | Usage                                         | Rationale                         |
| ----- | --------------------------------------------- | --------------------------------- |
| 8px   | Small buttons, compact badges, progress pills | Gentle baseline softening         |
| 12px  | Standard cards, list containers, inputs       | Primary container radius          |
| 16px  | Large cards, search bars, grouped sections    | Comfortable organic radius        |
| 20px  | Hero cards, feature sections, modal sheets    | Maximum organic softness          |
| 100px | Tab bar pill, toggle pills, round avatars     | Full capsule shape for navigation |

Design rationale: Organic 12-20px radii echo the smooth, rounded forms found in nature: pebbles, leaves, and seed pods. The range avoids sharp technical edges while staying grounded enough for a wellness context, never crossing into overly playful territory. Every container feels like a natural surface you can rest your attention on.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (organic strokes complement the nature-inspired aesthetic)

### Commonly Used Icons

heart, activity, droplet, sun, moon, leaf, flower, apple, flame, footprints, timer, clock, calendar, target, trophy, smile, cloud, wind, thermometer, bed, dumbbell, salad, plus, check, x, chevron-right

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline text indicators, small badges                 |
| 18px | Tab bar icons, list item leading icons               |
| 20px | Card action icons, activity icons                    |
| 24px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                |
| --------- | ------- | ------------------------------------ |
| Active    | #16A34A | Selected tab icon, active navigation |
| Default   | #8BA898 | Inactive tabs, secondary actions     |
| Muted     | #C2D4CA | Disabled states, placeholder icons   |
| On Accent | #FFFFFF | Icons on green-colored backgrounds   |
