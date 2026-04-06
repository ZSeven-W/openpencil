---
name: 'dark-bold-mobile'
tags: [dark-mode, bold-typography, neon, electric, geometric, confident, urban, lime-accent]
platform: mobile
---

## Style Summary

A high-energy dark mobile interface built for confidence and impact. Electric lime (#C4F82A) on pitch-black (#0A0A0A) creates a striking neon contrast that demands attention. Space Grotesk display typography at bold weights delivers an urban, street-smart personality, while Manrope body text keeps functional content clean and legible. Gradient program cards and subtle glow effects add depth and movement to the interface.

Key aesthetics:

- **Electric lime on black**: #C4F82A as the sole accent creates high-voltage visual energy against #0A0A0A
- **Bold geometric type**: Space Grotesk at 800 weight for hero metrics, creating a confident urban feel
- **Gradient cards**: Program and feature cards use dark gradient fills for depth and visual interest
- **Glow effects**: Lime-tinted shadows and glow indicators reinforce the neon aesthetic
- **Soft industrial radii**: 8-24px corners balance the geometric sharpness with comfortable touch targets
- **Pill tab bar**: 100px radius bottom navigation with lime active indicator

## Color System

### Core Backgrounds

| Token            | Value   | Usage                                |
| ---------------- | ------- | ------------------------------------ |
| Page Background  | #0A0A0A | Root screen background (pitch black) |
| Card Surface     | #141414 | Standard cards, list items           |
| Elevated Surface | #1A1A1A | Elevated cards, modal sheets         |
| Gradient Start   | #141414 | Gradient card dark edge              |
| Gradient End     | #1E1E1E | Gradient card lighter edge           |
| Tab Bar Surface  | #141414 | Bottom navigation pill fill          |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #FFFFFF | Headings, metric values, primary labels  |
| Secondary Text | #A0A0A0 | Body text, descriptions                  |
| Tertiary Text  | #6A6A6A | Timestamps, captions, inactive labels    |
| Muted Text     | #3A3A3A | Placeholders, disabled text              |
| Accent Text    | #C4F82A | Active states, highlighted values, links |

### Border Colors

| Token          | Value     | Usage                                |
| -------------- | --------- | ------------------------------------ |
| Default Border | #2A2A2A   | Card borders, dividers               |
| Subtle Border  | #1E1E1E   | Light separators                     |
| Active Border  | #C4F82A   | Focused inputs, active tab indicator |
| Glow Border    | #C4F82A40 | Subtle lime glow on active elements  |

### Accent Colors

| Token          | Value     | Usage                                             |
| -------------- | --------- | ------------------------------------------------- |
| Primary Accent | #C4F82A   | Active states, buttons, highlights, tab indicator |
| Accent Glow    | #C4F82A66 | Glow effects, status dot halo (40% opacity)       |
| Accent Muted   | #C4F82A20 | Tinted backgrounds, badge fills                   |
| Accent On-Dark | #0A0A0A   | Text/icons on lime backgrounds (black)            |
| Success        | #22C55E   | Positive metrics, completed habits                |
| Warning        | #F59E0B   | Pending states, streaks                           |
| Error          | #EF4444   | Missed habits, destructive actions                |

## Typography

### Font Families

| Role              | Family        | Usage                                                   |
| ----------------- | ------------- | ------------------------------------------------------- |
| Display           | Space Grotesk | Hero metrics, section headings, prominent titles        |
| Body / Functional | Manrope       | Body text, labels, buttons, navigation, descriptions    |
| Data / Mono       | Space Mono    | Metric values, timestamps, numerical data, chart labels |

### Type Scale

| Level   | Size | Font          | Weight | Usage                                 |
| ------- | ---- | ------------- | ------ | ------------------------------------- |
| Hero    | 52px | Space Grotesk | 800    | Hero percentage, primary stat         |
| Display | 36px | Space Grotesk | 700    | Large metric values                   |
| Title 1 | 24px | Space Grotesk | 700    | Section headings                      |
| Title 2 | 17px | Space Grotesk | 600    | Card titles, list headings            |
| Title 3 | 15px | Manrope       | 600    | Subsection headings, list item titles |
| Body    | 14px | Manrope       | 400    | Descriptions, body text               |
| Label   | 12px | Manrope       | 500    | Field labels, button text             |
| Data    | 14px | Space Mono    | 500    | Metric values, timer displays         |
| Caption | 11px | Manrope       | 400    | Timestamps, metadata                  |
| Micro   | 10px | Manrope       | 600    | Tab labels (uppercase), badges        |
| Tiny    | 9px  | Space Mono    | 400    | Micro data, auxiliary stats           |

### Font Weights

| Weight     | Value | Usage                                        |
| ---------- | ----- | -------------------------------------------- |
| Regular    | 400   | Body text, descriptions                      |
| Medium     | 500   | Labels, data values, navigation              |
| Semibold   | 600   | Card titles, subsection headings, tab labels |
| Bold       | 700   | Section headings, large metrics              |
| Extra Bold | 800   | Hero display numbers                         |

### Letter Spacing

- Hero/Display (36-52px): -2px (dramatic compression)
- Section headings (17-24px): -1px
- Uppercase tab labels: +1.5px
- Body text: 0px
- Monospace data: 0px

### Line Height

- Hero (52px): 0.9
- Display (36px): 1.0
- Headings (17-24px): 1.2
- Body (14px): 1.5
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                 |
| ----- | ------------------------------------- |
| 2px   | Tight inline pairs                    |
| 4px   | Tab icon to label, inline icon groups |
| 6px   | Status indicator groups               |
| 8px   | Compact card content, checklist items |
| 12px  | Between list items, button groups     |
| 16px  | Card internal sections, habit entries |
| 20px  | Between cards, major card sections    |
| 24px  | Section gaps within content           |
| 32px  | Top-level screen section breaks       |

### Padding Scale

| Value            | Usage                                    |
| ---------------- | ---------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only) |
| [12, 21, 21, 21] | Tab bar section outer padding            |
| 4px              | Tab bar pill inner padding               |
| [8, 16]          | Input fields, compact buttons            |
| [10, 20]         | Standard buttons                         |
| [12, 24]         | Large buttons, CTA buttons               |
| 16px             | Compact card padding                     |
| 20px             | Standard card padding                    |
| 24px             | Feature card padding, program cards      |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24-32
- Status bar: 62px standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Program cards: horizontal scroll, gradient background fills

