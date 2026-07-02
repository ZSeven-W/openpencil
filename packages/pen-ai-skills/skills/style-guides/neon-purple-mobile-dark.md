---
name: 'neon-purple-mobile-dark'
tags: [neon, dark-mode, vibrant, electric, purple, bold-typography, mobile]
platform: mobile
---

## Style Summary

A high-intensity dark mobile interface built for gaming, entertainment, and nightlife apps. The deep void background (#0D001A) provides a cosmic canvas for the electric purple accent (#A855F7) to create a vivid neon glow effect. Space Grotesk brings geometric confidence to display typography at bold weights, while Inter provides maximum legibility for body content in low-contrast dark environments. Gradient surfaces and glow effects add depth and energy, and the 12-20px corner radii balance modern sharpness with comfortable touch targets.

Key aesthetics:

- **Deep void canvas**: #0D001A background creates an immersive dark space with purple undertones
- **Electric purple neon**: #A855F7 as the signature accent, with glow effects for a vivid nightclub feel
- **Geometric display type**: Space Grotesk at bold weights for confident, urban headings
- **Glow depth**: Purple-tinted shadows and translucent surfaces add layers of visual depth
- **Gradient cards**: Dark purple gradients on featured content for visual energy
- **Pill tab bar**: 100px radius bottom navigation with purple active glow

## Color System

### Core Backgrounds

| Token            | Value   | Usage                              |
| ---------------- | ------- | ---------------------------------- |
| Page Background  | #0D001A | Root screen background (deep void) |
| Card Surface     | #1A0D2E | Standard cards, list items         |
| Elevated Surface | #221340 | Elevated cards, modal sheets       |
| Gradient Start   | #1A0D2E | Gradient card dark edge            |
| Gradient End     | #2D1B4E | Gradient card lighter edge         |
| Tab Bar Surface  | #1A0D2E | Bottom navigation pill fill        |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #FFFFFF | Headings, metric values, primary labels  |
| Secondary Text | #B8A0D4 | Body text, descriptions                  |
| Tertiary Text  | #7A5FA0 | Timestamps, captions, inactive labels    |
| Muted Text     | #3D2660 | Placeholders, disabled text              |
| Accent Text    | #A855F7 | Active states, highlighted values, links |

### Border Colors

| Token          | Value     | Usage                                 |
| -------------- | --------- | ------------------------------------- |
| Default Border | #2D1B4E   | Card borders, dividers                |
| Subtle Border  | #221340   | Light separators                      |
| Active Border  | #A855F7   | Focused inputs, active tab indicator  |
| Glow Border    | #A855F740 | Subtle purple glow on active elements |

### Accent Colors

| Token          | Value     | Usage                                             |
| -------------- | --------- | ------------------------------------------------- |
| Primary Accent | #A855F7   | Active states, buttons, highlights, tab indicator |
| Accent Glow    | #A855F766 | Glow effects, status dot halo (40% opacity)       |
| Accent Muted   | #A855F720 | Tinted backgrounds, badge fills                   |
| Accent On-Dark | #FFFFFF   | Text/icons on purple backgrounds                  |
| Success        | #22C55E   | Win states, positive metrics                      |
| Warning        | #F59E0B   | Streak warnings, pending states                   |
| Error          | #EF4444   | Failed states, destructive actions                |
| Neon Cyan      | #06B6D4   | Secondary neon accent, alt highlights             |

## Typography

### Font Families

| Role              | Family        | Usage                                                |
| ----------------- | ------------- | ---------------------------------------------------- |
| Display           | Space Grotesk | Hero metrics, section headings, prominent titles     |
| Body / Functional | Inter         | Body text, labels, buttons, navigation, descriptions |

### Type Scale

| Level   | Size | Font          | Weight | Usage                                 |
| ------- | ---- | ------------- | ------ | ------------------------------------- |
| Hero    | 48px | Space Grotesk | 700    | Hero score, primary stat              |
| Display | 34px | Space Grotesk | 700    | Large metric values                   |
| Title 1 | 24px | Space Grotesk | 700    | Section headings                      |
| Title 2 | 18px | Space Grotesk | 600    | Card titles, list headings            |
| Title 3 | 15px | Inter         | 600    | Subsection headings, list item titles |
| Body    | 14px | Inter         | 400    | Descriptions, body text               |
| Label   | 13px | Inter         | 500    | Field labels, button text             |
| Caption | 12px | Inter         | 400    | Timestamps, metadata                  |
| Small   | 11px | Inter         | 500    | Badges, auxiliary labels              |
| Micro   | 10px | Inter         | 600    | Tab labels (uppercase), micro badges  |

### Font Weights

| Weight   | Value | Usage                                   |
| -------- | ----- | --------------------------------------- |
| Regular  | 400   | Body text, descriptions                 |
| Medium   | 500   | Labels, navigation, button text         |
| Semibold | 600   | Card titles, subsection headings        |
| Bold     | 700   | Section headings, hero metrics, display |

### Letter Spacing

- Hero/Display (34-48px): -1.5px (dramatic compression)
- Section headings (18-24px): -0.5px
- Uppercase tab labels: +1.5px
- Body text: 0px

### Line Height

- Hero (48px): 0.9
- Display (34px): 1.0
- Headings (18-24px): 1.2
- Body (14px): 1.5
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                 |
| ----- | ------------------------------------- |
| 2px   | Tight inline pairs                    |
| 4px   | Tab icon to label, inline icon groups |
| 6px   | Status indicator groups               |
| 8px   | Compact card content, game entries    |
| 12px  | Between list items, button groups     |
| 16px  | Card internal sections                |
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
| 24px             | Feature card padding, gradient cards     |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24-32
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Feature cards: gradient background (#1A0D2E to #2D1B4E), purple glow border

## Corner Radius

| Value | Usage                                             | Rationale                         |
| ----- | ------------------------------------------------- | --------------------------------- |
| 8px   | Small buttons, compact badges, input fields       | Subtle modern sharpness           |
| 12px  | Standard cards, list items, game entries          | Primary container radius          |
| 16px  | Feature cards, leaderboard sections               | Comfortable interactive radius    |
| 20px  | Hero cards, modal bottom sheets, large containers | Maximum standard radius           |
| 100px | Tab bar pill, toggle pills, round buttons         | Full capsule shape for navigation |

Design rationale: The 8-20px range balances modern geometric sharpness with comfortable mobile touch targets. Smaller radii (8-12px) keep the interface feeling technical and gaming-oriented, while larger radii (16-20px) on featured content add visual softness where attention should linger. The contrast between sharp secondary elements and soft featured ones creates visual hierarchy through geometry.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

gamepad-2, trophy, crown, star, zap, flame, sword, shield, target, users, message-circle, volume-2, play, pause, skip-forward, heart, thumbs-up, gift, gem, rocket, bell, search, settings, plus, x, chevron-right

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline metrics, badge indicators                     |
| 18px | Tab bar icons, list leading icons                    |
| 20px | Card action icons, game icons                        |
| 24px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                         |
| --------- | ------- | --------------------------------------------- |
| Active    | #A855F7 | Selected tab, active states, purple highlight |
| Default   | #B8A0D4 | Inactive tabs, secondary actions              |
| Muted     | #3D2660 | Disabled states, placeholder icons            |
| On Accent | #FFFFFF | Icons on purple-colored backgrounds           |
