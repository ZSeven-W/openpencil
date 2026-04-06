---
name: 'tech-developer-dark'
tags: [developer, dark-mode, monospace, tech, code-inspired, terminal, landing-page]
platform: webapp
---

## Style Summary

A developer-focused landing page inspired by GitHub's dark UI and modern code editors. The deep ink background (#0D1117) paired with GitHub blue (#58A6FF) creates a familiar, trustworthy environment for technical audiences. JetBrains Mono headlines give the page a code-native identity that signals "built by developers, for developers," while Inter body text maintains comfortable readability for marketing copy. Code blocks, terminal-inspired UI elements, and monospace accents throughout reinforce the technical credibility.

Key aesthetics:

- **GitHub dark palette**: #0D1117 background with #161B22 card surfaces, instantly familiar to developers
- **GitHub blue accent**: #58A6FF for links, CTAs, and highlights — the color of clickable code references
- **Code-native display**: JetBrains Mono for headlines and feature highlights, giving text a monospaced rhythm
- **Compact radii**: 8px cards, 6px buttons — functional, engineering-minded rounding
- **Code block integration**: Syntax-highlighted code blocks as first-class content elements
- **Terminal motifs**: Dot indicators, command-line prompts, and monospace labels as decorative patterns

## Color System

### Core Backgrounds

| Token            | Value   | Usage                              |
| ---------------- | ------- | ---------------------------------- |
| Page Background  | #0D1117 | Root page background (GitHub dark) |
| Section Alt      | #161B22 | Alternating section background     |
| Card Surface     | #161B22 | Feature cards, pricing cards       |
| Elevated Surface | #1C2128 | Dropdowns, modals, tooltips        |
| Code Block       | #0D1117 | Code snippet backgrounds           |
| Code Block Alt   | #161B22 | Inline code backgrounds            |

### Text Colors

| Token          | Value   | Usage                            |
| -------------- | ------- | -------------------------------- |
| Primary Text   | #E6EDF3 | Headlines, primary content       |
| Secondary Text | #8B949E | Body text, descriptions          |
| Tertiary Text  | #6E7681 | Captions, comments, metadata     |
| Muted Text     | #484F58 | Placeholders, disabled text      |
| Blue Text      | #58A6FF | Links, active states, highlights |
| Green Text     | #3FB950 | Success states, code strings     |
| Orange Text    | #D29922 | Warning states, code keywords    |
| Red Text       | #F85149 | Error states, code deletions     |

### Border Colors

| Token          | Value   | Usage                             |
| -------------- | ------- | --------------------------------- |
| Default Border | #30363D | Card borders, dividers            |
| Subtle Border  | #21262D | Light separators                  |
| Active Border  | #58A6FF | Focused inputs, active indicators |
| Code Border    | #30363D | Code block borders                |

### Accent Colors

| Token          | Value     | Usage                             |
| -------------- | --------- | --------------------------------- |
| Primary Accent | #58A6FF   | CTA buttons, links, active states |
| Accent Muted   | #58A6FF20 | Tinted backgrounds, badge fills   |
| Accent Hover   | #79C0FF   | Button hover state                |
| Success        | #3FB950   | Positive indicators, checkmarks   |
| Warning        | #D29922   | Alert states                      |
| Error          | #F85149   | Error states, destructive actions |
| On Accent      | #FFFFFF   | Text on blue backgrounds          |

## Typography

### Font Families

| Role              | Family         | Usage                                                            |
| ----------------- | -------------- | ---------------------------------------------------------------- |
| Display / Code    | JetBrains Mono | Hero headlines, feature titles, code snippets, terminal elements |
| Body / Functional | Inter          | Body text, navigation, buttons, descriptions                     |

### Type Scale

| Level      | Size | Font                   | Weight  | Usage                                   |
| ---------- | ---- | ---------------------- | ------- | --------------------------------------- |
| Hero       | 56px | JetBrains Mono         | 700     | Primary hero headline                   |
| Display    | 44px | JetBrains Mono         | 600     | Large section headings                  |
| Title 1    | 32px | JetBrains Mono         | 600     | Section headings                        |
| Title 2    | 24px | JetBrains Mono         | 500     | Subsection headings                     |
| Title 3    | 18px | Inter                  | 600     | Card titles, feature names              |
| Body Large | 18px | Inter                  | 400     | Hero subtitle, lead paragraphs          |
| Body       | 16px | Inter                  | 400     | Descriptions, marketing copy            |
| Code       | 14px | JetBrains Mono         | 400     | Code blocks, CLI commands               |
| Label      | 14px | Inter                  | 500     | Navigation, button text                 |
| Caption    | 12px | Inter / JetBrains Mono | 400-500 | Metadata, terminal prompts, tech labels |
| Micro      | 11px | Inter                  | 500     | Badges, overlines (uppercase)           |

