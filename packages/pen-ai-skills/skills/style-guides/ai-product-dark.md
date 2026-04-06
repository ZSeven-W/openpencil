---
name: 'ai-product-dark'
tags: [tech, dark-mode, gradient, mesh-gradient, vibrant, modern, landing-page]
platform: webapp
---

## Style Summary

A futuristic AI product marketing page with a near-black (#09090B) foundation and dual-accent gradients blending violet (#A78BFA) and emerald (#34D399). This style captures the cutting-edge ambition of modern AI/ML products — intelligent, precise, and forward-looking. Geist serves as both heading and body font, its geometric clarity channeling the systematic nature of machine learning. Mesh gradient hero backgrounds and subtle glow effects create depth and atmosphere, while the violet-emerald spectrum evokes neural networks and data flows.

Key aesthetics:

- **True dark base**: #09090B background with #111113 elevated surfaces for layered depth
- **Dual-accent spectrum**: Violet (#A78BFA) for primary actions, emerald (#34D399) for success/secondary — suggesting AI input and output
- **Geometric Geist type**: Geist throughout, clean and systematic at 600-700 heading weight, 400 body
- **Mesh gradient hero**: Complex violet-emerald gradient backgrounds on hero and feature sections
- **Glow effects**: Subtle colored glows on cards and buttons reinforce the tech-forward identity
- **Data visualization ready**: Color palette supports charts, metrics, and live demo embeds

## Color System

### Core Backgrounds

| Token            | Value   | Usage                             |
| ---------------- | ------- | --------------------------------- |
| Page Background  | #09090B | Root page background (near black) |
| Section Alt      | #111113 | Alternating section background    |
| Card Surface     | #18181B | Feature cards, pricing cards      |
| Elevated Surface | #1F1F23 | Modals, dropdown menus, tooltips  |
| Gradient Start   | #A78BFA | Violet start of mesh gradient     |
| Gradient End     | #34D399 | Emerald end of mesh gradient      |

### Text Colors

| Token          | Value   | Usage                                |
| -------------- | ------- | ------------------------------------ |
| Primary Text   | #FAFAFA | Headlines, hero copy, primary labels |
| Secondary Text | #A1A1AA | Body text, descriptions              |
| Tertiary Text  | #71717A | Captions, metadata, footer links     |
| Muted Text     | #3F3F46 | Placeholders, disabled text          |
| Accent Violet  | #A78BFA | Highlighted labels, feature badges   |
| Accent Emerald | #34D399 | Success text, metric values, links   |

### Border Colors

| Token          | Value     | Usage                             |
| -------------- | --------- | --------------------------------- |
| Default Border | #27272A   | Card borders, dividers            |
| Subtle Border  | #1C1C1F   | Section separators                |
| Active Border  | #A78BFA   | Focused inputs, active indicators |
| Glow Border    | #A78BFA30 | Subtle violet glow on hover       |

### Accent Colors

| Token            | Value                    | Usage                                               |
| ---------------- | ------------------------ | --------------------------------------------------- |
| Primary Accent   | #A78BFA                  | CTA buttons, active states, violet spectrum         |
| Secondary Accent | #34D399                  | Secondary buttons, success states, emerald spectrum |
| Gradient Fill    | linear(#A78BFA, #34D399) | Hero backgrounds, premium badges, CTA fills         |
| Accent Muted     | #A78BFA15                | Tinted card backgrounds, feature highlights         |
| Emerald Muted    | #34D39915                | Success tint backgrounds                            |
| Accent Dark      | #7C3AED                  | Violet hover/pressed state                          |
| Error            | #EF4444                  | Error states                                        |
| Warning          | #F59E0B                  | Warning indicators                                  |

## Typography

### Font Families

| Role              | Family | Usage                                            |
| ----------------- | ------ | ------------------------------------------------ |
| Headings          | Geist  | Hero headlines, section headings, feature titles |
| Body / Functional | Geist  | Body text, navigation, buttons, descriptions     |

### Type Scale

| Level      | Size | Font       | Weight | Usage                                          |
| ---------- | ---- | ---------- | ------ | ---------------------------------------------- |
| Hero       | 64px | Geist      | 700    | Primary hero headline                          |
| Display    | 48px | Geist      | 700    | Large section headings                         |
| Title 1    | 36px | Geist      | 600    | Section headings                               |
| Title 2    | 24px | Geist      | 600    | Subsection headings, feature titles            |
| Title 3    | 20px | Geist      | 600    | Card titles, pricing plan names                |
| Body Large | 18px | Geist      | 400    | Hero subtitle, lead paragraphs                 |
| Body       | 16px | Geist      | 400    | Descriptions, feature copy                     |
| Label      | 14px | Geist      | 500    | Navigation items, button text                  |
| Caption    | 12px | Geist      | 500    | Badges, metadata, overline labels (uppercase)  |
| Code       | 14px | Geist Mono | 400    | Code snippets, API references, terminal output |

### Font Weights

| Weight   | Value | Usage                         |
| -------- | ----- | ----------------------------- |
| Regular  | 400   | Body text, descriptions, code |
| Medium   | 500   | Labels, navigation, buttons   |
| Semibold | 600   | Section headings, card titles |
| Bold     | 700   | Hero headline, display text   |

### Letter Spacing

- Hero (64px): -2.5px
- Display (48px): -1.5px
- Section headings (36px): -1px
- Subsection (24px): -0.5px
- Uppercase labels: +2px
- Body text: 0px
- Code: 0px

### Line Height

- Hero (64px): 1.0
- Display (48px): 1.05
- Headings (24-36px): 1.2
- Body (16-18px): 1.6
- Code (14px): 1.5
- Captions (12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                     |
| ----- | ----------------------------------------- |
| 4px   | Inline icon+text pairs                    |
| 8px   | Badge groups, tight element clusters      |
| 12px  | Button groups, nav items                  |
| 16px  | Card content sections, feature list items |
| 24px  | Between cards in grid, form fields        |
| 32px  | Section internal gaps                     |
| 48px  | Between content blocks within sections    |
| 64px  | Between page sections                     |
| 80px  | Major section breaks, hero to content     |

### Padding Scale

| Value    | Usage                         |
| -------- | ----------------------------- |
| [8, 16]  | Compact buttons, code badges  |
| [12, 24] | Standard buttons              |
| [14, 28] | CTA buttons                   |
| 24px     | Card padding (compact)        |
| 32px     | Card padding (standard)       |
| 40px     | Feature card padding          |
| [80, 0]  | Section vertical padding      |
| [120, 0] | Hero section vertical padding |

### Layout Pattern

- Page width: 1440px max
- Content container: 1200px centered, padding [0, 32]
- Hero section: centered text on mesh gradient, max-width 800px, demo embed below
- Feature grid: 3-column, gap 24px, icon + title + description
- Metrics row: 3-4 column, large numbers with labels
- Code showcase: dark inset panel with syntax highlighting
- Navigation: fixed top, transparent over hero, height 64px
- Footer: 4-column link grid + bottom bar

## Corner Radius

| Value  | Usage                        | Rationale                  |
| ------ | ---------------------------- | -------------------------- |
| 6px    | Code blocks, inline badges   | Technical precision        |
| 8px    | Input fields, small tags     | Subtle functional rounding |
| 12px   | Buttons, dropdown menus      | Modern button shape        |
| 16px   | Feature cards, pricing cards | Primary container radius   |
| 20px   | Hero cards, demo embeds      | Generous modern rounding   |
| 9999px | Pill badges, toggle switches | Full capsule for labels    |

Design rationale: The 12-16px range creates a polished, modern tech aesthetic. Cards at 16px feel substantial and contemporary, while buttons at 12px balance clickability with technical credibility. The generous radii soften the dark palette, making the interface feel approachable despite its technical subject matter.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

sparkles, brain, cpu, code, terminal, zap, rocket, bar-chart-2, trending-up, layers, database, cloud, lock, shield, globe, git-branch, play, check, arrow-right, chevron-right, chevron-down, menu, x, github, external-link

### Icon Sizes

| Size | Usage                                           |
| ---- | ----------------------------------------------- |
| 16px | Inline with body text, list item indicators     |
| 20px | Feature list icons, nav items                   |
| 24px | Card header icons, CTA arrows                   |
| 32px | Feature section icons (in violet-tinted circle) |

### Icon Color States

| State       | Color   | Usage                             |
| ----------- | ------- | --------------------------------- |
| Violet      | #A78BFA | Primary actions, AI features      |
| Emerald     | #34D399 | Success states, active outputs    |
| Default     | #A1A1AA | Standard icons, secondary actions |
| Muted       | #3F3F46 | Disabled, placeholder icons       |
| On Gradient | #FFFFFF | Icons on gradient backgrounds     |
