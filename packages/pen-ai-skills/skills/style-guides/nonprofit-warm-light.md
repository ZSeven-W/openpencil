---
name: 'nonprofit-warm-light'
tags: [warm-tones, light-mode, friendly, organic, rounded, landing-page, earth-tones]
platform: webapp
---

## Style Summary

An empathy-driven nonprofit landing page built on a warm parchment (#FFFBF5) foundation with red (#DC2626) and green (#16A34A) as dual accents representing urgency and hope. This style captures the compassion and authenticity of mission-driven organizations — human, trustworthy, and emotionally resonant. Merriweather serif headlines at 700 weight bring gravitas and editorial authority, while Source Sans 3 body text provides clean, highly readable content delivery. The warm earthy palette and organic radii create an inviting atmosphere that encourages connection and action.

Key aesthetics:

- **Warm parchment base**: #FFFBF5 background with #FEF3E2 for highlighted sections, earthy warmth throughout
- **Dual-purpose accents**: Red (#DC2626) for urgent CTAs (Donate Now), green (#16A34A) for positive outcomes and impact
- **Serif editorial display**: Merriweather at 700 weight for authoritative, story-driven headlines
- **Organic radii**: 12-16px rounded corners create a soft, approachable feel
- **Story-first layout**: Large hero images, testimonial blocks, and impact statistics
- **Earth-toned neutrals**: Warm brown undertones in all neutral colors reinforce organic identity

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                       |
| --------------- | ------- | ------------------------------------------- |
| Page Background | #FFFBF5 | Root page background (warm parchment)       |
| Section Alt     | #FEF3E2 | Alternating section background (warm cream) |
| Card Surface    | #FFFFFF | Impact cards, testimonials, feature cards   |
| Inset Surface   | #FDF6EC | Callout blocks, quote backgrounds           |

### Text Colors

| Token          | Value   | Usage                               |
| -------------- | ------- | ----------------------------------- |
| Primary Text   | #292524 | Headlines, primary content          |
| Secondary Text | #57534E | Body text, descriptions, story copy |
| Tertiary Text  | #A8A29E | Captions, metadata, dates           |
| Muted Text     | #D6D3D1 | Placeholders, disabled text         |
| Red Text       | #DC2626 | Urgent labels, donation amounts     |
| Green Text     | #16A34A | Impact metrics, positive outcomes   |

### Border Colors

| Token          | Value   | Usage                               |
| -------------- | ------- | ----------------------------------- |
| Default Border | #E7E5E4 | Card borders, dividers              |
| Subtle Border  | #F5F5F4 | Light separators                    |
| Active Border  | #DC2626 | Focused inputs, active states       |
| Success Border | #16A34A | Completed goals, success indicators |

### Accent Colors

| Token            | Value   | Usage                              |
| ---------------- | ------- | ---------------------------------- |
| Primary Accent   | #DC2626 | Donate buttons, urgent CTAs        |
| Secondary Accent | #16A34A | Impact indicators, success states  |
| Red Light        | #FEF2F2 | Urgent highlight backgrounds       |
| Green Light      | #F0FDF4 | Impact highlight backgrounds       |
| Red Dark         | #B91C1C | Donate button hover state          |
| Green Dark       | #15803D | Success hover state                |
| Warm Accent      | #D97706 | Secondary highlights, amber warmth |
| Error            | #DC2626 | Error states                       |

## Typography

### Font Families

| Role              | Family        | Usage                                          |
| ----------------- | ------------- | ---------------------------------------------- |
| Headings          | Merriweather  | Hero headlines, section headings, story titles |
| Body / Functional | Source Sans 3 | Body text, navigation, buttons, descriptions   |

### Type Scale

| Level      | Size | Font          | Weight | Usage                                 |
| ---------- | ---- | ------------- | ------ | ------------------------------------- |
| Hero       | 52px | Merriweather  | 700    | Primary hero headline                 |
| Display    | 40px | Merriweather  | 700    | Large section headings                |
| Title 1    | 32px | Merriweather  | 700    | Section headings                      |
| Title 2    | 24px | Merriweather  | 700    | Subsection headings, story titles     |
| Title 3    | 20px | Source Sans 3 | 600    | Card titles, list headings            |
| Body Large | 18px | Source Sans 3 | 400    | Hero subtitle, lead paragraphs        |
| Body       | 16px | Source Sans 3 | 400    | Descriptions, story copy              |
| Label      | 14px | Source Sans 3 | 600    | Navigation, button text, field labels |
| Caption    | 12px | Source Sans 3 | 600    | Overline labels (uppercase), badges   |
| Micro      | 11px | Source Sans 3 | 400    | Fine print, footer secondary          |

### Font Weights

| Weight   | Value | Usage                                    |
| -------- | ----- | ---------------------------------------- |
| Regular  | 400   | Body text, descriptions                  |
| Semibold | 600   | Card titles, labels, navigation, buttons |
| Bold     | 700   | All Merriweather headings                |

### Letter Spacing

- Hero (52px): -1.5px
- Display (40px): -1px
- Section headings (32px): -0.5px
- Subsection (24px): -0.3px
- Uppercase overlines: +2px
- Body text: 0px

### Line Height

- Hero (52px): 1.15
- Display (40px): 1.2
- Headings (24-32px): 1.3
- Body (16-18px): 1.7 (generous for long-form reading)
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                  |
| ----- | -------------------------------------- |
| 4px   | Inline icon+text pairs                 |
| 8px   | Badge groups, tight clusters           |
| 12px  | Button groups, nav items               |
| 16px  | Card content sections, list items      |
| 20px  | Between feature items, testimonials    |
| 24px  | Between cards in grid, form fields     |
| 32px  | Section internal content gaps          |
| 48px  | Between content blocks within sections |
| 64px  | Between page sections                  |
| 80px  | Major section breaks                   |

### Padding Scale

| Value    | Usage                         |
| -------- | ----------------------------- |
| [10, 20] | Compact buttons               |
| [12, 24] | Standard buttons              |
| [14, 32] | Donate CTA buttons (generous) |
| 24px     | Card padding (compact)        |
| 32px     | Card padding (standard)       |
| 40px     | Testimonial card padding      |
| [64, 0]  | Section vertical padding      |
| [96, 0]  | Hero section vertical padding |

### Layout Pattern

- Page width: 1280px max
- Content container: 1080px centered, padding [0, 40]
- Hero section: full-width image background with overlaid text, max-width 700px
- Impact stats: 3-4 column row, large numbers with labels
- Story grid: 2-column, text + image alternating sides
- Testimonial: large quote block, centered, with photo
- Donation CTA: full-width warm background, centered content
- Navigation: fixed top, warm white, height 64px
- Footer: 4-column link grid + bottom bar with legal

## Corner Radius

| Value  | Usage                               | Rationale                   |
| ------ | ----------------------------------- | --------------------------- |
| 6px    | Input fields, small badges          | Minimal functional rounding |
| 8px    | Buttons, tag pills                  | Friendly button shape       |
| 12px   | Feature cards, testimonial cards    | Warm container radius       |
| 16px   | Impact cards, donation blocks       | Primary generous rounding   |
| 20px   | Hero image containers, CTA sections | Soft, organic feel          |
| 9999px | Pill badges, donation amount pills  | Full capsule for selectors  |

Design rationale: The 12-16px range creates a warm, human-centered feel that reflects nonprofit values of approachability and compassion. Generous radii soften the interface and make serious subject matter more accessible. The organic rounding contrasts with sharp corporate aesthetics, signaling authenticity and grassroots connection.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

heart, hand-heart, users, globe, shield, check, arrow-right, chevron-right, chevron-down, map-pin, calendar, mail, phone, share-2, download, play-circle, target, trending-up, bar-chart-2, star, menu, x, twitter, facebook, instagram

### Icon Sizes

| Size | Usage                                  |
| ---- | -------------------------------------- |
| 16px | Inline with body text, list checkmarks |
| 20px | Navigation items, button icons         |
| 24px | Card icons, section indicators         |
| 32px | Impact section icons (in warm circle)  |

### Icon Color States

| State    | Color   | Usage                               |
| -------- | ------- | ----------------------------------- |
| Red      | #DC2626 | Urgent actions, donate, heart icons |
| Green    | #16A34A | Impact indicators, success states   |
| Default  | #57534E | Standard icons, secondary actions   |
| Muted    | #A8A29E | Disabled, placeholder icons         |
| On Red   | #FFFFFF | Icons on red backgrounds            |
| On Green | #FFFFFF | Icons on green backgrounds          |