### Font Weights

| Weight   | Value | Usage                                   |
| -------- | ----- | --------------------------------------- |
| Regular  | 400   | Body text, code blocks                  |
| Medium   | 500   | Labels, navigation, subsection headings |
| Semibold | 600   | Section headings, display text          |
| Bold     | 700   | Hero headline                           |

### Letter Spacing

- Hero (56px): -2px
- Display (44px): -1.5px
- Section headings (32px): -1px
- Uppercase labels: +2px
- Code/monospace: 0px
- Body text: 0px

### Line Height

- Hero (56px): 1.05
- Display (44px): 1.1
- Headings (24-32px): 1.2
- Body (16-18px): 1.6
- Code (14px): 1.5
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value   | Usage                                     |
| ------- | ----------------------------------------- |
| 4px     | Inline icon+text pairs, dot separators    |
| 8px     | Badge groups, tight clusters              |
| 12px    | Navigation items, button groups           |
| 16px    | Card content sections, code block padding |
| 24px    | Between cards in grid, feature list items |
| 32px    | Section internal content                  |
| 48px    | Between content blocks within sections    |
| 64-80px | Between page sections                     |

### Padding Scale

| Value    | Usage                         |
| -------- | ----------------------------- |
| [8, 16]  | Compact buttons, inline code  |
| [10, 20] | Standard buttons              |
| [12, 24] | CTA buttons                   |
| 16px     | Code block padding            |
| 24px     | Card padding (compact)        |
| 32px     | Card padding (standard)       |
| [80, 0]  | Section vertical padding      |
| [120, 0] | Hero section vertical padding |

### Layout Pattern

- Page width: 1280px max
- Content container: 1120px centered, padding [0, 32]
- Hero section: centered text + code block or terminal animation below
- Feature grid: 3-column, gap 24px, monospace-titled cards
- Code showcase: full-width code block with syntax highlighting
- Integration logos: single-row bar, grayscale/muted developer tool logos
- Pricing: 3-column, gap 24px, popular plan with blue border
- Navigation: fixed top, #0D1117, height 64px, subtle bottom border (#30363D)
- Footer: dark (#0D1117), minimal, repository-style links

## Corner Radius

| Value  | Usage                                     | Rationale                     |
| ------ | ----------------------------------------- | ----------------------------- |
| 4px    | Inline code, small badges                 | Minimal code-block softening  |
| 6px    | Buttons, input fields, dropdown menus     | GitHub-inspired button radius |
| 8px    | Feature cards, pricing cards, code blocks | Primary container radius      |
| 12px   | Large cards, hero panels                  | Maximum container radius      |
| 9999px | Status dots, avatar frames                | Full circle                   |

Design rationale: Compact radii (6-8px) mirror the GitHub and VS Code design systems that developers interact with daily. This creates instant visual familiarity — the page looks and feels like the tools the audience already uses. Avoiding generous radii prevents the design from feeling consumer-oriented or non-technical.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

terminal, code, git-branch, git-pull-request, github, zap, shield, lock, cloud, server, database, cpu, layers, package, download, arrow-right, chevron-right, chevron-down, menu, x, check, star, eye, copy, external-link, command, hash

### Icon Sizes

| Size    | Usage                                 |
| ------- | ------------------------------------- |
| 14px    | Inline with code, metadata indicators |
| 16px    | Navigation items, list item bullets   |
| 20px    | Card header icons, button icons       |
| 24-32px | Feature section icons, header actions |

### Icon Color States

| State     | Color   | Usage                             |
| --------- | ------- | --------------------------------- |
| Blue      | #58A6FF | Primary actions, links, selected  |
| Default   | #8B949E | Standard icons, secondary actions |
| Muted     | #484F58 | Disabled, placeholder icons       |
| Bright    | #E6EDF3 | Icons on dark surfaces            |
| Green     | #3FB950 | Success, check indicators         |
| On Accent | #FFFFFF | Icons on blue backgrounds         |
