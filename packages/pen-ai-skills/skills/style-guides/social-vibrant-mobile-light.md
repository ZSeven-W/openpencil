---
name: 'social-vibrant-mobile-light'
tags: [vibrant, light-mode, colorful, playful, rounded, mobile, friendly, gradient]
platform: mobile
---

## Style Summary

A lively, expressive mobile interface designed for social media, content sharing, and community apps. The pure white background (#FFFFFF) lets vibrant violet (#8B5CF6) and pink (#EC4899) accents pop with full saturation, creating an energetic, Instagram/TikTok-inspired aesthetic. Poppins provides a friendly geometric personality at every typographic level, with rounded letterforms that mirror the generous 16-24px corner radii. Gradient accents on buttons and featured content add a contemporary, eye-catching quality that encourages engagement and interaction.

Key aesthetics:

- **Clean white canvas**: #FFFFFF background lets colorful accents and user content take center stage
- **Violet-pink duo accent**: #8B5CF6 and #EC4899 combine for gradient CTAs and playful highlights
- **Friendly geometry**: Poppins typeface has naturally rounded letterforms that feel approachable and modern
- **Generous radii**: 16-24px corners create soft, pillowy containers that invite touch interaction
- **Gradient buttons**: Violet-to-pink gradients on primary actions create visual magnetism
- **Pill tab bar**: 100px radius bottom navigation with violet active indicator

## Color System

### Core Backgrounds

| Token           | Value   | Usage                                           |
| --------------- | ------- | ----------------------------------------------- |
| Page Background | #FFFFFF | Root screen background                          |
| Card Surface    | #FFFFFF | Cards, post containers                          |
| Inset Surface   | #F5F3FF | Input backgrounds, search bars, recessed areas  |
| Subtle Surface  | #FDF2F8 | Story highlights, featured sections (pink tint) |
| Tab Bar Surface | #FFFFFF | Bottom navigation pill fill                     |

### Text Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Primary Text   | #1E1B2E | Headings, usernames, primary labels    |
| Secondary Text | #6B6580 | Body text, descriptions, captions      |
| Tertiary Text  | #A59FBA | Timestamps, placeholders, metadata     |
| Muted Text     | #D4D0E0 | Disabled text, background labels       |
| Accent Text    | #8B5CF6 | Active tabs, links, highlighted values |

### Border Colors

| Token          | Value   | Usage                                  |
| -------------- | ------- | -------------------------------------- |
| Default Border | #E9E5F5 | Card borders, input outlines, dividers |
| Subtle Border  | #F3F0FA | Light separators, section breaks       |
| Active Border  | #8B5CF6 | Focused inputs, active states          |

### Accent Colors

| Token            | Value     | Usage                                                 |
| ---------------- | --------- | ----------------------------------------------------- |
| Primary Accent   | #8B5CF6   | Active states, primary buttons, tab indicator         |
| Secondary Accent | #EC4899   | Gradient end, hearts, reactions, secondary highlights |
| Accent Light     | #8B5CF615 | Tinted backgrounds, selected row highlights           |
| Accent Muted     | #EDE9FE   | Badge backgrounds, subtle indicators                  |
| Success          | #10B981   | Verified badges, positive states                      |
| Warning          | #F59E0B   | Alert indicators, pending states                      |
| Error            | #EF4444   | Error states, destructive actions                     |

## Typography

### Font Families

| Role              | Family  | Usage                                         |
| ----------------- | ------- | --------------------------------------------- |
| Display / Heading | Poppins | Screen titles, section headings, hero metrics |
| Body / Functional | Poppins | Body text, labels, buttons, navigation        |

### Type Scale

| Level   | Size | Font    | Weight | Usage                                |
| ------- | ---- | ------- | ------ | ------------------------------------ |
| Display | 32px | Poppins | 700    | Hero titles, profile stats           |
| Title 1 | 24px | Poppins | 700    | Screen titles                        |
| Title 2 | 18px | Poppins | 600    | Section headings, group names        |
| Title 3 | 16px | Poppins | 600    | Card titles, usernames               |
| Body    | 14px | Poppins | 400    | Post text, descriptions              |
| Label   | 13px | Poppins | 500    | Field labels, button text            |
| Caption | 12px | Poppins | 400    | Timestamps, secondary info           |
| Small   | 11px | Poppins | 500    | Badges, follower counts              |
| Micro   | 10px | Poppins | 600    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight   | Value | Usage                                      |
| -------- | ----- | ------------------------------------------ |
| Regular  | 400   | Body text, descriptions, captions          |
| Medium   | 500   | Labels, buttons, navigation items          |
| Semibold | 600   | Section headings, card titles, usernames   |
| Bold     | 700   | Screen titles, hero display, profile stats |

### Letter Spacing

- Display (32px): -0.5px
- Section headings (16-24px): -0.3px
- Uppercase tab labels: +1px
- Body text: 0px

### Line Height

- Display (32px): 1.1
- Headings (16-24px): 1.25
- Body (14px): 1.5
- Captions (10-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                    |
| ----- | ---------------------------------------- |
| 2px   | Tight inline pairs                       |
| 4px   | Tab icon to label, inline icon groups    |
| 6px   | Avatar to username, reaction icon groups |
| 8px   | Compact card content, comment entries    |
| 12px  | Between list items, story circles        |
| 16px  | Card internal sections, post-to-actions  |
| 20px  | Between cards, post feed spacing         |
| 24px  | Section gaps within content              |
| 32px  | Top-level screen section breaks          |

### Padding Scale

| Value            | Usage                                    |
| ---------------- | ---------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only) |
| [12, 21, 21, 21] | Tab bar section outer padding            |
| 4px              | Tab bar pill inner padding               |
| [8, 16]          | Input fields, search bars                |
| [10, 20]         | Standard buttons                         |
| [12, 24]         | Large buttons, CTA buttons               |
| 16px             | Compact card padding                     |
| 20px             | Standard card padding                    |
| 24px             | Feature card padding, story sections     |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 20-24
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Story row: horizontal scroll, 12px gap between avatar circles

## Corner Radius

| Value | Usage                                                 | Rationale                                       |
| ----- | ----------------------------------------------------- | ----------------------------------------------- |
| 12px  | Small buttons, compact badges, tag chips              | Soft baseline for small elements                |
| 16px  | Standard cards, post containers, inputs               | Primary container radius, friendly and inviting |
| 20px  | Large cards, search bars, image containers            | Generous interactive radius                     |
| 24px  | Hero cards, feature sections, modal sheets            | Maximum softness for featured content           |
| 100px | Tab bar pill, story avatars, round buttons, pill tags | Full capsule shape                              |

Design rationale: Generous 16-24px radii create the soft, pillow-like containers that define social media aesthetics. The rounded forms mirror Poppins' circular letterforms and encourage casual, playful interaction. Story avatars and tab bar use 100px for full circles and capsule shapes, reinforcing the friendly, approachable personality throughout.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps (soft strokes match the playful aesthetic)

### Commonly Used Icons

home, search, heart, message-circle, send, plus, camera, image, video, user, users, bell, bookmark, share-2, more-horizontal, smile, thumbs-up, music, globe, hash, at-sign, trending-up, star, x, chevron-right

### Icon Sizes

| Size | Usage                                            |
| ---- | ------------------------------------------------ |
| 14px | Inline text indicators, reaction counts          |
| 18px | Tab bar icons, list item leading icons           |
| 20px | Card action icons, post action bar               |
| 24px | Header actions, notification bell, camera button |

### Icon Color States

| State     | Color   | Usage                                |
| --------- | ------- | ------------------------------------ |
| Active    | #8B5CF6 | Selected tab icon, active navigation |
| Liked     | #EC4899 | Heart filled, reaction active (pink) |
| Default   | #A59FBA | Inactive tabs, secondary actions     |
| Muted     | #D4D0E0 | Disabled states, placeholder icons   |
| On Accent | #FFFFFF | Icons on violet/pink backgrounds     |
