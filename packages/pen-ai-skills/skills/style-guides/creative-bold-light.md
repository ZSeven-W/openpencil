---
name: 'creative-bold-light'
tags: [creative, bold-typography, light-mode, vibrant, playful, gradient, expressive]
platform: webapp
---

## Style Summary

A high-energy, expressive interface designed for creative agencies, portfolio platforms, and design tool dashboards. Pure white backgrounds (#FFFFFF) provide maximum contrast for the vibrant violet accent (#7C3AED) and its gradient companions. Clash Display delivers oversized, unapologetic headlines with dramatic weight (700-800) that command attention, while Satoshi handles functional text with geometric clarity. Generous 16-24px radii create pill-shaped buttons and soft card containers, while gradient accents on primary CTAs and feature sections inject creative energy into every interaction.

Key aesthetics:

- **Bold display type**: Clash Display at 700-800 weight creates dramatic, poster-like headlines
- **Violet energy**: #7C3AED as the primary accent with gradient extensions to fuchsia (#D946EF)
- **Gradient accents**: Linear gradients on primary buttons and feature highlights for creative punch
- **Pill-shaped buttons**: 24px radius on buttons creates distinctive, eye-catching interactive elements
- **Clean white canvas**: #FFFFFF background lets vibrant typography and color accents dominate
- **Generous type scale**: Large size jumps between levels create clear visual drama

## Color System

### Core Backgrounds

| Token            | Value                                     | Usage                                  |
| ---------------- | ----------------------------------------- | -------------------------------------- |
| Page Background  | #FFFFFF                                   | Root page background (pure white)      |
| Card Surface     | #FFFFFF                                   | Cards with border, elevated containers |
| Inset Surface    | #F9FAFB                                   | Input backgrounds, secondary surfaces  |
| Accent Surface   | #F5F3FF                                   | Violet-tinted highlight sections       |
| Gradient Surface | linear-gradient(135deg, #F5F3FF, #FDF2F8) | Feature hero backgrounds               |

### Text Colors

| Token          | Value                                     | Usage                                    |
| -------------- | ----------------------------------------- | ---------------------------------------- |
| Primary Text   | #111827                                   | Headlines, primary content               |
| Secondary Text | #4B5563                                   | Body text, descriptions                  |
| Tertiary Text  | #9CA3AF                                   | Captions, timestamps                     |
| Muted Text     | #D1D5DB                                   | Placeholders, disabled text              |
| Accent Text    | #7C3AED                                   | Active states, links, highlighted values |
| Gradient Text  | linear-gradient(135deg, #7C3AED, #D946EF) | Hero headlines (clipped gradient)        |

### Border Colors

| Token           | Value                                     | Usage                             |
| --------------- | ----------------------------------------- | --------------------------------- |
| Default Border  | #E5E7EB                                   | Card borders, dividers            |
| Subtle Border   | #F3F4F6                                   | Light separators                  |
| Active Border   | #7C3AED                                   | Focused inputs, active indicators |
| Gradient Border | linear-gradient(135deg, #7C3AED, #D946EF) | Featured card borders             |

### Accent Colors

| Token                 | Value     | Usage                                |
| --------------------- | --------- | ------------------------------------ |
| Primary Accent        | #7C3AED   | Solid buttons, active states, violet |
| Accent Hover          | #6D28D9   | Button hover state                   |
| Accent Gradient Start | #7C3AED   | Gradient button start (violet)       |
| Accent Gradient End   | #D946EF   | Gradient button end (fuchsia)        |
| Accent Light          | #7C3AED15 | Tinted backgrounds, hover states     |
| Accent Muted          | #7C3AED40 | Badge fills, progress tracks         |
| Secondary Accent      | #D946EF   | Secondary highlights, fuchsia        |
| Success               | #10B981   | Positive metrics, completed states   |
| Warning               | #F59E0B   | Alert indicators                     |
| Error                 | #EF4444   | Error states, destructive actions    |
| Info                  | #3B82F6   | Informational elements               |

## Typography

### Font Families

| Role                | Family        | Usage                                                             |
| ------------------- | ------------- | ----------------------------------------------------------------- |
| Display / Headlines | Clash Display | Hero headings, section titles, feature headlines (weight 700-800) |
| Body / Functional   | Satoshi       | Body text, labels, buttons, navigation, all functional text       |

### Type Scale

| Level      | Size | Font          | Weight | Usage                                  |
| ---------- | ---- | ------------- | ------ | -------------------------------------- |
| Hero       | 64px | Clash Display | 800    | Landing hero headline                  |
| Display    | 48px | Clash Display | 700    | Page titles, feature headings          |
| Title 1    | 32px | Clash Display | 700    | Section headings                       |
| Title 2    | 22px | Clash Display | 600    | Card titles, subsection headings       |
| Title 3    | 17px | Satoshi       | 700    | List headings, compact card titles     |
| Body Large | 16px | Satoshi       | 400    | Lead paragraphs, featured descriptions |
| Body       | 14px | Satoshi       | 400    | Standard body text, descriptions       |
| Label      | 13px | Satoshi       | 500    | Button text, field labels, nav items   |
| Caption    | 12px | Satoshi       | 400    | Timestamps, secondary info             |
| Small      | 11px | Satoshi       | 500    | Badges, auxiliary labels               |
| Micro      | 10px | Satoshi       | 600    | Tag labels, micro indicators           |

### Font Weights

| Weight     | Value | Usage                             |
| ---------- | ----- | --------------------------------- |
| Regular    | 400   | Body text, descriptions, captions |
| Medium     | 500   | Labels, buttons, navigation items |
| Semibold   | 600   | Sub-headings, card titles         |
| Bold       | 700   | Section titles, display headings  |
| Extra Bold | 800   | Hero headlines only               |

### Letter Spacing

- Hero (64px): -3px (extremely tight for poster impact)
- Display (48px): -2px
- Title 1 (32px): -1px
- Title 2 (22px): -0.5px
- Uppercase labels: +1.5px
- Body text: 0px

### Line Height

- Hero (64px): 0.9 (ultra-tight display leading)
- Display (48px): 0.95
- Headings (22-32px): 1.15
- Body (14-16px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                               |
| ----- | ----------------------------------- |
| 4px   | Inline icon+text pairs              |
| 6px   | Tag items, badge groups             |
| 8px   | Compact element groups              |
| 12px  | Between list items, nav items       |
| 16px  | Card internal sections, form fields |
| 20px  | Between cards in a grid             |
| 24px  | Content section gaps                |
| 32px  | Major section breaks                |
| 48px  | Feature section gaps                |
| 64px  | Top-level page hero sections        |

### Padding Scale

| Value    | Usage                       |
| -------- | --------------------------- |
| [6, 16]  | Small badges, tags          |
| [10, 20] | Standard buttons            |
| [12, 28] | Large buttons, primary CTAs |
| [16, 32] | Hero buttons, feature CTAs  |
| 20px     | Card padding (compact)      |
| 24px     | Card padding (standard)     |
| 32px     | Feature card padding        |
| [32, 48] | Content area padding        |
| [48, 48] | Hero section padding        |

### Layout Pattern

- Screen width: 1440px
- Navigation: top horizontal bar, height 72px, logo + centered nav + CTA button
- Content area: max-width 1280px centered, vertical, gap 48-64, padding [32, 48]
- Hero sections: full-width, gradient backgrounds, centered content, gap 24
- Card grids: 3-column, gap 20, responsive
- Cards use subtle border (#E5E7EB) with soft shadow (0 4px 16px rgba(0,0,0,0.06))
- Feature cards may use gradient borders for emphasis

## Corner Radius

| Value | Usage                                            | Rationale                     |
| ----- | ------------------------------------------------ | ----------------------------- |
| 8px   | Input fields, small badges, tooltips             | Baseline softening            |
| 12px  | Standard buttons, dropdowns                      | Soft interactive radius       |
| 16px  | Cards, modals, panels, feature sections          | Primary container radius      |
| 20px  | Large feature cards, image containers            | Generous creative softness    |
| 24px  | Primary CTA buttons, pill buttons                | Signature pill-shaped buttons |
| 100px | Avatar circles, toggle pills, round icon buttons | Full capsule/circle shape     |

Design rationale: The 8-24px radius range is deliberately bold and expressive. Pill-shaped buttons (24px) are the signature interactive element — they break from conventional rectangles and signal creative confidence. Cards at 16-20px feel friendly and modern without becoming childish. This progressive scale from structured (8px) to expressive (24px) mirrors the creative agency aesthetic of balancing professionalism with visual excitement.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (complements the generous radius system)

### Commonly Used Icons

palette, sparkles, wand-2, layers, grid, image, video, pen-tool, brush, star, heart, rocket, zap, target, globe, share, download, upload, plus, x, search, bell, user, settings, arrow-right, arrow-up-right, play, eye, copy, link

### Icon Sizes

| Size | Usage                                  |
| ---- | -------------------------------------- |
| 14px | Inline indicators, tag icons           |
| 16px | Navigation icons, button leading icons |
| 20px | Card action icons, toolbar actions     |
| 24px | Header actions, feature icons          |
| 32px | Hero section accent icons              |

### Icon Color States

| State     | Color   | Usage                                   |
| --------- | ------- | --------------------------------------- |
| Active    | #7C3AED | Selected navigation, active states      |
| Default   | #4B5563 | Standard icons, secondary actions       |
| Muted     | #D1D5DB | Disabled icons, placeholder states      |
| On Accent | #FFFFFF | Icons on violet/gradient backgrounds    |
| Creative  | #D946EF | Special highlight icons, fuchsia accent |
