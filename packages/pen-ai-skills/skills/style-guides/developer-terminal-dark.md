---
name: 'developer-terminal-dark'
tags: [developer, terminal, dark-mode, monospace, minimal, tech, code-inspired]
platform: webapp
---

## Style Summary

A developer-centric dark interface inspired by modern code editors and IDE environments. The #1E1E1E background — the exact shade of VS Code's default dark theme — immediately feels familiar to engineers. Fira Code brings ligature-ready monospace typography for headings and data, while Inter handles body text with clean readability. The blue accent (#569CD6) references VS Code's keyword highlighting, creating an interface that feels like a natural extension of the development environment.

Key aesthetics:

- **IDE palette**: Background (#1E1E1E) and accent (#569CD6) directly reference VS Code's default dark theme colors
- **Code typography**: Fira Code for headings and data values, enabling programming ligatures and a native dev feel
- **Compact radii**: 4-8px keeps elements tight and functional, matching IDE panel aesthetics
- **Activity bar pattern**: Narrow left sidebar with icon-only navigation, mirroring VS Code's activity bar
- **Syntax-inspired color coding**: Blue for primary, green for success, orange for warnings — matching common syntax themes
- **Tab-based navigation**: Content area uses tab-style headers with underline active indicators

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                        |
| --------------- | ------- | -------------------------------------------- |
| Page Background | #1E1E1E | Root page background (VS Code dark)          |
| Card Surface    | #252526 | Cards, panels, editor surfaces               |
| Sidebar Surface | #1E1E1E | Activity bar / sidebar background            |
| Inset Surface   | #2D2D2D | Input fields, nested containers, code blocks |
| Title Bar       | #3C3C3C | Top bar, tab headers                         |

### Text Colors

| Token          | Value   | Usage                                          |
| -------------- | ------- | ---------------------------------------------- |
| Primary Text   | #D4D4D4 | Headings, primary content (VS Code foreground) |
| Secondary Text | #9CDCFE | Variable-style text, secondary highlights      |
| Tertiary Text  | #808080 | Comments, captions, inactive labels            |
| Muted Text     | #5A5A5A | Placeholders, disabled states                  |
| Accent Text    | #569CD6 | Keywords, active items, links                  |
| String Text    | #CE9178 | Quoted values, special labels                  |

### Border Colors

| Token               | Value     | Usage                                |
| ------------------- | --------- | ------------------------------------ |
| Default Border      | #3C3C3C   | Card borders, panel dividers         |
| Subtle Border       | #2D2D2D   | Section separators, light dividers   |
| Active Border       | #569CD6   | Focused inputs, active tab underline |
| Active Muted Border | #569CD640 | Decorative accent at 25% opacity     |

### Accent Colors

| Token          | Value     | Usage                                            |
| -------------- | --------- | ------------------------------------------------ |
| Primary Accent | #569CD6   | Active states, keywords, tab indicators          |
| Accent Subtle  | #569CD620 | Hover backgrounds, selection tint                |
| Success        | #6A9955   | Positive metrics, success states (comment green) |
| Error          | #F44747   | Error states, destructive actions                |
| Warning        | #CE9178   | Warnings, string-colored alerts                  |
| Info           | #4EC9B0   | Informational badges (type green)                |

## Typography

### Font Families

| Role           | Family    | Usage                                                        |
| -------------- | --------- | ------------------------------------------------------------ |
| Display / Data | Fira Code | Metric values, headings, labels, navigation, code references |
| Body           | Inter     | Body text, descriptions, longer paragraphs                   |

### Type Scale

| Level     | Size | Font      | Weight | Usage                                    |
| --------- | ---- | --------- | ------ | ---------------------------------------- |
| Display   | 36px | Fira Code | 700    | Hero metric values, primary counters     |
| Title 1   | 24px | Fira Code | 600    | Section headings                         |
| Title 2   | 18px | Fira Code | 600    | Card titles, panel headings              |
| Title 3   | 15px | Fira Code | 500    | List item titles, sub-panel headers      |
| Body      | 14px | Inter     | 400    | Descriptions, body text                  |
| Body Mono | 13px | Fira Code | 400    | Code snippets, terminal output           |
| Label     | 12px | Fira Code | 500    | Field labels, tab labels, column headers |
| Caption   | 11px | Fira Code | 400    | Timestamps, metadata, file paths         |
| Micro     | 10px | Fira Code | 500    | Status badges, line numbers (uppercase)  |

### Font Weights

| Weight   | Value | Usage                                        |
| -------- | ----- | -------------------------------------------- |
| Regular  | 400   | Body text, code, descriptions                |
| Medium   | 500   | Labels, navigation items, secondary headings |
| Semibold | 600   | Section titles, card headings                |
| Bold     | 700   | Display metrics, primary emphasis            |

### Letter Spacing

- Display metrics (36px): -1px
- Uppercase labels: +1px to +1.5px
- Monospace code: 0px (natural mono spacing)
- Body text: 0px

### Line Height

- Display metrics (36px): 1.0
- Headings (18-24px): 1.2
- Body text (14px): 1.5
- Code / captions (10-13px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                        |
| ----- | -------------------------------------------- |
| 0px   | Activity bar items (padding-only separation) |
| 4px   | Inline icon+text pairs, status dot to label  |
| 8px   | Tab groups, compact metric sections          |
| 12px  | Between list items, button groups            |
| 16px  | Card internal sections, nav item spacing     |
| 20px  | Panel sections, form field groups            |
| 24px  | Between cards in a grid                      |
| 28px  | Content area major sections                  |
| 32px  | Top-level page sections                      |

### Padding Scale

| Value    | Usage                              |
| -------- | ---------------------------------- |
| [8, 12]  | Activity bar items                 |
| [10, 16] | Standard buttons                   |
| [8, 12]  | Input fields, compact buttons      |
| 16px     | Card padding (compact)             |
| 20px     | Standard card padding              |
| 24px     | Large card padding, section insets |
| [24, 32] | Content area padding               |

### Layout Pattern

- Screen width: 1440px
- Activity bar: 48px wide, vertical, icon-only, centered icons
- Sidebar panel: 240px wide, vertical, gap 0 (items use padding)
- Content area: fill_container, vertical, gap 24-32, padding [24, 32]
- Tab underline indicator: 2px bottom border in accent blue

## Corner Radius

| Value | Usage                                   | Rationale                      |
| ----- | --------------------------------------- | ------------------------------ |
| 0px   | Tab underlines, dividers, activity bar  | Flat precision elements        |
| 4px   | Buttons, inputs, badges, small elements | Compact IDE-style rounding     |
| 6px   | Cards, panels, dropdown menus           | Primary container radius       |
| 8px   | Large cards, modal dialogs              | Maximum radius — still compact |

Design rationale: The 4-8px range mirrors VS Code and modern IDE panel aesthetics. Corners are rounded just enough to feel approachable without becoming soft or playful. The compact radii maintain the tool-like, functional character of a developer environment while avoiding the harshness of fully sharp corners.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

terminal, code, file, folder, git-branch, git-commit, search, settings, play, pause, bug, check-circle, x-circle, chevron-right, chevron-down, plus, minus, copy, clipboard, download, upload, refresh-cw, eye, lock, globe, database, cpu, layers

### Icon Sizes

| Size | Usage                                    |
| ---- | ---------------------------------------- |
| 14px | Inline indicators, breadcrumb separators |
| 16px | Activity bar icons, tab icons            |
| 18px | Sidebar navigation icons, list items     |
| 20px | Card header icons, action buttons        |
| 24px | Primary actions, header icons            |

### Icon Color States

| State      | Color   | Usage                              |
| ---------- | ------- | ---------------------------------- |
| Active     | #569CD6 | Selected activity bar, active tab  |
| Default    | #808080 | Inactive items, secondary actions  |
| Muted      | #5A5A5A | Disabled icons, placeholder states |
| On Surface | #D4D4D4 | Icons on elevated surfaces         |
