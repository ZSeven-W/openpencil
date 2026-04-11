---
name: 'minimal-playful-light'
tags: [minimal, playful, light-mode, rounded, colorful, friendly, clean, soft-corners]
platform: mobile
---

## Style Summary

A bright, welcoming mobile interface that combines clean minimalism with playful softness. Pure white backgrounds and generous whitespace let the colorful coral accent (#FF6B6B) and semantic status colors pop without overwhelming. Pillow-soft corner radii (14-26px) give every card and button a friendly, approachable quality. Typography pairs the characterful Bricolage Grotesque for display text with the geometric clarity of DM Sans for body content.

Key aesthetics:

- **Pillow-soft corners**: 14-26px radii on cards and buttons create a tactile, inviting feel
- **Coral accent**: #FF6B6B as the primary accent with full semantic color palette (green, blue, amber)
- **Generous whitespace**: Airy layouts with comfortable padding and clear visual breathing room
- **Display character**: Bricolage Grotesque headlines add personality without sacrificing legibility
- **Icon-only tab bar**: Bottom navigation with icons only (no labels) for a clean, minimal footprint
- **Light and open**: Pure #FFFFFF background with subtle gray borders (#E5E5E5) for structure

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                             |
| --------------- | ------- | ------------------------------------------------- |
| Page Background | #FFFFFF | Root screen background                            |
| Card Surface    | #FFFFFF | Cards with border, elevated containers            |
| Inset Surface   | #F5F5F5 | Search bars, input backgrounds, subtle containers |
| Accent Surface  | #FFF0F0 | Coral-tinted highlight areas                      |

### Text Colors

| Token          | Value   | Usage                                   |
| -------------- | ------- | --------------------------------------- |
| Primary Text   | #1A1A1A | Headings, metric values, primary labels |
| Secondary Text | #6B7280 | Body text, descriptions                 |
| Tertiary Text  | #9CA3AF | Captions, timestamps, placeholders      |
| Muted Text     | #D1D5DB | Disabled text, background labels        |
| Accent Text    | #FF6B6B | Active tabs, links, highlighted values  |

### Border Colors

| Token          | Value   | Usage                              |
| -------------- | ------- | ---------------------------------- |
| Default Border | #E5E5E5 | Card borders, input outlines       |
| Subtle Border  | #F0F0F0 | Section separators, light dividers |
| Active Border  | #FF6B6B | Focused inputs, active states      |

### Accent Colors

| Token          | Value     | Usage                                      |
| -------------- | --------- | ------------------------------------------ |
| Primary Accent | #FF6B6B   | Active states, primary buttons, highlights |
| Accent Light   | #FF6B6B20 | Tinted backgrounds, badge fills            |
| Success        | #22C55E   | Positive metrics, completed states         |
| Success Light  | #22C55E20 | Success badge backgrounds                  |
| Info           | #3B82F6   | Informational elements, links              |
| Warning        | #F59E0B   | Alert indicators, pending states           |
| Error          | #EF4444   | Error states, destructive actions          |

## Typography

### Font Families

| Role              | Family              | Usage                                  |
| ----------------- | ------------------- | -------------------------------------- |
| Display           | Bricolage Grotesque | Headings, hero metrics, section titles |
| Body / Functional | DM Sans             | Body text, labels, buttons, navigation |

### Type Scale

| Level   | Size | Font                | Weight | Usage                               |
| ------- | ---- | ------------------- | ------ | ----------------------------------- |
| Display | 36px | Bricolage Grotesque | 700    | Hero metric values                  |
| Title 1 | 24px | Bricolage Grotesque | 600    | Screen titles                       |
| Title 2 | 18px | Bricolage Grotesque | 600    | Section headings                    |
| Title 3 | 16px | DM Sans             | 600    | Card titles, list headings          |
| Body    | 14px | DM Sans             | 400    | Descriptions, body text             |
| Label   | 13px | DM Sans             | 500    | Field labels, button text           |
| Caption | 12px | DM Sans             | 400    | Timestamps, secondary info          |
| Small   | 11px | DM Sans             | 500    | Badges, auxiliary labels            |
| Micro   | 10px | DM Sans             | 500    | Tab labels (if shown), micro badges |

### Font Weights

| Weight   | Value | Usage                             |
| -------- | ----- | --------------------------------- |
| Regular  | 400   | Body text, descriptions, captions |
| Medium   | 500   | Labels, buttons, navigation items |
| Semibold | 600   | Section titles, card headings     |
| Bold     | 700   | Hero metrics, display headings    |

### Letter Spacing

- Display headings: -0.5px
- Uppercase labels: +1px
- Body text: 0px
- Small caps/badges: +0.5px

### Line Height

- Display (36px): 1.0
- Headings (18-24px): 1.2
- Body (14px): 1.5
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                     |
| ----- | ----------------------------------------- |
| 2px   | Tight inline pairs (icon badge overlaps)  |
| 4px   | Tab icon to label (if labels shown)       |
| 6px   | Status indicator to text                  |
| 8px   | Compact card content groups               |
| 12px  | Between list items, form fields           |
| 16px  | Card internal sections, search-to-content |
| 20px  | Between cards in a list                   |
| 24px  | Major section gaps                        |
| 32px  | Top-level screen section breaks           |

### Padding Scale

| Value            | Usage                                    |
| ---------------- | ---------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only) |
| [12, 21, 21, 21] | Tab bar section outer padding            |
| 4px              | Tab bar pill inner padding               |
| [8, 16]          | Input fields, search bars                |
| [10, 20]         | Standard buttons                         |
| [12, 16]         | Compact buttons                          |
| 16px             | Card padding (compact)                   |
| 20px             | Card padding (standard)                  |
| 24px             | Large section padding                    |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned

## Corner Radius

| Value | Usage                                       | Rationale                              |
| ----- | ------------------------------------------- | -------------------------------------- |
| 14px  | Input fields, small buttons, badges         | Soft entry point                       |
| 16px  | Standard cards, list containers             | Primary container radius               |
| 20px  | Large cards, modal sheets                   | Generous softness                      |
| 22px  | Search bars, prominent interactive elements | Pillow-soft feel                       |
| 26px  | Hero cards, featured content                | Maximum softness for featured elements |
| 100px | Tab bar pill, toggle pills, round buttons   | Full capsule/circle shape              |

Design rationale: Pillow-soft radii (14-26px) are the defining visual characteristic. Every surface feels approachable and touchable, like a friendly mobile app that invites interaction. The progressive scale from 14px to 26px creates hierarchy through softness, with the most important containers having the most generous curves.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (complements the soft radius system)

### Commonly Used Icons

home, search, bell, heart, user, plus, check, check-circle, x, star, calendar, clock, target, flame, trophy, smile, sun, moon, activity, bar-chart, trending-up, bookmark, share

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline text indicators, small badges                 |
| 18px | Tab bar icons, list item leading icons               |
| 20px | Card action icons, button leading icons              |
| 24px | Header actions, notification bell, prominent actions |

### Icon Color States

| State     | Color   | Usage                                |
| --------- | ------- | ------------------------------------ |
| Active    | #FF6B6B | Selected tab icon, active navigation |
| Default   | #9CA3AF | Inactive tabs, secondary actions     |
| Muted     | #D1D5DB | Disabled states, placeholder icons   |
| On Accent | #FFFFFF | Icons on coral-colored backgrounds   |
