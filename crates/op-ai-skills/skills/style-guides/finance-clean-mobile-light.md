---
name: 'finance-clean-mobile-light'
tags: [fintech, clean, light-mode, blue-accent, professional, mobile, data-focused]
platform: mobile
---

## Style Scope

This guide is self-contained. Apply its palette, radius scale, spacing, shadows, and component treatments only when this exact guide is selected; do not borrow food, social, dashboard, luxury, terminal, or other guide-specific patterns into another style. Treat unnamed layout frames as structural by default: no fill, stroke, cornerRadius, or shadow unless the node is intentionally a card, input, button, badge, media mask, navigation surface, or other visible component. Avoid decorative wrapper shells around components; hierarchy should come from spacing, typography, and this guide's explicit surfaces.

## Style Summary

A precise, trust-building mobile interface designed for banking, fintech, and investment applications. The cool off-white background (#F8FAFC) provides a clinical clarity that reinforces accuracy and reliability, while the teal accent (#0D9488) signals growth and stability without the aggressive energy of blue or green alone. DM Sans delivers clean, no-nonsense readability at every scale, and DM Mono ensures numerical data—balances, rates, percentages—is rendered with monospaced precision. Restrained 8-12px corner radii project professionalism, and the overall aesthetic prioritizes data legibility and user confidence above all else.

Key aesthetics:

- **Clinical clarity**: #F8FAFC background with structured card hierarchy for maximum data readability
- **Trust teal accent**: #0D9488 conveys growth and stability, balancing warmth and professionalism
- **Dual-purpose type**: DM Sans for headers and body, DM Mono for financial data and figures
- **Professional radii**: 8-12px corners keep containers structured and business-appropriate
- **Data-first layout**: Generous spacing around numbers, clear visual hierarchy for financial metrics
- **Integrated bottom nav**: full-width bottom navigation with teal active indicator

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                            |
| --------------- | ------- | ------------------------------------------------ |
| Page Background | #F8FAFC | Root screen background (cool off-white)          |
| Card Surface    | #FFFFFF | Cards, account containers, transaction items     |
| Inset Surface   | #F1F5F9 | Input backgrounds, search bars, grouped sections |
| Subtle Surface  | #F0FDFA | Positive balance highlight, growth indicators    |
| Tab Bar Surface | #FFFFFF | Integrated bottom navigation surface                      |

### Text Colors

| Token          | Value   | Usage                                        |
| -------------- | ------- | -------------------------------------------- |
| Primary Text   | #0F172A | Headings, balance values, primary labels     |
| Secondary Text | #64748B | Body text, descriptions, transaction details |
| Tertiary Text  | #94A3B8 | Timestamps, captions, placeholders           |
| Muted Text     | #CBD5E1 | Disabled text, background labels             |
| Accent Text    | #0D9488 | Active tabs, links, positive values          |

### Border Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Default Border | #E2E8F0 | Card borders, input outlines, dividers |
| Subtle Border  | #F1F5F9 | Light separators, section breaks       |
| Active Border  | #0D9488 | Focused inputs, active states          |

### Accent Colors

| Token          | Value     | Usage                                            |
| -------------- | --------- | ------------------------------------------------ |
| Primary Accent | #0D9488   | Active states, primary buttons, tab indicator    |
| Accent Light   | #0D948815 | Tinted backgrounds, selected row highlights      |
| Accent Muted   | #CCFBF1   | Badge backgrounds, subtle indicators             |
| Success        | #059669   | Positive returns, gains, completed transfers     |
| Warning        | #D97706   | Pending transactions, review required            |
| Error          | #DC2626   | Failed transactions, negative returns, overdraft |
| Negative       | #EF4444   | Loss values, declined, insufficient funds        |

## Typography

### Font Families

| Role              | Family  | Usage                                                   |
| ----------------- | ------- | ------------------------------------------------------- |
| Display / Heading | DM Sans | Screen titles, section headings, account names          |
| Body / Functional | DM Sans | Body text, labels, buttons, navigation                  |
| Data / Mono       | DM Mono | Balance values, transaction amounts, rates, percentages |

### Type Scale

| Level      | Size | Font    | Weight | Usage                                |
| ---------- | ---- | ------- | ------ | ------------------------------------ |
| Display    | 34px | DM Mono | 600    | Primary balance, portfolio value     |
| Title 1    | 24px | DM Sans | 600    | Screen titles                        |
| Title 2    | 18px | DM Sans | 600    | Section headings, account names      |
| Title 3    | 16px | DM Sans | 500    | Card titles, transaction categories  |
| Body       | 14px | DM Sans | 400    | Descriptions, transaction details    |
| Label      | 13px | DM Sans | 500    | Field labels, button text            |
| Data       | 14px | DM Mono | 500    | Transaction amounts, rates           |
| Data Small | 12px | DM Mono | 500    | Secondary amounts, percentages       |
| Caption    | 12px | DM Sans | 400    | Timestamps, secondary info           |
| Micro      | 10px | DM Sans | 500    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight   | Value | Usage                                            |
| -------- | ----- | ------------------------------------------------ |
| Regular  | 400   | Body text, descriptions, captions                |
| Medium   | 500   | Labels, buttons, card titles, data values        |
| Semibold | 600   | Section headings, screen titles, primary balance |

### Letter Spacing

- Display balance (34px): -0.5px
- Section headings (16-24px): -0.3px
- Uppercase tab labels: +1px
- Monospace data: +0.5px (improved digit readability)
- Body text: 0px

### Line Height

- Display (34px): 1.0
- Headings (16-24px): 1.2
- Body (14px): 1.5
- Data (12-14px): 1.3
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                         |
| ----- | --------------------------------------------- |
| 2px   | Tight inline pairs                            |
| 4px   | Tab icon to label, currency symbol to amount  |
| 6px   | Status indicator to text, rate change arrows  |
| 8px   | Compact card content, transaction entry items |
| 12px  | Between list items, form fields               |
| 16px  | Card internal sections, search-to-content     |
| 20px  | Between cards, account card list              |
| 24px  | Section gaps within content                   |
| 32px  | Top-level screen section breaks               |

### Padding Scale

| Value            | Usage                                    |
| ---------------- | ---------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only) |
| [12, 21, 21, 21] | Tab bar section outer padding            |
| 4px              | Integrated tab bar inner padding               |
| [8, 16]          | Input fields, search bars                |
| [10, 20]         | Standard buttons                         |
| [12, 24]         | Large buttons, CTA buttons               |
| 16px             | Compact card padding, transaction items  |
| 20px             | Standard card padding                    |
| 24px             | Account card padding, large sections     |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24
- Status bar: 62px, standard iOS
- Integrated tab bar: height 62-72px, cornerRadius 0-12, padding 0-4, full-width in page flow
- Tab items: fill_container, vertical, gap 4, center aligned
- Transaction list: card surface with dividers, 16px internal padding

