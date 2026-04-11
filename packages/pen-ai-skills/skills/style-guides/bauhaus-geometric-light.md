---
name: 'bauhaus-geometric-light'
tags: [bauhaus, geometric, light-mode, primary-colors, bold-typography, sharp-corners, webapp]
platform: webapp
---

## Style Summary

A bold, Bauhaus-inspired interface channeling the geometric rigor and primary-color confidence of the 1920s design movement. The near-white #FAFAFA background acts as a neutral gallery wall against which strong blocks of red (#D32F2F), blue (#1565C0), and yellow (#FBC02D) create decisive visual anchors. Work Sans at weight 700 delivers sturdy, grotesk-style headings with industrial authority, while the same family at 400 provides clean, highly legible body text. Every corner radius is 0px — pure geometric precision with no decorative softening. The result is a system that feels architectural, intentional, and unapologetically modern.

Key aesthetics:

- **Primary color triad**: Red (#D32F2F), blue (#1565C0), yellow (#FBC02D) used as functional accent blocks
- **Zero radii**: 0px corners throughout — geometric purity with no curves
- **Industrial grotesk type**: Work Sans at two weights (700/400) creates clear hierarchy with a single family
- **Gallery-white canvas**: #FAFAFA background lets color blocks and bold typography dominate
- **Flat, borderless cards**: Cards rely on background contrast and color accents rather than border outlines
- **Grid-driven layout**: Strong alignment to an 8px grid reinforces the architectural philosophy

## Color System

### Core Backgrounds

| Token              | Value   | Usage                                    |
| ------------------ | ------- | ---------------------------------------- |
| Page Background    | #FAFAFA | Root page background (gallery white)     |
| Card Surface       | #FFFFFF | Cards, panels, elevated containers       |
| Inset Surface      | #F5F5F5 | Input backgrounds, recessed areas        |
| Color Block Red    | #D32F2F | Primary accent blocks, active indicators |
| Color Block Blue   | #1565C0 | Secondary accent blocks, navigation      |
| Color Block Yellow | #FBC02D | Tertiary accent blocks, highlights       |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #212121 | Headings, primary labels, display values |
| Secondary Text | #616161 | Body text, descriptions                  |
| Tertiary Text  | #9E9E9E | Captions, timestamps, helper text        |
| Muted Text     | #BDBDBD | Placeholders, disabled text              |
| On Color Text  | #FFFFFF | Text on red/blue color blocks            |
| On Yellow Text | #212121 | Text on yellow color blocks              |

### Border Colors

| Token             | Value   | Usage                                     |
| ----------------- | ------- | ----------------------------------------- |
| Default Border    | #E0E0E0 | Card borders, dividers, input outlines    |
| Subtle Border     | #EEEEEE | Light separators, table row dividers      |
| Active Border     | #212121 | Focused inputs, active indicators (black) |
| Accent Border Red | #D32F2F | Red accent borders, error states          |

### Accent Colors

| Token             | Value     | Usage                                        |
| ----------------- | --------- | -------------------------------------------- |
| Primary Accent    | #D32F2F   | Primary buttons, active states (red)         |
| Primary Hover     | #B71C1C   | Button hover, pressed states (darker red)    |
| Secondary Accent  | #1565C0   | Secondary buttons, navigation accents (blue) |
| Tertiary Accent   | #FBC02D   | Highlight badges, callout blocks (yellow)    |
| Accent Light Red  | #D32F2F15 | Tinted backgrounds, selected rows            |
| Accent Light Blue | #1565C015 | Blue-tinted hover backgrounds                |
| Success           | #2E7D32   | Positive metrics, completion states          |
| Warning           | #F57F17   | Alert indicators, pending states             |
| Error             | #C62828   | Error states, destructive actions            |

## Typography

### Font Families

| Role              | Family    | Usage                                                      |
| ----------------- | --------- | ---------------------------------------------------------- |
| Headings          | Work Sans | Section headings, display titles, card titles (weight 700) |
| Body / Functional | Work Sans | Body text, labels, buttons, navigation (weight 400)        |

### Type Scale

| Level      | Size | Font      | Weight | Usage                                   |
| ---------- | ---- | --------- | ------ | --------------------------------------- |
| Display    | 48px | Work Sans | 700    | Page titles, hero headings              |
| Metric     | 36px | Work Sans | 700    | Primary metric values, KPI numbers      |
| Title 1    | 28px | Work Sans | 700    | Section headings                        |
| Title 2    | 20px | Work Sans | 700    | Card titles, subsection headings        |
| Title 3    | 16px | Work Sans | 600    | List headings, compact card titles      |
| Body Large | 16px | Work Sans | 400    | Lead paragraphs, featured descriptions  |
| Body       | 14px | Work Sans | 400    | Standard body text, table cells         |
| Label      | 12px | Work Sans | 600    | Field labels, column headers, nav items |
| Caption    | 11px | Work Sans | 400    | Timestamps, secondary metadata          |
| Overline   | 11px | Work Sans | 700    | Section overlines (uppercase)           |
| Micro      | 10px | Work Sans | 600    | Badges, status labels                   |

### Font Weights

| Weight   | Value | Usage                                 |
| -------- | ----- | ------------------------------------- |
| Regular  | 400   | Body text, descriptions, table data   |
| Semibold | 600   | Labels, buttons, compact headings     |
| Bold     | 700   | All headings, display text, overlines |

### Letter Spacing

- Display (48px): -1.5px
- Section headings (20-28px): -0.5px
- Overline / uppercase labels: +2px (wide, architectural)
- Body text: 0px
- Metric values: -1px

### Line Height

- Display (48px): 1.0
- Headings (20-28px): 1.15
- Body (14-16px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                       |
| ----- | ------------------------------------------- |
| 4px   | Inline icon+text pairs                      |
| 8px   | Compact element groups, button icon to text |
| 12px  | Between list items, nav items               |
| 16px  | Card internal sections, form fields         |
| 24px  | Between cards in a grid, content sections   |
| 32px  | Major section breaks                        |
| 48px  | Top-level page sections                     |
| 64px  | Hero section spacing                        |

### Padding Scale

| Value    | Usage                          |
| -------- | ------------------------------ |
| [8, 16]  | Compact buttons, badges        |
| [10, 20] | Standard buttons, input fields |
| [12, 24] | Large buttons, search bars     |
| [12, 16] | Sidebar nav items              |
| 20px     | Card padding (compact)         |
| 24px     | Card padding (standard)        |
| 32px     | Feature card padding           |
| [32, 40] | Content area padding           |

### Layout Pattern

- Screen width: 1440px
- Sidebar: 240px wide, vertical, white background with black text and red active indicators
- Content area: fill_container, vertical, gap 24-32, padding [32, 40]
- Cards use flat surfaces with no shadow, relying on background contrast (#FFFFFF on #FAFAFA)
- Color blocks (red/blue/yellow rectangles) used as section accents and dividers
- Header bar: 64px height, white background, bottom border #E0E0E0

## Corner Radius

| Value | Usage                                                | Rationale                 |
| ----- | ---------------------------------------------------- | ------------------------- |
| 0px   | All elements: buttons, cards, inputs, badges, modals | Total geometric precision |

Design rationale: The Bauhaus movement rejected ornament in favor of pure geometric form. A universal 0px radius honors this philosophy — every rectangle is a true rectangle, every surface meets its neighbor at a precise right angle. This creates an unmistakably architectural quality that distinguishes the interface from contemporary rounded-everything trends. The sharpness amplifies the authority of the primary color blocks and bold typography, producing an interface that feels like a designed artifact rather than a soft consumer product.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (the stroke softness counterbalances the sharp corners)

### Commonly Used Icons

layout-grid, square, circle, triangle, grid, layers, ruler, pen-tool, move, maximize, minimize, plus, x, search, settings, chevron-right, chevron-down, arrow-right, arrow-up-right, eye, download, upload, folder, file, users, bar-chart-2, target, zap

### Icon Sizes

| Size | Usage                                      |
| ---- | ------------------------------------------ |
| 14px | Inline indicators, table row actions       |
| 16px | Sidebar navigation, button leading icons   |
| 20px | Card header action icons, toolbar actions  |
| 24px | Primary sidebar icons, page header actions |
| 32px | Feature section accent icons               |

### Icon Color States

| State       | Color   | Usage                                      |
| ----------- | ------- | ------------------------------------------ |
| Active Red  | #D32F2F | Active navigation, primary selected states |
| Active Blue | #1565C0 | Secondary active states, linked elements   |
| Default     | #616161 | Standard icons, secondary actions          |
| Muted       | #BDBDBD | Disabled icons, placeholder states         |
| On Color    | #FFFFFF | Icons on red/blue accent backgrounds       |
| On Yellow   | #212121 | Icons on yellow accent backgrounds         |