## Corner Radius

| Value | Usage                                       | Rationale                         |
| ----- | ------------------------------------------- | --------------------------------- |
| 8px   | Input fields, small buttons, compact badges | Subtle industrial softening       |
| 12px  | Standard cards, list items, habit entries   | Primary container radius          |
| 16px  | Feature cards, program cards                | Comfortable interactive radius    |
| 20px  | Large cards, modal bottom sheets            | Generous but controlled           |
| 24px  | Hero cards, prominent feature sections      | Maximum standard radius           |
| 100px | Tab bar pill, toggle pills, round buttons   | Full capsule shape for navigation |

Design rationale: Soft industrial radii (8-24px) balance the bold, geometric typography with comfortable touch targets. The range is wider than the terminal style (3-12px) but more restrained than playful (14-26px), reflecting an urban-industrial aesthetic that is confident without being aggressive. The 100px pill for the tab bar provides the signature bottom navigation capsule.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

home, target, trophy, flame, zap, activity, check-circle, clock, calendar, plus, x, chevron-right, trending-up, trending-down, bar-chart-2, star, heart, dumbbell, play, pause, timer, sun, moon, bell, user

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline metrics, checkbox indicators                  |
| 18px | Tab bar icons, list leading icons                    |
| 20px | Card action icons, habit icons                       |
| 24px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                      |
| --------- | ------- | ------------------------------------------ |
| Active    | #C4F82A | Selected tab, active habit, lime highlight |
| Default   | #A0A0A0 | Inactive tabs, secondary actions           |
| Muted     | #3A3A3A | Disabled states, placeholder icons         |
| On Accent | #0A0A0A | Icons on lime-colored backgrounds (black)  |
