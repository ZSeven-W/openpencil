---
name: 'education-friendly-light'
tags: [education, friendly, light-mode, rounded, colorful, playful, landing-page]
platform: webapp
---

## Style Summary

A warm, inviting education platform landing page with a creamy off-white (#FFFBF0) background and a dual-accent palette of amber (#F59E0B) and blue (#3B82F6). This style feels approachable, trustworthy, and encouraging — designed to make learning feel accessible rather than intimidating. Nunito rounded sans-serif for both headings and body creates a friendly, cohesive typographic voice, while generous 16-20px corner radii soften every container into welcoming, touchable shapes.

Key aesthetics:

- **Warm cream canvas**: #FFFBF0 background creates a book-page warmth that pure white lacks
- **Amber + blue duo**: #F59E0B for primary CTAs and highlights, #3B82F6 for links and secondary actions
- **Nunito rounded type**: Rounded terminals on every character reinforce the friendly, educational personality
- **Generous soft radii**: 16-20px corners on cards, 12px on buttons — everything feels pillow-soft
- **Illustration-friendly**: Large whitespace areas designed to accommodate colorful illustrations
- **Clear hierarchy**: Strong heading sizes with ample spacing guide the eye through content sections

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                         |
| --------------- | ------- | --------------------------------------------- |
| Page Background | #FFFBF0 | Root page background (warm cream)             |
| Section Alt     | #FFF7E6 | Alternating section background (deeper cream) |
| Card Surface    | #FFFFFF | Feature cards, testimonial cards              |
| Inset Surface   | #FEF3C7 | Input backgrounds, highlighted areas          |
| Blue Tint       | #EFF6FF | Blue-tinted feature sections                  |

### Text Colors

| Token          | Value   | Usage                                   |
| -------------- | ------- | --------------------------------------- |
| Primary Text   | #1C1917 | Headlines, primary content (warm black) |
| Secondary Text | #57534E | Body text, descriptions                 |
| Tertiary Text  | #A8A29E | Captions, metadata                      |
| Muted Text     | #D6D3D1 | Placeholders, disabled text             |
| Amber Text     | #D97706 | Active nav, amber highlights            |
| Blue Text      | #2563EB | Links, secondary highlights             |

### Border Colors

| Token          | Value   | Usage                           |
| -------------- | ------- | ------------------------------- |
| Default Border | #E7E5E4 | Card borders, dividers          |
| Subtle Border  | #F5F5F4 | Light separators                |
| Amber Border   | #F59E0B | Active inputs, amber indicators |
| Blue Border    | #3B82F6 | Focus rings, blue indicators    |

### Accent Colors

| Token            | Value   | Usage                                  |
| ---------------- | ------- | -------------------------------------- |
| Primary Accent   | #F59E0B | CTA buttons, primary highlights, amber |
| Secondary Accent | #3B82F6 | Secondary buttons, links, blue         |
| Amber Light      | #FEF3C7 | Amber tinted backgrounds               |
| Blue Light       | #DBEAFE | Blue tinted backgrounds                |
| Amber Dark       | #D97706 | Amber button hover                     |
| Blue Dark        | #1D4ED8 | Blue button hover                      |
| Success          | #16A34A | Completed lessons, progress            |
| Error            | #DC2626 | Error states                           |

## Typography

### Font Families

| Role              | Family | Usage                                   |
| ----------------- | ------ | --------------------------------------- |
| Headings          | Nunito | All headings, hero text, section titles |
| Body / Functional | Nunito | Body text, navigation, buttons, labels  |

### Type Scale

| Level      | Size | Font   | Weight | Usage                               |
| ---------- | ---- | ------ | ------ | ----------------------------------- |
| Hero       | 56px | Nunito | 700    | Primary hero headline               |
| Display    | 44px | Nunito | 700    | Large section headings              |
| Title 1    | 32px | Nunito | 700    | Section headings                    |
| Title 2    | 24px | Nunito | 600    | Subsection headings, feature titles |
| Title 3    | 20px | Nunito | 600    | Card titles                         |
| Body Large | 18px | Nunito | 400    | Hero subtitle, lead text            |
| Body       | 16px | Nunito | 400    | Descriptions, feature copy          |
| Label      | 14px | Nunito | 600    | Navigation, button text             |
| Caption    | 12px | Nunito | 600    | Overline labels, badges             |
| Micro      | 11px | Nunito | 400    | Fine print, footer text             |

### Font Weights

| Weight   | Value | Usage                                  |
| -------- | ----- | -------------------------------------- |
| Regular  | 400   | Body text, descriptions                |
| Semibold | 600   | Labels, navigation, card titles        |
| Bold     | 700   | Headlines, hero text, section headings |

### Letter Spacing

- Hero (56px): -1.5px
- Display (44px): -1px
- Section headings (32px): -0.5px
- Uppercase overlines: +1.5px
- Body text: 0px
- Button text: +0.5px

### Line Height

- Hero (56px): 1.1
- Display (44px): 1.15
- Headings (24-32px): 1.25
- Body (16-18px): 1.6
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value   | Usage                                  |
| ------- | -------------------------------------- |
| 4px     | Inline icon+text pairs                 |
| 8px     | Badge groups, tight clusters           |
| 12px    | Button groups, feature list items      |
| 16px    | Card content sections                  |
| 24px    | Between cards in grid, feature entries |
| 32px    | Section internal content               |
| 48px    | Between content blocks within sections |
| 64-80px | Between page sections                  |

### Padding Scale

| Value    | Usage                         |
| -------- | ----------------------------- |
| [10, 20] | Compact buttons, input fields |
| [12, 24] | Standard buttons              |
| [14, 28] | CTA buttons (generous)        |
| 24px     | Card padding (compact)        |
| 32px     | Card padding (standard)       |
| 40px     | Feature card padding          |
| [64, 0]  | Section vertical padding      |
| [96, 0]  | Hero section vertical padding |

### Layout Pattern

- Page width: 1280px max
- Content container: 1120px centered, padding [0, 32]
- Hero section: split layout (text left, illustration right) or centered text with illustration below
- Feature grid: 3-column, gap 24px, icon-led cards
- Stats row: 4-column, centered numbers with labels
- Testimonials: 3-column card grid or horizontal carousel
- CTA section: centered text on amber-tinted background, max-width 640px
- Navigation: sticky top, white, height 68px, rounded shadow
- Footer: 4-column links + newsletter signup

## Corner Radius

| Value  | Usage                                       | Rationale                |
| ------ | ------------------------------------------- | ------------------------ |
| 8px    | Input fields, small badges                  | Gentle but compact       |
| 12px   | Buttons, dropdown menus                     | Friendly button shape    |
| 16px   | Standard cards, feature cards               | Primary container radius |
| 20px   | Large cards, testimonial cards, hero cards  | Maximum softness         |
| 9999px | Pill badges, avatar frames, toggle switches | Full capsule             |

Design rationale: Generous radii (16-20px) create a warm, approachable feeling that makes educational content feel inviting rather than clinical. The rounded corners mirror Nunito's rounded terminals, creating a cohesive visual language that says "learning is fun and accessible." This pillow-soft approach deliberately contrasts with the sharp edges of enterprise or luxury styles.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (matches Nunito's rounded terminals)

### Commonly Used Icons

book-open, graduation-cap, trophy, star, check-circle, play-circle, users, heart, rocket, lightbulb, target, clock, calendar, arrow-right, chevron-right, chevron-down, menu, x, search, bell, bookmark, award, sparkles, puzzle, brain

### Icon Sizes

| Size    | Usage                                                    |
| ------- | -------------------------------------------------------- |
| 16px    | Inline with body text, list checks                       |
| 20px    | Navigation items, button icons                           |
| 24px    | Card header icons, feature indicators                    |
| 32-48px | Feature grid icons, hero decorative (in colored circles) |

### Icon Color States

| State    | Color   | Usage                       |
| -------- | ------- | --------------------------- |
| Amber    | #F59E0B | Primary actions, highlights |
| Blue     | #3B82F6 | Secondary actions, links    |
| Default  | #57534E | Standard icons              |
| Muted    | #A8A29E | Disabled, placeholder icons |
| On Amber | #FFFFFF | Icons on amber backgrounds  |
| On Blue  | #FFFFFF | Icons on blue backgrounds   |
