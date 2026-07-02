---
name: 'saas-modern-light'
tags: [minimal, clean, light-mode, blue-accent, modern, rounded, webapp]
platform: webapp
---

## Style Summary

A clean, refined SaaS product interface inspired by modern tools like Linear, Vercel, and Raycast. Pure white backgrounds (#FFFFFF) with subtle depth through layered surfaces and soft shadows create a polished, premium feel. The confident blue accent (#2563EB) provides clear interactive affordance without overwhelming the neutral palette. Geist, Vercel's purpose-built typeface, delivers exceptional clarity at all sizes with its balanced geometry and optimized screen rendering, serving as both heading and body font with weight variation for hierarchy.

Key aesthetics:

- **Pure white canvas**: #FFFFFF background with subtle gray (#F9FAFB) for depth layering
- **Confident blue accent**: #2563EB for primary actions and interactive states, never decorative
- **Geist mono-family**: Single font family with weight variation for clean, systematic hierarchy
- **Refined radii**: 8-12px corners feel modern and precise without becoming playful
- **Subtle shadows**: Soft multi-layer shadows instead of borders for premium depth perception
- **Systematic spacing**: Consistent 4px-based grid for pixel-perfect alignment

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                                |
| --------------- | ------- | ---------------------------------------------------- |
| Page Background | #FFFFFF | Root page background                                 |
| Card Surface    | #FFFFFF | Cards, panels, elevated containers                   |
| Inset Surface   | #F9FAFB | Input backgrounds, table stripes, secondary surfaces |
| Hover Surface   | #F3F4F6 | Row hovers, interactive element hover states         |
| Sidebar Surface | #FAFAFA | Optional sidebar background                          |

### Text Colors

| Token          | Value   | Usage                                     |
| -------------- | ------- | ----------------------------------------- |
| Primary Text   | #111827 | Headings, primary content, key values     |
| Secondary Text | #4B5563 | Body text, descriptions                   |
| Tertiary Text  | #9CA3AF | Captions, timestamps, helper text         |
| Muted Text     | #D1D5DB | Placeholders, disabled text               |
| Accent Text    | #2563EB | Active nav items, links, interactive text |

### Border Colors

| Token          | Value   | Usage                                      |
| -------------- | ------- | ------------------------------------------ |
| Default Border | #E5E7EB | Card borders, dividers, input outlines     |
| Subtle Border  | #F3F4F6 | Light separators, inset borders            |
| Active Border  | #2563EB | Focused inputs, active tab indicators      |
| Strong Border  | #D1D5DB | Emphasized dividers, header bottom borders |

### Accent Colors

| Token          | Value     | Usage                                               |
| -------------- | --------- | --------------------------------------------------- |
| Primary Accent | #2563EB   | Primary buttons, active states, key indicators      |
| Accent Hover   | #1D4ED8   | Button hover, pressed states                        |
| Accent Light   | #2563EB15 | Selected row backgrounds, hover tints               |
| Accent Muted   | #2563EB30 | Badge fills, progress backgrounds                   |
| Success        | #16A34A   | Positive metrics, success states, online indicators |
| Success Light  | #16A34A15 | Success badge backgrounds                           |
| Warning        | #D97706   | Alert indicators, pending states                    |
| Warning Light  | #D9770615 | Warning badge backgrounds                           |
| Error          | #DC2626   | Error states, destructive actions                   |
| Error Light    | #DC262615 | Error badge backgrounds                             |

## Typography

### Font Families

| Role              | Family | Usage                                                   |
| ----------------- | ------ | ------------------------------------------------------- |
| Headings          | Geist  | Page titles, section headings, card titles (weight 600) |
| Body / Functional | Geist  | Body text, labels, buttons, navigation (weight 400)     |

### Type Scale

| Level      | Size | Font  | Weight | Usage                                |
| ---------- | ---- | ----- | ------ | ------------------------------------ |
| Display    | 36px | Geist | 700    | Page titles, hero headings           |
| Title 1    | 24px | Geist | 600    | Section headings                     |
| Title 2    | 18px | Geist | 600    | Card titles, subsection headings     |
| Title 3    | 15px | Geist | 600    | List headings, compact card titles   |
| Body Large | 16px | Geist | 400    | Lead descriptions, featured text     |
| Body       | 14px | Geist | 400    | Standard body text, descriptions     |
| Label      | 13px | Geist | 500    | Button text, field labels, nav items |
| Caption    | 12px | Geist | 400    | Timestamps, secondary metadata       |
| Small      | 11px | Geist | 500    | Badges, status labels                |
| Micro      | 10px | Geist | 500    | Auxiliary labels, inline indicators  |

### Font Weights

| Weight   | Value | Usage                                     |
| -------- | ----- | ----------------------------------------- |
| Regular  | 400   | Body text, descriptions, captions         |
| Medium   | 500   | Labels, buttons, navigation items, badges |
| Semibold | 600   | Card titles, section headings             |
| Bold     | 700   | Display headings, page titles             |

### Letter Spacing

- Display (36px): -1px
- Section headings (18-24px): -0.5px
- Title 3 / Label (13-15px): -0.2px
- Uppercase labels: +1px
- Body text: 0px

### Line Height

- Display (36px): 1.0
- Headings (18-24px): 1.2
- Body (14-16px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                      |
| ----- | ------------------------------------------ |
| 4px   | Inline icon+text pairs, tight groups       |
| 6px   | Status dot to label, badge internals       |
| 8px   | Compact element groups, metric pairs       |
| 12px  | Between list items, nav items, form fields |
| 16px  | Card internal sections, button groups      |
| 20px  | Between cards in a grid                    |
| 24px  | Content section gaps                       |
| 32px  | Major section breaks                       |
| 48px  | Top-level page sections                    |

### Padding Scale

| Value    | Usage                                |
| -------- | ------------------------------------ |
| [4, 8]   | Compact badges, inline tags          |
| [6, 12]  | Small buttons, status pills          |
| [8, 16]  | Standard buttons, input fields       |
| [10, 20] | Large buttons, search bars           |
| 16px     | Card padding (compact)               |
| 20px     | Card padding (standard)              |
| 24px     | Large card padding, feature sections |
| [24, 28] | Content area padding                 |
| [16, 20] | Sidebar section padding              |

### Layout Pattern

- Screen width: 1440px
- Navigation: top bar (64px) or 220px sidebar, clean with minimal chrome
- Content area: fill_container, vertical, gap 24, padding [24, 28]
- Cards use subtle multi-layer shadow (0 1px 2px rgba(0,0,0,0.04), 0 4px 12px rgba(0,0,0,0.03))
- Optional 1px border (#E5E7EB) on cards for additional definition
- Tables: clean rows with hover state (#F3F4F6), no visible row borders

## Corner Radius

| Value | Usage                                  | Rationale                           |
| ----- | -------------------------------------- | ----------------------------------- |
| 4px   | Inline badges, code snippets, tooltips | Subtle softening for small elements |
| 6px   | Input fields, small buttons, dropdowns | Crisp interactive radius            |
| 8px   | Standard buttons, popovers, tags       | Primary interactive radius          |
| 12px  | Cards, modals, panels, dialog windows  | Primary container radius            |
| 16px  | Large feature cards, onboarding panels | Generous for prominent surfaces     |
| 100px | Pill buttons, toggle switches, avatars | Full capsule/circle shape           |

Design rationale: The 4-12px primary radius range defines the modern SaaS aesthetic — refined enough to signal quality engineering, controlled enough to maintain professional density. Cards at 12px with soft shadows feel premium without being decorative. Buttons at 8px strike the balance between the too-sharp (4px) of enterprise tools and the too-soft (16px+) of consumer apps. This is the radius language of tools built for daily professional use.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (clean, systematic stroke style)

### Commonly Used Icons

layout-dashboard, bar-chart-2, users, settings, folder, file-text, search, bell, chevron-down, chevron-right, plus, x, edit, trash-2, check, check-circle, alert-circle, info, external-link, copy, download, upload, filter, calendar, clock, refresh-cw, command, zap, globe, link, mail, lock

### Icon Sizes

| Size | Usage                                              |
| ---- | -------------------------------------------------- |
| 14px | Inline indicators, table actions, badge icons      |
| 16px | Navigation icons, button leading icons, list icons |
| 20px | Card header actions, toolbar icons                 |
| 24px | Page header actions, primary navigation            |

### Icon Color States

| State     | Color   | Usage                              |
| --------- | ------- | ---------------------------------- |
| Active    | #2563EB | Active navigation, selected states |
| Default   | #4B5563 | Standard icons, secondary actions  |
| Muted     | #D1D5DB | Disabled icons, placeholder states |
| On Accent | #FFFFFF | Icons on blue-accent backgrounds   |
| Success   | #16A34A | Status online, positive indicators |
