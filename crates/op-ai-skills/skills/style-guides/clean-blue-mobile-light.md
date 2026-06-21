---
name: 'clean-blue-mobile-light'
tags: [clean, blue-accent, light-mode, minimal, mobile, rounded, friendly]
platform: mobile
---

## Style Scope

This guide is self-contained. Apply its palette, radius scale, spacing, shadows, and component treatments only when this exact guide is selected; do not borrow food, social, dashboard, luxury, terminal, or other guide-specific patterns into another style. Treat unnamed layout frames as structural by default: no fill, stroke, cornerRadius, or shadow unless the node is intentionally a card, input, button, badge, media mask, navigation surface, or other visible component. Avoid decorative wrapper shells around components; hierarchy should come from spacing, typography, and this guide's explicit surfaces.

## Style Summary

A crisp, professional mobile interface inspired by iOS design language. Pure white (#FFFFFF) backgrounds provide maximum clarity, while a confident blue accent (#2563EB) conveys trust and reliability across interactive elements. SF Pro serves as both heading and body typeface, leveraging Apple's system font for native-feeling readability at every size. Comfortable 12-16px corner radii keep containers approachable without being playful, and the overall aesthetic prioritizes content legibility and intuitive navigation.

Key aesthetics:

- **Pure white canvas**: #FFFFFF background with subtle gray surfaces for depth and hierarchy
- **Trusted blue accent**: #2563EB as the sole accent, conveying professionalism and reliability
- **iOS-native typography**: SF Pro throughout, matching the platform's system font for seamless integration
- **Comfortable radii**: 12-16px corners feel natural and inviting without being bubbly
- **Integrated bottom nav**: full-width bottom navigation with blue active indicator
- **Clean separation**: Light borders (#E5E7EB) and subtle surfaces provide structure without visual weight

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                            |
| --------------- | ------- | ------------------------------------------------ |
| Page Background | #FFFFFF | Root screen background                           |
| Card Surface    | #FFFFFF | Cards, list containers                           |
| Inset Surface   | #F3F4F6 | Search bars, input backgrounds, grouped sections |
| Subtle Surface  | #F9FAFB | Alternating list rows, secondary areas           |
| Tab Bar Surface | #FFFFFF | Integrated bottom navigation surface                      |

### Text Colors

| Token          | Value   | Usage                                   |
| -------------- | ------- | --------------------------------------- |
| Primary Text   | #111827 | Headings, metric values, primary labels |
| Secondary Text | #6B7280 | Body text, descriptions                 |
| Tertiary Text  | #9CA3AF | Captions, timestamps, placeholders      |
| Muted Text     | #D1D5DB | Disabled text, background labels        |
| Accent Text    | #2563EB | Active tabs, links, highlighted values  |

### Border Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Default Border | #E5E7EB | Card borders, input outlines, dividers |
| Subtle Border  | #F3F4F6 | Light separators, section breaks       |
| Active Border  | #2563EB | Focused inputs, active states          |

### Accent Colors

| Token          | Value     | Usage                                         |
| -------------- | --------- | --------------------------------------------- |
| Primary Accent | #2563EB   | Active states, primary buttons, tab indicator |
| Accent Light   | #2563EB15 | Tinted backgrounds, selected row highlights   |
| Accent Muted   | #DBEAFE   | Badge backgrounds, subtle indicators          |
| Success        | #16A34A   | Positive metrics, completed states            |
| Warning        | #F59E0B   | Alert indicators, pending states              |
| Error          | #EF4444   | Error states, destructive actions             |

## Typography

### Font Families

| Role              | Family | Usage                                         |
| ----------------- | ------ | --------------------------------------------- |
| Display / Heading | SF Pro | Screen titles, section headings, hero metrics |
| Body / Functional | SF Pro | Body text, labels, buttons, navigation        |

### Type Scale

| Level   | Size | Font   | Weight | Usage                                |
| ------- | ---- | ------ | ------ | ------------------------------------ |
| Display | 34px | SF Pro | 700    | Hero metric values, large titles     |
| Title 1 | 24px | SF Pro | 700    | Screen titles                        |
| Title 2 | 20px | SF Pro | 600    | Section headings                     |
| Title 3 | 17px | SF Pro | 600    | Card titles, list headings           |
| Body    | 15px | SF Pro | 400    | Body text, descriptions              |
| Label   | 13px | SF Pro | 500    | Field labels, button text            |
| Caption | 12px | SF Pro | 400    | Timestamps, secondary info           |
| Small   | 11px | SF Pro | 500    | Badges, auxiliary labels             |
| Micro   | 10px | SF Pro | 500    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight   | Value | Usage                             |
| -------- | ----- | --------------------------------- |
| Regular  | 400   | Body text, descriptions, captions |
| Medium   | 500   | Labels, buttons, navigation items |
| Semibold | 600   | Section headings, card titles     |
| Bold     | 700   | Screen titles, hero metrics       |

### Letter Spacing

- Display (34px): -0.5px
- Section headings (17-24px): -0.3px
- Uppercase tab labels: +1px
- Body text: 0px

### Line Height

- Display (34px): 1.0
- Headings (17-24px): 1.2
- Body (15px): 1.5
- Captions (10-12px): 1.4

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
| 4px              | Integrated tab bar inner padding               |
| [8, 16]          | Input fields, search bars                |
| [10, 20]         | Standard buttons                         |
| [12, 24]         | Large buttons, CTA buttons               |
| 16px             | Compact card padding                     |
| 20px             | Standard card padding                    |
| 24px             | Large section padding                    |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24
- Status bar: 62px, standard iOS
- Integrated tab bar: height 62-72px, cornerRadius 0-12, padding 0-4, full-width in page flow
- Tab items: fill_container, vertical, gap 4, center aligned
- Grouped sections: inset surface background with 12px radius containers

### Mobile Composition Guardrails

- Keep top rhythm compact: status bar, header, and first content block should feel connected; avoid empty 80px+ bands unless the design is intentionally image-led.
- Search should be one clean row: a neutral input plus optional filter button. Do not wrap the input in a second tinted rounded shell; the grouping frame should stay structural and transparent.
- Horizontal rails and carousel viewport frames stay transparent: no white rounded backing frame, no extra shadow. Only individual cards, images, or intentional promo panels receive fill, radius, or elevation.
- Favorite, like, bookmark, and heart actions on image cards should be icon-only overlays or very subtle translucent hit areas; do not add circular white bubbles, borders, or shadows around the heart.
- Bottom navigation is integrated into the screen flow with a quiet divider or active indicator; avoid floating capsule navigation unless a prompt explicitly asks for a floating nav pattern.
- Use 9999px radius only for true circles or tiny capsules such as avatars, status dots, and short badges; never for page sections, search shells, carousels, card action buttons, or full navigation bars.

## Corner Radius

| Value | Usage                                        | Rationale                         |
| ----- | -------------------------------------------- | --------------------------------- |
| 8px   | Small buttons, compact badges, tag chips     | Subtle iOS-style softening        |
| 12px  | Standard cards, list containers, inputs      | Primary container radius          |
| 14px  | Large cards, search bars, grouped sections   | Comfortable interactive radius    |
| 16px  | Modal sheets, feature cards, hero containers | Maximum standard radius           |
| 9999px | Avatars, status dots, short badges only | True circles/capsules for semantic micro-elements; never navigation, search, carousels, or card action buttons |

Design rationale: The 8-16px range mirrors iOS design conventions, where corners are soft enough to feel friendly but restrained enough to maintain professionalism. Every interactive element feels natural on a mobile device, with the progressive scale reinforcing visual hierarchy. The bottom navigation should feel integrated with the page palette, using accent color or a tiny indicator for active state.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (clean strokes complement the minimal aesthetic)

### Commonly Used Icons

home, search, bell, user, settings, plus, check, x, chevron-right, chevron-left, heart, star, calendar, clock, mail, phone, camera, share, bookmark, filter, arrow-up, arrow-down, circle, map-pin

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
| Active    | #2563EB | Selected tab icon, active navigation |
| Default   | #9CA3AF | Inactive tabs, secondary actions     |
| Muted     | #D1D5DB | Disabled states, placeholder icons   |
| On Accent | #FFFFFF | Icons on blue-colored backgrounds    |
