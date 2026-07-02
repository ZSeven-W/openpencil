---
name: 'saas-clean-light'
tags: [clean, light-mode, minimal, blue-accent, modern, landing-page, professional]
platform: webapp
---

## Style Summary

A clean, professional SaaS marketing page built on a pure white (#FFFFFF) foundation with indigo (#4F46E5) as the singular accent color. This is the archetypal B2B SaaS landing page — trustworthy, organized, and conversion-focused. Inter serves as both heading and body font, creating typographic unity and eliminating visual noise. The restrained palette and consistent 12px card radius project reliability and competence without decorative excess.

Key aesthetics:

- **Pure white base**: #FFFFFF background with #F9FAFB for alternating sections, no tinted backgrounds
- **Single indigo accent**: #4F46E5 used for all CTAs, active states, and highlights — no secondary accent
- **Unified Inter type**: Inter for everything, differentiated by weight (600-700 headings, 400 body)
- **Functional radii**: 8px buttons, 12px cards — rounded enough to feel modern, sharp enough to feel professional
- **Structured sections**: Clear section hierarchy with consistent heading + subtitle + content pattern
- **Soft shadows**: Subtle elevation shadows on cards instead of heavy borders

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                       |
| --------------- | ------- | ------------------------------------------- |
| Page Background | #FFFFFF | Root page background (pure white)           |
| Section Alt     | #F9FAFB | Alternating section background (light gray) |
| Card Surface    | #FFFFFF | Cards on gray sections                      |
| Inset Surface   | #F3F4F6 | Input backgrounds, code blocks              |

### Text Colors

| Token          | Value   | Usage                                 |
| -------------- | ------- | ------------------------------------- |
| Primary Text   | #111827 | Headlines, primary content            |
| Secondary Text | #4B5563 | Body text, descriptions               |
| Tertiary Text  | #9CA3AF | Captions, metadata                    |
| Muted Text     | #D1D5DB | Placeholders, disabled text           |
| Accent Text    | #4F46E5 | Links, active nav, highlighted values |

### Border Colors

| Token          | Value   | Usage                         |
| -------------- | ------- | ----------------------------- |
| Default Border | #E5E7EB | Card borders, dividers        |
| Subtle Border  | #F3F4F6 | Light separators              |
| Active Border  | #4F46E5 | Focused inputs, active states |

### Accent Colors

| Token          | Value     | Usage                             |
| -------------- | --------- | --------------------------------- |
| Primary Accent | #4F46E5   | CTA buttons, active states, links |
| Accent Light   | #EEF2FF   | Tinted backgrounds, badge fills   |
| Accent Muted   | #4F46E520 | Subtle hover backgrounds          |
| Accent Dark    | #3730A3   | Button hover state                |
| Success        | #059669   | Positive indicators               |
| Error          | #DC2626   | Error states                      |
| Warning        | #D97706   | Warning indicators                |

## Typography

### Font Families

| Role              | Family | Usage                                   |
| ----------------- | ------ | --------------------------------------- |
| Headings          | Inter  | All headings, section titles, hero text |
| Body / Functional | Inter  | Body text, navigation, buttons, labels  |

### Type Scale

| Level      | Size | Font  | Weight | Usage                                 |
| ---------- | ---- | ----- | ------ | ------------------------------------- |
| Hero       | 56px | Inter | 700    | Primary hero headline                 |
| Display    | 48px | Inter | 700    | Large section headings                |
| Title 1    | 36px | Inter | 600    | Section headings                      |
| Title 2    | 24px | Inter | 600    | Subsection headings, feature titles   |
| Title 3    | 20px | Inter | 600    | Card titles, list headings            |
| Body Large | 18px | Inter | 400    | Hero subtitle, lead paragraphs        |
| Body       | 16px | Inter | 400    | Descriptions, feature copy            |
| Label      | 14px | Inter | 500    | Navigation, button text, field labels |
| Caption    | 12px | Inter | 500    | Overline labels (uppercase), badges   |
| Micro      | 11px | Inter | 400    | Fine print, footer secondary          |

### Font Weights

| Weight   | Value | Usage                                 |
| -------- | ----- | ------------------------------------- |
| Regular  | 400   | Body text, descriptions               |
| Medium   | 500   | Labels, navigation, buttons, captions |
| Semibold | 600   | Section headings, card titles         |
| Bold     | 700   | Hero headline, display text           |

### Letter Spacing

- Hero (56px): -2px
- Display (48px): -1.5px
- Section headings (36px): -1px
- Subsection (24px): -0.5px
- Uppercase overlines: +2px
- Body text: 0px

### Line Height

- Hero (56px): 1.05
- Display (48px): 1.1
- Headings (24-36px): 1.2
- Body (16-18px): 1.6
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                   |
| ----- | --------------------------------------- |
| 4px   | Inline icon+text pairs                  |
| 8px   | Badge groups, tight clusters            |
| 12px  | Button groups, nav items, list items    |
| 16px  | Card content sections, feature entries  |
| 20px  | Between feature list items              |
| 24px  | Between cards in grid, form groups      |
| 32px  | Section internal content gaps           |
| 48px  | Between content blocks within a section |
| 64px  | Between page sections                   |
| 80px  | Major section breaks                    |

### Padding Scale

| Value    | Usage                         |
| -------- | ----------------------------- |
| [8, 16]  | Compact buttons, input fields |
| [12, 24] | Standard buttons              |
| [14, 28] | CTA buttons                   |
| 24px     | Card padding (compact)        |
| 32px     | Card padding (standard)       |
| [64, 0]  | Section vertical padding      |
| [96, 0]  | Hero section vertical padding |

### Layout Pattern

- Page width: 1280px max
- Content container: 1120px centered, padding [0, 32]
- Hero section: centered text, max-width 720px, CTA button group centered
- Feature grid: 3-column, gap 24px
- Social proof: logo bar, single row, grayscale logos
- Pricing: 3-column, gap 24px, popular plan with indigo border
- Navigation: fixed top, white, height 64px, shadow on scroll
- Footer: 4-column link grid + bottom bar

## Corner Radius

| Value  | Usage                                      | Rationale                   |
| ------ | ------------------------------------------ | --------------------------- |
| 6px    | Input fields, small badges                 | Minimal functional rounding |
| 8px    | Buttons, dropdown menus                    | Professional button shape   |
| 12px   | Feature cards, pricing cards, testimonials | Primary container radius    |
| 16px   | Large hero cards, image containers         | Comfortable modern rounding |
| 9999px | Pill badges, toggle buttons                | Full capsule for labels     |

Design rationale: The 8-12px range is the sweet spot for professional SaaS — rounded enough to feel modern and approachable, but sharp enough to convey seriousness and reliability. Avoiding the generous 20px+ radii of consumer apps signals enterprise readiness.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

check, arrow-right, chevron-right, chevron-down, shield-check, lock, zap, bar-chart-2, users, settings, globe, code, layers, cloud, database, mail, phone, play-circle, star, menu, x, external-link, download, upload

### Icon Sizes

| Size | Usage                                       |
| ---- | ------------------------------------------- |
| 16px | Inline with body text, feature list checks  |
| 20px | Navigation items, button icons              |
| 24px | Feature card icons, section icons           |
| 32px | Feature grid icons (in light indigo circle) |

### Icon Color States

| State     | Color   | Usage                             |
| --------- | ------- | --------------------------------- |
| Active    | #4F46E5 | Primary actions, links, selected  |
| Default   | #4B5563 | Standard icons, secondary actions |
| Muted     | #9CA3AF | Disabled, placeholder icons       |
| On Accent | #FFFFFF | Icons on indigo backgrounds       |
| Success   | #059669 | Check icons in feature lists      |
