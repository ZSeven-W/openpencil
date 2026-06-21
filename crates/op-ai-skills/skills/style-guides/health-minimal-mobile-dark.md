---
name: 'health-minimal-mobile-dark'
tags: [wellness, dark-mode, minimal, calm, sage-green, mobile, quiet]
platform: mobile
---

## Style Scope

This guide is self-contained. Apply its palette, radius scale, spacing, shadows, and component treatments only when this exact guide is selected; do not borrow food, social, dashboard, luxury, terminal, or other guide-specific patterns into another style. Treat unnamed layout frames as structural by default: no fill, stroke, cornerRadius, or shadow unless the node is intentionally a card, input, button, badge, media mask, navigation surface, or other visible component. Avoid decorative wrapper shells around components; hierarchy should come from spacing, typography, and this guide's explicit surfaces.

## Style Summary

A calm, focused dark mobile interface designed for health tracking, fitness monitoring, and wellness applications. The deep charcoal background (#111111) creates a restful environment that reduces eye strain during early-morning or late-night health check-ins, while the green accent (#22C55E) signals vitality and positive progress without the clinical coldness of blue or the alarm of red. Inter's neutral, unadorned letterforms provide maximum data readability—essential when users are scanning heart rates, step counts, and sleep metrics at a glance. The restrained 12-16px corner radii and sparse use of color reinforce the minimal philosophy: every element earns its place through function, not decoration.

Key aesthetics:

- **Restful dark canvas**: #111111 background minimizes eye strain and focuses attention on health data
- **Vital green accent**: #22C55E represents growth, health, and positive progress
- **Invisible type**: Inter at medium weights—so readable it disappears, letting data speak
- **Minimal radii**: 12-16px corners are clean and functional without unnecessary roundness
- **Data clarity**: Generous whitespace around metrics, clear hierarchy for at-a-glance reading
- **Integrated bottom nav**: full-width bottom navigation with green active indicator

## Color System

### Core Backgrounds

| Token            | Value   | Usage                                  |
| ---------------- | ------- | -------------------------------------- |
| Page Background  | #111111 | Root screen background (deep charcoal) |
| Card Surface     | #1A1A1A | Standard cards, metric containers      |
| Elevated Surface | #222222 | Elevated cards, modal sheets           |
| Subtle Surface   | #161616 | Inset areas, grouped sections          |
| Tab Bar Surface  | #1A1A1A | Integrated bottom navigation surface            |

### Text Colors

| Token          | Value   | Usage                                         |
| -------------- | ------- | --------------------------------------------- |
| Primary Text   | #F5F5F5 | Headings, metric values, primary labels       |
| Secondary Text | #A3A3A3 | Body text, descriptions                       |
| Tertiary Text  | #6B6B6B | Timestamps, captions, inactive labels         |
| Muted Text     | #3B3B3B | Placeholders, disabled text                   |
| Accent Text    | #22C55E | Active states, positive values, goal progress |

### Border Colors

| Token          | Value   | Usage                                |
| -------------- | ------- | ------------------------------------ |
| Default Border | #2A2A2A | Card borders, dividers               |
| Subtle Border  | #1E1E1E | Light separators                     |
| Active Border  | #22C55E | Focused inputs, active tab indicator |

### Accent Colors

| Token          | Value     | Usage                                                        |
| -------------- | --------- | ------------------------------------------------------------ |
| Primary Accent | #22C55E   | Active states, primary buttons, progress bars, tab indicator |
| Accent Light   | #22C55E15 | Tinted backgrounds, selected row highlights                  |
| Accent Muted   | #22C55E20 | Subtle badge fills, progress track background                |
| Accent On-Dark | #111111   | Text/icons on green backgrounds (dark)                       |
| Success        | #22C55E   | Goal met, workout complete (same as accent)                  |
| Warning        | #F59E0B   | Resting too long, hydration reminder                         |
| Error          | #EF4444   | Missed workout, abnormal reading                             |
| Rest           | #8B5CF6   | Sleep tracking, recovery mode (soft violet)                  |

## Typography

### Font Families

| Role              | Family | Usage                                          |
| ----------------- | ------ | ---------------------------------------------- |
| Display / Heading | Inter  | Screen titles, section headings, metric labels |
| Body / Functional | Inter  | Body text, labels, buttons, navigation         |

### Type Scale

| Level   | Size | Font  | Weight | Usage                                 |
| ------- | ---- | ----- | ------ | ------------------------------------- |
| Display | 36px | Inter | 600    | Hero metric value (steps, heart rate) |
| Title 1 | 22px | Inter | 600    | Screen titles                         |
| Title 2 | 17px | Inter | 500    | Section headings                      |
| Title 3 | 15px | Inter | 500    | Card titles, metric categories        |
| Body    | 14px | Inter | 400    | Descriptions, notes, workout details  |
| Label   | 13px | Inter | 500    | Field labels, button text             |
| Data    | 14px | Inter | 600    | Metric values, stats, durations       |
| Caption | 12px | Inter | 400    | Timestamps, secondary info            |
| Small   | 11px | Inter | 500    | Badges, unit labels                   |
| Micro   | 10px | Inter | 500    | Tab labels (uppercase), micro badges  |

### Font Weights

| Weight   | Value | Usage                                          |
| -------- | ----- | ---------------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions              |
| Medium   | 500   | Labels, buttons, section headings, card titles |
| Semibold | 600   | Screen titles, hero metrics, data values       |

### Letter Spacing

- Display (36px): -0.5px
- Section headings (15-22px): -0.3px
- Uppercase tab labels: +1px
- Body text: 0px

### Line Height

- Display (36px): 1.0
- Headings (15-22px): 1.2
- Body (14px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                       |
| ----- | ------------------------------------------- |
| 2px   | Tight inline pairs                          |
| 4px   | Tab icon to label, unit label to value      |
| 6px   | Status indicator to text                    |
| 8px   | Compact metric groups, progress ring labels |
| 12px  | Between list items, metric cards in grid    |
| 16px  | Card internal sections                      |
| 20px  | Between cards in a list                     |
| 24px  | Section gaps within content                 |
| 32px  | Top-level screen section breaks             |

### Padding Scale

| Value            | Usage                                      |
| ---------------- | ------------------------------------------ |
| [0, 24]          | Screen content wrapper (horizontal only)   |
| [12, 21, 21, 21] | Tab bar section outer padding              |
| 4px              | Integrated tab bar inner padding                 |
| [8, 16]          | Input fields, compact buttons              |
| [10, 20]         | Standard buttons                           |
| [12, 24]         | Large buttons, CTA buttons                 |
| 16px             | Compact card padding, metric tiles         |
| 20px             | Standard card padding                      |
| 24px             | Feature card padding, hero metric sections |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24
- Status bar: 62px, standard iOS
- Integrated tab bar: height 62-72px, cornerRadius 0-12, padding 0-4, full-width in page flow
- Tab items: fill_container, vertical, gap 4, center aligned
- Metric grid: 2-column grid, 12px gap, equal-width tiles
- Progress rings: centered in card, value inside ring

### Mobile Composition Guardrails

- Keep top rhythm compact: status bar, header, and first content block should feel connected; avoid empty 80px+ bands unless the design is intentionally image-led.
- Search should be one clean row: a neutral input plus optional filter button. Do not wrap the input in a second tinted rounded shell; the grouping frame should stay structural and transparent.
- Horizontal rails and carousel viewport frames stay transparent: no white rounded backing frame, no extra shadow. Only individual cards, images, or intentional promo panels receive fill, radius, or elevation.
- Favorite, like, bookmark, and heart actions on image cards should be icon-only overlays or very subtle translucent hit areas; do not add circular white bubbles, borders, or shadows around the heart.
- Bottom navigation is integrated into the screen flow with a quiet divider or active indicator; avoid floating capsule navigation unless a prompt explicitly asks for a floating nav pattern.
- Use 9999px radius only for true circles or tiny capsules such as avatars, status dots, and short badges; never for page sections, search shells, carousels, card action buttons, or full navigation bars.

## Corner Radius

| Value | Usage                                            | Rationale                              |
| ----- | ------------------------------------------------ | -------------------------------------- |
| 8px   | Small buttons, compact badges, progress pills    | Minimal functional softening           |
| 12px  | Standard cards, metric containers, inputs        | Primary container radius               |
| 14px  | Large cards, grouped sections                    | Comfortable radius for data containers |
| 16px  | Hero cards, modal sheets, feature sections       | Maximum standard radius                |
| 9999px | Avatars, status dots, short badges only | True circles/capsules for semantic micro-elements; never navigation, search, carousels, or card action buttons |

Design rationale: Restrained 12-16px radii reflect the minimal, no-nonsense philosophy of a health tracking app. Users checking vital signs need clarity, not decoration—the tight radius range keeps containers functional and unobtrusive. The minimal spread (12-16px vs. the wider 12-24px of playful styles) reinforces the quiet confidence of a tool that prioritizes data over aesthetics.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (clean, minimal strokes)

### Commonly Used Icons

heart, activity, footprints, flame, droplet, moon, sun, dumbbell, timer, clock, target, trophy, apple, salad, bed, thermometer, wind, eye, scale, ruler, plus, minus, check, x, chevron-right, trending-up, trending-down, bar-chart-2

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline indicators, small badges                      |
| 18px | Tab bar icons, list item leading icons               |
| 20px | Card action icons, metric category icons             |
| 24px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                        |
| --------- | ------- | -------------------------------------------- |
| Active    | #22C55E | Selected tab, active states, green highlight |
| Default   | #6B6B6B | Inactive tabs, secondary actions             |
| Muted     | #3B3B3B | Disabled states, placeholder icons           |
| On Accent | #111111 | Icons on green-colored backgrounds (dark)    |
