---
name: 'industrial-mobile-dark'
tags: [industrial, dark-mode, monospace, sharp-corners, tech, mobile, developer]
platform: mobile
---

## Style Summary

A technical, no-nonsense dark mobile interface designed for developer tools, system monitors, and data-heavy applications. The zinc-dark background (#18181B) provides a neutral, eye-friendly foundation for long reading sessions, while the amber accent (#F59E0B) creates high-visibility callouts reminiscent of terminal warnings and industrial signage. JetBrains Mono brings monospace precision to headings and data displays, complemented by Inter for readable body content. Compact 4-8px corner radii keep the interface feeling sharp and engineered, with every pixel serving a functional purpose.

Key aesthetics:

- **Zinc-dark canvas**: #18181B background is neutral and easy on the eyes for extended use
- **Amber signal accent**: #F59E0B evokes terminal caution signals and industrial warning lights
- **Monospace precision**: JetBrains Mono for headings and data reinforces the technical identity
- **Sharp corners**: 4-8px radii create a compact, engineered feel with minimal decoration
- **Data density**: Tighter spacing and smaller type for information-heavy screens
- **Pill tab bar**: 100px radius bottom navigation with amber active indicator

## Color System

### Core Backgrounds

| Token            | Value   | Usage                              |
| ---------------- | ------- | ---------------------------------- |
| Page Background  | #18181B | Root screen background (zinc dark) |
| Card Surface     | #27272A | Standard cards, list items         |
| Elevated Surface | #3F3F46 | Elevated cards, modal sheets       |
| Code Surface     | #1E1E22 | Code blocks, terminal output areas |
| Tab Bar Surface  | #27272A | Bottom navigation pill fill        |

### Text Colors

| Token          | Value   | Usage                                    |
| -------------- | ------- | ---------------------------------------- |
| Primary Text   | #FAFAFA | Headings, metric values, primary labels  |
| Secondary Text | #A1A1AA | Body text, descriptions                  |
| Tertiary Text  | #71717A | Timestamps, captions, inactive labels    |
| Muted Text     | #3F3F46 | Placeholders, disabled text              |
| Accent Text    | #F59E0B | Active states, highlighted values, links |

### Border Colors

| Token          | Value   | Usage                                |
| -------------- | ------- | ------------------------------------ |
| Default Border | #3F3F46 | Card borders, dividers               |
| Subtle Border  | #27272A | Light separators                     |
| Active Border  | #F59E0B | Focused inputs, active tab indicator |
| Code Border    | #52525B | Code block borders, terminal frames  |

### Accent Colors

| Token          | Value     | Usage                                             |
| -------------- | --------- | ------------------------------------------------- |
| Primary Accent | #F59E0B   | Active states, buttons, highlights, tab indicator |
| Accent Light   | #F59E0B20 | Tinted backgrounds, badge fills                   |
| Accent Muted   | #78350F   | Deep amber backgrounds, subtle indicators         |
| Accent On-Dark | #18181B   | Text/icons on amber backgrounds (dark)            |
| Success        | #22C55E   | Build passed, healthy metrics                     |
| Warning        | #F59E0B   | Warnings, caution states (same as accent)         |
| Error          | #EF4444   | Build failed, error states                        |
| Info           | #3B82F6   | Informational, debug messages                     |

## Typography

### Font Families

| Role              | Family         | Usage                                                |
| ----------------- | -------------- | ---------------------------------------------------- |
| Display / Data    | JetBrains Mono | Headings, metric values, code, data displays         |
| Body / Functional | Inter          | Body text, labels, buttons, navigation, descriptions |

### Type Scale

| Level   | Size | Font           | Weight | Usage                                |
| ------- | ---- | -------------- | ------ | ------------------------------------ |
| Display | 32px | JetBrains Mono | 700    | Hero metric values, primary stats    |
| Title 1 | 22px | JetBrains Mono | 700    | Screen titles                        |
| Title 2 | 17px | JetBrains Mono | 600    | Section headings                     |
| Title 3 | 14px | Inter          | 600    | Card titles, list headings           |
| Body    | 13px | Inter          | 400    | Body text, descriptions              |
| Label   | 12px | Inter          | 500    | Field labels, button text            |
| Data    | 13px | JetBrains Mono | 400    | Metric values, code, timestamps      |
| Caption | 11px | Inter          | 400    | Timestamps, metadata                 |
| Small   | 10px | JetBrains Mono | 500    | Log entries, auxiliary data          |
| Micro   | 10px | Inter          | 500    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight   | Value | Usage                                |
| -------- | ----- | ------------------------------------ |
| Regular  | 400   | Body text, data values, descriptions |
| Medium   | 500   | Labels, navigation, button text      |
| Semibold | 600   | Section headings, card titles        |
| Bold     | 700   | Screen titles, hero metrics          |

### Letter Spacing

- Display (32px): -1px
- Section headings (17-22px): -0.5px
- Uppercase tab labels: +2px (wide for monospace feel)
- Monospace data: 0px
- Body text: 0px

### Line Height

- Display (32px): 1.0
- Headings (17-22px): 1.2
- Body (13px): 1.5
- Monospace data (13px): 1.4
- Captions (10-11px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                 |
| ----- | ------------------------------------- |
| 2px   | Tight inline pairs, status dots       |
| 4px   | Tab icon to label, inline icon groups |
| 6px   | Status indicator groups               |
| 8px   | Compact card content, log entries     |
| 10px  | Between list items, metric rows       |
| 12px  | Form fields, button groups            |
| 16px  | Card internal sections                |
| 20px  | Between cards, section content        |
| 24px  | Section gaps within content           |
| 32px  | Top-level screen section breaks       |

### Padding Scale

| Value            | Usage                                        |
| ---------------- | -------------------------------------------- |
| [0, 20]          | Screen content wrapper (horizontal, compact) |
| [12, 21, 21, 21] | Tab bar section outer padding                |
| 4px              | Tab bar pill inner padding                   |
| [6, 12]          | Compact input fields, code blocks            |
| [8, 16]          | Standard buttons, input fields               |
| [10, 20]         | Large buttons, CTA buttons                   |
| 12px             | Compact card padding                         |
| 16px             | Standard card padding                        |
| 20px             | Feature card padding, large sections         |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 20], vertical, gap 20-24
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Code blocks: code surface (#1E1E22), code border, 4px radius, monospace
- Data rows: compact 10px gap, monospace values right-aligned

## Corner Radius

| Value | Usage                                         | Rationale                         |
| ----- | --------------------------------------------- | --------------------------------- |
| 4px   | Code blocks, small buttons, compact badges    | Sharp, engineered precision       |
| 6px   | Standard cards, list containers, input fields | Primary container radius          |
| 8px   | Large cards, grouped sections, modal sheets   | Maximum standard radius           |
| 100px | Tab bar pill, toggle pills, status indicators | Full capsule shape for navigation |

Design rationale: Compact 4-8px radii create the sharp, engineered aesthetic expected in developer and technical tools. Minimal rounding signals precision and seriousness, while still softening harsh 90-degree corners enough for comfortable mobile interaction. The dramatic jump to 100px for the tab bar pill creates a clear visual distinction between navigation and content.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

terminal, code, git-branch, git-commit, cpu, server, database, wifi, battery, signal, activity, bar-chart-2, clock, bell, search, settings, folder, file-code, play, pause, refresh-cw, check-circle, alert-triangle, x-circle, chevron-right

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 12px | Inline log indicators, status dots                   |
| 16px | Tab bar icons, list leading icons                    |
| 18px | Card action icons, data row icons                    |
| 20px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                        |
| --------- | ------- | -------------------------------------------- |
| Active    | #F59E0B | Selected tab, active states, amber highlight |
| Default   | #A1A1AA | Inactive tabs, secondary actions             |
| Muted     | #3F3F46 | Disabled states, placeholder icons           |
| On Accent | #18181B | Icons on amber-colored backgrounds (dark)    |
