---
name: 'startup-gradient-dark'
tags: [gradient, dark-mode, startup, bold-typography, vibrant, mesh-gradient, landing-page]
platform: webapp
---

## Style Summary

A bold, high-energy startup landing page with a deep space-dark background (#0F0F1A) and vibrant dual-accent gradients blending violet (#7C3AED) and teal (#2DD4BF). This style captures the ambition and momentum of modern tech startups — confident, forward-looking, and unapologetically bold. Clash Display headlines at 700-800 weight deliver punchy, oversized statements, while Inter body text maintains clarity and professionalism. Mesh gradient backgrounds on hero sections and CTA blocks create depth and visual intrigue.

Key aesthetics:

- **Deep space background**: #0F0F1A base with subtle gradient overlays for depth and atmosphere
- **Dual-accent gradients**: Violet (#7C3AED) to teal (#2DD4BF) for hero backgrounds, buttons, and accent elements
- **Oversized display type**: Clash Display at 700-800 weight, 56-72px hero headlines for maximum impact
- **Generous radii**: 16px cards, 24px buttons create a modern, approachable feel
- **Mesh gradient hero**: Complex gradient backgrounds on hero and CTA sections for visual depth
- **Section-based flow**: Full-width sections stacked vertically with alternating background treatments

## Color System

### Core Backgrounds

| Token               | Value   | Usage                              |
| ------------------- | ------- | ---------------------------------- |
| Page Background     | #0F0F1A | Root page background (deep space)  |
| Section Alt         | #141428 | Alternating section background     |
| Card Surface        | #1A1A2E | Feature cards, pricing cards       |
| Elevated Surface    | #222240 | Elevated containers, modals        |
| Hero Gradient Start | #7C3AED | Violet start of hero mesh gradient |
| Hero Gradient End   | #2DD4BF | Teal end of hero mesh gradient     |

### Text Colors

| Token              | Value   | Usage                                 |
| ------------------ | ------- | ------------------------------------- |
| Primary Text       | #FFFFFF | Headlines, hero copy, primary labels  |
| Secondary Text     | #B8B8D0 | Body text, descriptions, feature copy |
| Tertiary Text      | #7878A0 | Captions, metadata, footer links      |
| Muted Text         | #4A4A6A | Placeholders, disabled text           |
| Accent Text Violet | #A78BFA | Highlighted labels, badge text        |
| Accent Text Teal   | #2DD4BF | Links, active nav, success states     |

### Border Colors

| Token          | Value     | Usage                             |
| -------------- | --------- | --------------------------------- |
| Default Border | #2A2A45   | Card borders, dividers            |
| Subtle Border  | #1E1E38   | Section separators                |
| Active Border  | #7C3AED   | Focused inputs, active indicators |
| Glow Border    | #7C3AED40 | Subtle violet glow on hover       |

### Accent Colors

| Token            | Value                    | Usage                                        |
| ---------------- | ------------------------ | -------------------------------------------- |
| Primary Accent   | #7C3AED                  | Buttons, active states, violet spectrum      |
| Secondary Accent | #2DD4BF                  | Secondary buttons, highlights, teal spectrum |
| Gradient Fill    | linear(#7C3AED, #2DD4BF) | CTA buttons, hero backgrounds, badges        |
| Accent Muted     | #7C3AED20                | Tinted card backgrounds                      |
| Success          | #2DD4BF                  | Positive indicators, checkmarks              |
| Error            | #EF4444                  | Error states                                 |

## Typography

### Font Families

| Role              | Family        | Usage                                        |
| ----------------- | ------------- | -------------------------------------------- |
| Display           | Clash Display | Hero headlines, section headings, CTA text   |
| Body / Functional | Inter         | Body text, navigation, buttons, descriptions |

### Type Scale

| Level      | Size | Font          | Weight | Usage                                         |
| ---------- | ---- | ------------- | ------ | --------------------------------------------- |
| Hero       | 72px | Clash Display | 800    | Primary hero headline                         |
| Display    | 56px | Clash Display | 700    | Secondary hero text, large section headings   |
| Title 1    | 40px | Clash Display | 700    | Section headings                              |
| Title 2    | 28px | Clash Display | 600    | Subsection headings, feature titles           |
| Title 3    | 20px | Inter         | 600    | Card titles, pricing plan names               |
| Body Large | 18px | Inter         | 400    | Hero subtitle, lead paragraphs                |
| Body       | 16px | Inter         | 400    | Descriptions, feature copy                    |
| Label      | 14px | Inter         | 500    | Navigation items, button text                 |
| Caption    | 12px | Inter         | 500    | Badges, metadata, overline labels (uppercase) |
| Micro      | 11px | Inter         | 500    | Footer links, fine print                      |

### Font Weights

| Weight     | Value | Usage                            |
| ---------- | ----- | -------------------------------- |
| Regular    | 400   | Body text, descriptions          |
| Medium     | 500   | Labels, navigation, buttons      |
| Semibold   | 600   | Subsection headings, card titles |
| Bold       | 700   | Section headings, display text   |
| Extra Bold | 800   | Hero headline                    |

### Letter Spacing

- Hero (72px): -3px (dramatic compression)
- Display (56px): -2px
- Section headings (40px): -1.5px
- Subsection (28px): -0.5px
- Uppercase labels: +2px
- Body text: 0px

### Line Height

- Hero (72px): 0.95
- Display (56px): 1.0
- Headings (28-40px): 1.15
- Body (16-18px): 1.6
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value   | Usage                                     |
| ------- | ----------------------------------------- |
| 4px     | Inline icon+text pairs                    |
| 8px     | Badge groups, tight element clusters      |
| 12px    | Button groups, nav items                  |
| 16px    | Card content sections, feature list items |
| 24px    | Between cards in grid, form fields        |
| 32px    | Section internal gaps                     |
| 48px    | Between content blocks within sections    |
| 64-80px | Between page sections, hero gap           |

### Padding Scale

| Value    | Usage                                 |
| -------- | ------------------------------------- |
| [10, 20] | Compact buttons, input fields         |
| [14, 28] | Standard buttons                      |
| [16, 32] | CTA buttons (generous)                |
| 24px     | Card padding (compact)                |
| 32px     | Card padding (standard)               |
| 40px     | Feature card padding                  |
| [80, 0]  | Section vertical padding (full-width) |
| [120, 0] | Hero section vertical padding         |

### Layout Pattern

- Page width: 1440px max
- Content container: 1200px centered, padding [0, 24]
- Hero section: full-width gradient background, centered content max-width 800px
- Feature grid: 3-column, gap 24px
- Pricing grid: 3-column, gap 24px, center card highlighted
- CTA section: full-width gradient, centered text max-width 600px
- Navigation: fixed top, full-width, height 72px, transparent on hero

## Corner Radius

| Value  | Usage                               | Rationale                 |
| ------ | ----------------------------------- | ------------------------- |
| 8px    | Input fields, small badges          | Subtle softening          |
| 12px   | Dropdown menus, tooltips            | Functional rounding       |
| 16px   | Feature cards, pricing cards        | Primary container radius  |
| 20px   | Testimonial cards, large containers | Generous rounding         |
| 24px   | CTA buttons, hero cards             | Bold, modern button shape |
| 9999px | Pill badges, toggle switches        | Full capsule              |

Design rationale: Generous radii (16-24px) project a modern, approachable startup personality. The 24px CTA buttons feel bold and tappable, while 16px cards create a cohesive, friendly container language. This contrasts with the sharp, architectural zero-radius of luxury brands, signaling accessibility and warmth despite the dark palette.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

arrow-right, check, star, zap, shield, lock, rocket, sparkles, users, bar-chart-2, globe, code, layers, play, chevron-right, chevron-down, menu, x, mail, github, twitter, linkedin

### Icon Sizes

| Size    | Usage                                    |
| ------- | ---------------------------------------- |
| 16px    | Inline with body text, list item bullets |
| 20px    | Feature list icons, nav items            |
| 24px    | Card header icons, CTA arrows            |
| 32-48px | Feature section icons, hero decorative   |

### Icon Color States

| State       | Color   | Usage                             |
| ----------- | ------- | --------------------------------- |
| Active      | #7C3AED | Primary actions, violet accent    |
| Highlight   | #2DD4BF | Success states, teal accent       |
| Default     | #B8B8D0 | Standard icons, secondary actions |
| Muted       | #4A4A6A | Disabled, placeholder icons       |
| On Gradient | #FFFFFF | Icons on gradient backgrounds     |