### Mobile Composition Guardrails

- Keep top rhythm compact: status bar, header, and first content block should feel connected; avoid empty 80px+ bands unless the design is intentionally image-led.
- Search should be one clean row: a neutral input plus optional filter button. Do not wrap the input in a second tinted rounded shell; the grouping frame should stay structural and transparent.
- Horizontal rails and carousel viewport frames stay transparent: no white rounded backing frame, no extra shadow. Only individual cards, images, or intentional promo panels receive fill, radius, or elevation.
- Favorite, like, bookmark, and heart actions on image cards should be icon-only overlays or very subtle translucent hit areas; do not add circular white bubbles, borders, or shadows around the heart.
- Bottom navigation is integrated into the screen flow with a quiet divider or active indicator; avoid floating capsule navigation unless a prompt explicitly asks for a floating nav pattern.
- Use 9999px radius only for true circles or tiny capsules such as avatars, status dots, and short badges; never for page sections, search shells, carousels, card action buttons, or full navigation bars.

## Corner Radius

| Value | Usage                                                | Rationale                                |
| ----- | ---------------------------------------------------- | ---------------------------------------- |
| 6px   | Small badges, tag chips, compact indicators          | Minimal softening for data elements      |
| 8px   | Standard buttons, input fields, transaction items    | Subtle professional radius               |
| 12px  | Standard cards, account containers, grouped sections | Primary container radius                 |
| 16px  | Large cards, modal sheets, feature sections          | Comfortable radius for prominent content |
| 9999px | Avatars, status dots, short badges only | True circles/capsules for semantic micro-elements; never navigation, search, carousels, or card action buttons |

Design rationale: Restrained 8-12px radii project the structured professionalism that financial applications demand. Users need to trust that their money is handled with precision—overly rounded corners can undermine that perception. The tight range (6-16px) keeps every element feeling calculated and deliberate, while still being comfortable on mobile touch targets.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (clean strokes reinforce the professional aesthetic)

### Commonly Used Icons

home, wallet, credit-card, banknote, trending-up, trending-down, arrow-up-right, arrow-down-left, send, receipt, building-2, shield-check, lock, eye, eye-off, copy, qr-code, scan, bell, search, plus, x, chevron-right, circle-check, clock

### Icon Sizes

| Size | Usage                                                |
| ---- | ---------------------------------------------------- |
| 14px | Inline indicators, rate change arrows                |
| 18px | Tab bar icons, list item leading icons               |
| 20px | Card action icons, transaction category icons        |
| 24px | Header actions, notification bell, prominent buttons |

### Icon Color States

| State     | Color   | Usage                                |
| --------- | ------- | ------------------------------------ |
| Active    | #0D9488 | Selected tab icon, active navigation |
| Default   | #94A3B8 | Inactive tabs, secondary actions     |
| Muted     | #CBD5E1 | Disabled states, placeholder icons   |
| On Accent | #FFFFFF | Icons on teal-colored backgrounds    |
