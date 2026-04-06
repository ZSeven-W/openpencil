---
name: 'editorial-serif-light'
tags: [editorial, serif, light-mode, magazine, elegant, typographic, horizontal-nav]
platform: webapp
---

## Style Summary

A refined editorial interface inspired by premium magazine layouts and literary publishing. The warm ivory background (#FAFAF8) provides a paper-like reading surface, while near-black (#1A1A1A) serves as both the primary text color and the sole accent — letting typography itself create all visual hierarchy. Playfair Display brings classical elegance to headlines with its high contrast strokes, while Source Sans 3 delivers effortless readability for body content. Zero-radius corners and hairline dividers enforce a strict typographic grid that recalls printed broadsheets.

Key aesthetics:

- **Typography-driven hierarchy**: No colored accent — weight, size, and serif/sans-serif contrast create all emphasis
- **Ivory canvas**: #FAFAF8 background evokes premium uncoated paper stock
- **High-contrast serif**: Playfair Display with dramatic thick-thin strokes for editorial headlines
- **Horizontal top navigation**: Clean nav bar with text-only items, no sidebar, preserving reading width
- **Zero-radius geometry**: Sharp rectangular forms echo print layout precision
- **Hairline dividers**: 1px #E5E4E0 rules structure content like newspaper columns

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                      |
| --------------- | ------- | ------------------------------------------ |
| Page Background | #FAFAF8 | Root page background (warm ivory)          |
| Card Surface    | #FFFFFF | Cards, elevated containers, feature blocks |
| Inset Surface   | #F5F4F0 | Pull quotes, input fields, recessed areas  |
| Nav Surface     | #FFFFFF | Top navigation bar background              |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #1A1A1A | Headlines, primary content, metric values |
| Secondary Text | #4A4A46 | Body text, article content                |
| Tertiary Text  | #8A8A85 | Bylines, dates, captions                  |
| Muted Text     | #B5B5B0 | Placeholders, disabled text               |
| Accent Text    | #1A1A1A | Active nav items, links (underlined)      |

### Border Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Default Border | #E5E4E0 | Card borders, horizontal rules, dividers |
| Subtle Border  | #F0EFEB | Light section separators                 |
| Active Border  | #1A1A1A | Focused inputs, active tab underlines    |
| Heavy Border   | #1A1A1A | Section header rules (2px weight)        |

### Accent Colors

| Token           | Value     | Usage                                      |
| --------------- | --------- | ------------------------------------------ |
| Primary Accent  | #1A1A1A   | Active states, primary buttons, underlines |
| Accent Inverted | #FFFFFF   | Text on dark backgrounds                   |
| Accent Light    | #1A1A1A10 | Hover backgrounds, subtle tints            |
| Success         | #2D6A4F   | Positive indicators (deep editorial green) |
| Warning         | #B45309   | Alert states (warm editorial amber)        |
| Error           | #991B1B   | Error states (deep editorial red)          |

## Typography

### Font Families

| Role                | Family           | Usage                                                         |
| ------------------- | ---------------- | ------------------------------------------------------------- |
| Display / Headlines | Playfair Display | Hero headings, section titles, pull quotes, feature headlines |
| Body / Functional   | Source Sans 3    | Body text, labels, buttons, navigation, captions              |

### Type Scale

| Level      | Size | Font             | Weight     | Usage                                |
| ---------- | ---- | ---------------- | ---------- | ------------------------------------ |
| Display    | 56px | Playfair Display | 700        | Hero headline, feature title         |
| Title 1    | 36px | Playfair Display | 700        | Section headings, article titles     |
| Title 2    | 24px | Playfair Display | 600        | Subsection headings, card titles     |
| Title 3    | 18px | Playfair Display | 600        | Minor headings, sidebar titles       |
| Pull Quote | 28px | Playfair Display | 400 italic | Editorial pull quotes                |
| Body Large | 16px | Source Sans 3    | 400        | Lead paragraphs, featured body text  |
| Body       | 14px | Source Sans 3    | 400        | Standard body text, descriptions     |
| Label      | 12px | Source Sans 3    | 600        | Section labels, column headers       |
| Caption    | 11px | Source Sans 3    | 400        | Photo captions, bylines, dates       |
| Overline   | 10px | Source Sans 3    | 600        | Category labels (uppercase, tracked) |

### Font Weights

| Weight   | Value | Usage                                         |
| -------- | ----- | --------------------------------------------- |
| Regular  | 400   | Body text, pull quotes (italic), descriptions |
| Semibold | 600   | Labels, sub-headings, navigation items        |
| Bold     | 700   | Display headlines, section titles             |

### Letter Spacing

- Display (56px): -2px (tight editorial compression)
- Title 1 (36px): -1.5px
- Title 2-3 (18-24px): -0.5px
- Overline / uppercase labels: +2px (wide editorial tracking)
- Body text: 0px
- Pull quotes: -0.5px

### Line Height

- Display (56px): 0.95 (tight editorial leading)
- Headings (24-36px): 1.15
- Pull quotes (28px): 1.3
- Body (14-16px): 1.6 (generous reading leading)
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                   |
| ----- | --------------------------------------- |
| 4px   | Inline icon+text pairs, byline elements |
| 8px   | Compact metadata groups, tag rows       |
| 12px  | Navigation items, list items            |
| 16px  | Card internal sections, form fields     |
| 20px  | Between article blocks                  |
| 24px  | Between cards, section content gaps     |
| 32px  | Column gutters, major content gaps      |
| 48px  | Section breaks, editorial page sections |
| 64px  | Top-level page regions                  |

### Padding Scale

| Value    | Usage                                 |
| -------- | ------------------------------------- |
| [8, 16]  | Input fields, compact buttons         |
| [10, 24] | Standard buttons, nav items           |
| [12, 0]  | Section labels (vertical only)        |
| 24px     | Card padding (standard)               |
| 32px     | Feature card padding                  |
| [32, 48] | Content area padding                  |
| [16, 48] | Navigation bar padding                |
| [24, 0]  | Section heading padding (bottom rule) |

### Layout Pattern

- Screen width: 1440px
- Navigation: horizontal top bar, height 56px, full width, text-only items
- Content area: max-width 1120px centered, vertical, gap 48-64, padding [32, 48]
- Article columns: max-width 720px for body text, 960px for feature layouts
- Cards use 1px stroke (#E5E4E0) with no shadow for editorial flatness
- Heavy 2px top rules (#1A1A1A) above major section headings

## Corner Radius

| Value | Usage                                       | Rationale               |
| ----- | ------------------------------------------- | ----------------------- |
| 0px   | Everything — cards, buttons, inputs, images | Pure editorial geometry |

Design rationale: Zero radius across the entire system is a deliberate editorial choice. Sharp corners reference the precise geometry of print layouts — newspaper columns, magazine spreads, and book typography all work with exact rectangles. This forces the design to rely entirely on typography, spacing, and color weight for hierarchy, which is the hallmark of editorial excellence. The warm ivory background (#FAFAF8) prevents the sharp geometry from feeling cold.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (the stroke warmth contrasts with sharp geometry)

### Commonly Used Icons

search, menu, x, chevron-right, chevron-left, arrow-right, arrow-up-right, bookmark, share, heart, clock, calendar, user, mail, rss, external-link, image, type, quote, grid, list, eye, pen-line, globe

### Icon Sizes

| Size | Usage                                         |
| ---- | --------------------------------------------- |
| 14px | Inline metadata indicators, byline icons      |
| 16px | Navigation bar icons, list leading icons      |
| 20px | Card action icons, social share buttons       |
| 24px | Header actions, hamburger menu, search toggle |

### Icon Color States

| State   | Color   | Usage                              |
| ------- | ------- | ---------------------------------- |
| Active  | #1A1A1A | Active navigation, selected states |
| Default | #4A4A46 | Standard icons, secondary actions  |
| Muted   | #B5B5B0 | Disabled icons, placeholder states |
| On Dark | #FFFFFF | Icons on dark accent backgrounds   |
