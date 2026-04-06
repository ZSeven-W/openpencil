---
name: 'healthcare-trust-light'
tags: [clean, light-mode, professional, blue-accent, calm, landing-page, rounded]
platform: webapp
---

## Style Summary

A trustworthy, calm healthcare SaaS landing page built on a soft sky-tinted (#F0F9FF) foundation with sky blue (#0284C7) as the singular accent color. This style captures the reliability and professionalism of modern health technology — clean, authoritative, and reassuring. DM Sans serves as both heading and body font, its geometric openness projecting clarity and approachability critical in healthcare communication. The cool blue palette and gentle radii create an environment that feels clinical in its precision but human in its warmth.

Key aesthetics:

- **Soft sky base**: #F0F9FF background with #FFFFFF card surfaces, a subtle blue tint signals trust and calm
- **Single sky blue accent**: #0284C7 for all CTAs, active states, and key indicators — calm and professional
- **Unified DM Sans type**: DM Sans throughout, clean geometric letterforms at 600 headings, 400 body
- **Gentle radii**: 12-16px corners create a soft, approachable container language
- **Trust signals**: Designed for compliance badges, security indicators, and credentialing
- **Information hierarchy**: Clear section flow for complex healthcare content

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                        |
| --------------- | ------- | -------------------------------------------- |
| Page Background | #F0F9FF | Root page background (soft sky tint)         |
| Section Alt     | #E0F2FE | Alternating section background (deeper sky)  |
| Card Surface    | #FFFFFF | Feature cards, pricing cards, content panels |
| Inset Surface   | #F0F4F8 | Input backgrounds, secondary panels          |

### Text Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Primary Text   | #0C4A6E | Headlines, primary content (deep navy) |
| Secondary Text | #475569 | Body text, descriptions                |
| Tertiary Text  | #94A3B8 | Captions, metadata, helper text        |
| Muted Text     | #CBD5E1 | Placeholders, disabled text            |
| Accent Text    | #0284C7 | Links, active nav, highlighted values  |

### Border Colors

| Token          | Value   | Usage                         |
| -------------- | ------- | ----------------------------- |
| Default Border | #E2E8F0 | Card borders, dividers        |
| Subtle Border  | #F1F5F9 | Light separators              |
| Active Border  | #0284C7 | Focused inputs, active states |

### Accent Colors

| Token          | Value     | Usage                                     |
| -------------- | --------- | ----------------------------------------- |
| Primary Accent | #0284C7   | CTA buttons, active states, links         |
| Accent Light   | #E0F2FE   | Tinted backgrounds, badge fills           |
| Accent Muted   | #0284C720 | Subtle hover backgrounds                  |
| Accent Dark    | #0369A1   | Button hover/pressed state                |
| Success        | #059669   | Positive health indicators, secure badges |
| Error          | #DC2626   | Error states, critical alerts             |
| Warning        | #D97706   | Warning indicators, caution states        |

## Typography

### Font Families

| Role              | Family  | Usage                                   |
| ----------------- | ------- | --------------------------------------- |
| Headings          | DM Sans | All headings, section titles, hero text |
| Body / Functional | DM Sans | Body text, navigation, buttons, labels  |

### Type Scale

| Level      | Size | Font    | Weight | Usage                                 |
| ---------- | ---- | ------- | ------ | ------------------------------------- |
| Hero       | 52px | DM Sans | 600    | Primary hero headline                 |
| Display    | 40px | DM Sans | 600    | Large section headings                |
| Title 1    | 32px | DM Sans | 600    | Section headings                      |
| Title 2    | 24px | DM Sans | 600    | Subsection headings, feature titles   |
| Title 3    | 20px | DM Sans | 600    | Card titles, list headings            |
| Body Large | 18px | DM Sans | 400    | Hero subtitle, lead paragraphs        |
| Body       | 16px | DM Sans | 400    | Descriptions, feature copy            |
| Label      | 14px | DM Sans | 500    | Navigation, button text, field labels |
| Caption    | 12px | DM Sans | 500    | Overline labels (uppercase), badges   |
| Micro      | 11px | DM Sans | 400    | Fine print, compliance text, footer   |

### Font Weights

| Weight   | Value | Usage                                 |
| -------- | ----- | ------------------------------------- |
| Regular  | 400   | Body text, descriptions               |
| Medium   | 500   | Labels, navigation, buttons, captions |
| Semibold | 600   | All headings, section titles          |

### Letter Spacing

- Hero (52px): -1.5px
- Display (40px): -1px
- Section headings (32px): -0.5px
- Subsection (24px): -0.3px
- Uppercase overlines: +2px
- Body text: 0px

### Line Height

- Hero (52px): 1.1
- Display (40px): 1.15
- Headings (24-32px): 1.25
- Body (16-18px): 1.6
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                       |
| ----- | ------------------------------------------- |
| 4px   | Inline icon+text pairs                      |
| 8px   | Badge groups, tight clusters                |
| 12px  | Button groups, nav items, compliance badges |
| 16px  | Card content sections, feature items        |
| 20px  | Between feature list items                  |
| 24px  | Between cards in grid, form groups          |
| 32px  | Section internal content gaps               |
| 48px  | Between content blocks within a section     |
| 64px  | Between page sections                       |
| 80px  | Major section breaks                        |

### Padding Scale

| Value    | Usage                         |
| -------- | ----------------------------- |
| [8, 16]  | Compact buttons, input fields |
| [12, 24] | Standard buttons              |
| [14, 28] | CTA buttons                   |
| 24px     | Card padding (compact)        |
| 32px     | Card padding (standard)       |
| 40px     | Feature card padding          |
| [64, 0]  | Section vertical padding      |
| [96, 0]  | Hero section vertical padding |

### Layout Pattern

- Page width: 1280px max
- Content container: 1120px centered, padding [0, 32]
- Hero section: split layout — copy + CTA left, product screenshot right
- Feature grid: 3-column with icons, gap 24px
- Trust bar: compliance badges (HIPAA, SOC2), security certifications row
- Testimonial section: healthcare professional quotes with credentials
- Pricing: 3-column, gap 24px, recommended plan with blue border
- Navigation: fixed top, white, height 64px, subtle shadow
- Footer: 4-column link grid + compliance section + bottom bar

## Corner Radius

| Value  | Usage                                | Rationale                         |
| ------ | ------------------------------------ | --------------------------------- |
| 6px    | Input fields, small badges           | Minimal functional rounding       |
| 8px    | Buttons, dropdown menus              | Professional, clickable shape     |
| 12px   | Feature cards, testimonial cards     | Soft container radius             |
| 16px   | Pricing cards, hero image containers | Primary generous rounding         |
| 9999px | Pill badges, compliance badges       | Full capsule for trust indicators |

Design rationale: The 12-16px range strikes the balance between clinical precision and human warmth that healthcare products require. Cards at 12-16px feel trustworthy and modern without being playful. Buttons at 8px project professionalism while remaining approachable. The gentle rounding softens the clinical subject matter, making complex health information feel accessible and non-intimidating.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

shield-check, lock, heart-pulse, stethoscope, activity, check, arrow-right, chevron-right, chevron-down, users, bar-chart-2, calendar, clock, phone, mail, video, file-text, clipboard, star, award, menu, x, external-link, download, eye

### Icon Sizes

| Size | Usage                                        |
| ---- | -------------------------------------------- |
| 16px | Inline with body text, compliance checkmarks |
| 20px | Navigation items, button icons               |
| 24px | Feature card icons, section indicators       |
| 32px | Feature grid icons (in light blue circle)    |

### Icon Color States

| State     | Color   | Usage                              |
| --------- | ------- | ---------------------------------- |
| Active    | #0284C7 | Primary actions, links, selected   |
| Default   | #475569 | Standard icons, secondary actions  |
| Muted     | #94A3B8 | Disabled, placeholder icons        |
| On Accent | #FFFFFF | Icons on blue backgrounds          |
| Success   | #059669 | Security verified, health positive |
