---
name: 'pastel-soft-mobile-light'
tags: [pastel, soft, light-mode, playful, rounded, friendly, mobile]
platform: mobile
---

## Style Summary

A dreamy, soft-focus mobile interface wrapped in pastel warmth. The blush-pink background (#FFF5F7) bathes every screen in gentle color, while the pink accent (#EC4899) adds energetic pops to interactive elements and highlights. Quicksand, a rounded geometric sans-serif, reinforces the soft personality at every typographic level, from bold headings to lightweight body text. Generous 16-24px corner radii create pillow-like containers that feel inviting and touchable, perfect for lifestyle, social, or wellness apps targeting a playful, approachable aesthetic.

Key aesthetics:

- **Blush-pink canvas**: #FFF5F7 background wraps every screen in gentle warmth
- **Pink energy**: #EC4899 accent pops against soft surfaces for clear interactivity
- **Rounded typography**: Quicksand at all levels, its rounded letterforms echo the soft corner radii
- **Pillow corners**: 16-24px radii make every card and container feel soft and touchable
- **Pastel palette**: Muted pinks, lavenders, and warm grays for a cohesive dreamy feel
- **Pill tab bar**: 100px radius bottom navigation with pink active indicator

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                          |
| --------------- | ------- | ---------------------------------------------- |
| Page Background | #FFF5F7 | Root screen background (light blush pink)      |
| Card Surface    | #FFFFFF | Cards, elevated containers                     |
| Inset Surface   | #FFF0F3 | Input backgrounds, search bars, recessed areas |
| Soft Surface    | #FCE7F3 | Highlighted sections, feature callouts         |
| Tab Bar Surface | #FFFFFF | Bottom navigation pill fill                    |

### Text Colors

| Token          | Value   | Usage                                   |
| -------------- | ------- | --------------------------------------- |
| Primary Text   | #1F1215 | Headings, metric values, primary labels |
| Secondary Text | #78606B | Body text, descriptions                 |
| Tertiary Text  | #B8A0AA | Captions, timestamps, placeholders      |
| Muted Text     | #D4C5CB | Disabled text, background labels        |
| Accent Text    | #EC4899 | Active tabs, links, highlighted values  |

### Border Colors

| Token          | Value   | Usage                            |
| -------------- | ------- | -------------------------------- |
| Default Border | #F3D5DE | Card borders, input outlines     |
| Subtle Border  | #FAE8ED | Light separators, section breaks |
| Active Border  | #EC4899 | Focused inputs, active states    |

### Accent Colors

| Token          | Value     | Usage                                           |
| -------------- | --------- | ----------------------------------------------- |
| Primary Accent | #EC4899   | Active states, primary buttons, tab indicator   |
| Accent Light   | #EC489920 | Tinted backgrounds, badge fills                 |
| Accent Muted   | #FBCFE8   | Soft badge backgrounds, subtle indicators       |
| Success        | #34D399   | Positive metrics, completed states (mint green) |
| Success Light  | #34D39920 | Success badge backgrounds                       |
| Warning        | #FBBF24   | Alert indicators, pending states (warm yellow)  |
| Error          | #F87171   | Error states, destructive actions (soft red)    |

## Typography

### Font Families

| Role              | Family    | Usage                                         |
| ----------------- | --------- | --------------------------------------------- |
| Display / Heading | Quicksand | Screen titles, section headings, hero metrics |
| Body / Functional | Quicksand | Body text, labels, buttons, navigation        |

### Type Scale

| Level   | Size | Font      | Weight | Usage                                |
| ------- | ---- | --------- | ------ | ------------------------------------ |
| Display | 34px | Quicksand | 700    | Hero metric values, large titles     |
| Title 1 | 24px | Quicksand | 700    | Screen titles                        |
| Title 2 | 18px | Quicksand | 600    | Section headings                     |
| Title 3 | 16px | Quicksand | 600    | Card titles, list headings           |
| Body    | 14px | Quicksand | 400    | Body text, descriptions              |
| Label   | 13px | Quicksand | 500    | Field labels, button text            |
| Caption | 12px | Quicksand | 400    | Timestamps, secondary info           |
| Small   | 11px | Quicksand | 500    | Badges, auxiliary labels             |
| Micro   | 10px | Quicksand | 600    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight   | Value | Usage                             |
| -------- | ----- | --------------------------------- |
| Regular  | 400   | Body text, descriptions, captions |
| Medium   | 500   | Labels, buttons, navigation items |
| Semibold | 600   | Section headings, card titles     |
| Bold     | 700   | Screen titles, hero metrics       |

### Letter Spacing

- Display (34px): -0.5px
- Section headings (16-24px): -0.3px
- Uppercase tab labels: +1.5px
- Body text: 0px
- Small caps/badges: +0.5px

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
| 8px   | Compact card content groups               |
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
- Feature cards: soft pink surface (#FCE7F3) with generous 24px radius

## Corner Radius

| Value | Usage                                      | Rationale                         |
| ----- | ------------------------------------------ | --------------------------------- |
| 12px  | Small buttons, compact badges, tag chips   | Soft baseline rounding            |
| 16px  | Standard cards, list containers, inputs    | Primary container radius          |
| 20px  | Large cards, search bars, grouped sections | Generous softness                 |
| 24px  | Hero cards, feature sections, modal sheets | Maximum pillow softness           |
| 100px | Tab bar pill, toggle pills, round avatars  | Full capsule shape for navigation |

Design rationale: Very soft 16-24px radii are the defining characteristic, making every surface feel like a pastel cushion. The rounded Quicksand typeface and pillow-like corners work together to create a cohesive dreamy aesthetic. Progressive scaling from 12px to 24px provides hierarchy through softness, with featured content getting the most generous curves.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (round strokes complement the soft, pastel aesthetic)

### Commonly Used Icons

heart, star, smile, sun, moon, sparkles, flower, cloud, music, camera, gift, palette, feather, coffee, cake, shopping-bag, bookmark, bell, user, plus, check, x, chevron-right, search

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline text indicators, small badges                 |
| 18px | Tab bar icons, list item leading icons               |
| 20px | Card action icons, button leading icons              |
| 24px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                |
| --------- | ------- | ------------------------------------ |
| Active    | #EC4899 | Selected tab icon, active navigation |
| Default   | #B8A0AA | Inactive tabs, secondary actions     |
| Muted     | #D4C5CB | Disabled states, placeholder icons   |
| On Accent | #FFFFFF | Icons on pink-colored backgrounds    |
