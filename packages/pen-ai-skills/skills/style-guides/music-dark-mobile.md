---
name: 'music-dark-mobile'
tags: [dark-mode, vibrant, gradient, bold-typography, mobile, electric, lime-accent]
platform: mobile
---

## Style Summary

An immersive dark mobile interface built for music streaming, podcast, and audio applications. The near-black background (#0A0A0A) creates a theater-like canvas where album art and vibrant accents can command full attention. Electric lime (#84CC16) and cyan (#22D3EE) provide a dual-accent system—lime for primary actions and progress, cyan for secondary highlights and audio visualizations. Manrope's bold geometric letterforms deliver confident, punchy headings that feel like concert posters, while its lighter weights maintain excellent readability for track listings and metadata. The 14-20px corner radii balance modern sharpness with the organic softness of sound waves.

Key aesthetics:

- **Theater-dark canvas**: #0A0A0A background creates an immersive listening environment
- **Lime-cyan duo accent**: #84CC16 for primary actions, #22D3EE for secondary highlights and visualizers
- **Concert poster type**: Manrope at 700-800 weights for bold, punchy display headings
- **Gradient energy**: Lime-to-cyan gradients on featured playlists and now-playing elements
- **Audio-inspired radii**: 14-20px corners evoke the smooth, organic contours of speaker forms
- **Pill tab bar**: 100px radius bottom navigation with lime active indicator

## Color System

### Core Backgrounds

| Token            | Value   | Usage                                 |
| ---------------- | ------- | ------------------------------------- |
| Page Background  | #0A0A0A | Root screen background (near black)   |
| Card Surface     | #151515 | Standard cards, track list containers |
| Elevated Surface | #1E1E1E | Now playing bar, modal sheets         |
| Gradient Start   | #151515 | Gradient card dark edge               |
| Gradient End     | #1A2A1A | Gradient card lime-tinted edge        |
| Tab Bar Surface  | #151515 | Bottom navigation pill fill           |

### Text Colors

| Token          | Value   | Usage                                          |
| -------------- | ------- | ---------------------------------------------- |
| Primary Text   | #FFFFFF | Track titles, artist names, primary labels     |
| Secondary Text | #A3A3A3 | Body text, album descriptions                  |
| Tertiary Text  | #6B6B6B | Timestamps, track durations, inactive labels   |
| Muted Text     | #3B3B3B | Placeholders, disabled text                    |
| Accent Text    | #84CC16 | Active states, now playing, highlighted values |

### Border Colors

| Token          | Value     | Usage                                |
| -------------- | --------- | ------------------------------------ |
| Default Border | #2A2A2A   | Card borders, dividers               |
| Subtle Border  | #1E1E1E   | Light separators                     |
| Active Border  | #84CC16   | Focused inputs, active tab indicator |
| Glow Border    | #84CC1640 | Subtle lime glow on active elements  |

### Accent Colors

| Token            | Value     | Usage                                                   |
| ---------------- | --------- | ------------------------------------------------------- |
| Primary Accent   | #84CC16   | Active states, play button, progress bar, tab indicator |
| Secondary Accent | #22D3EE   | Visualizer, secondary highlights, alt buttons           |
| Accent Glow      | #84CC1666 | Glow effects, now-playing halo (40% opacity)            |
| Accent Muted     | #84CC1620 | Tinted backgrounds, badge fills                         |
| Accent On-Dark   | #0A0A0A   | Text/icons on lime backgrounds (black)                  |
| Success          | #22C55E   | Download complete, saved to library                     |
| Warning          | #F59E0B   | Offline mode indicator, expiring content                |
| Error            | #EF4444   | Playback error, failed download                         |

## Typography

### Font Families

| Role              | Family  | Usage                                                    |
| ----------------- | ------- | -------------------------------------------------------- |
| Display / Heading | Manrope | Hero titles, section headings, album names, artist names |
| Body / Functional | Manrope | Body text, labels, buttons, navigation, track listings   |

### Type Scale

| Level   | Size | Font    | Weight | Usage                                |
| ------- | ---- | ------- | ------ | ------------------------------------ |
| Hero    | 44px | Manrope | 800    | Featured playlist title, genre hero  |
| Display | 32px | Manrope | 700    | Album titles, large section headers  |
| Title 1 | 24px | Manrope | 700    | Screen titles, artist names          |
| Title 2 | 18px | Manrope | 600    | Section headings, playlist names     |
| Title 3 | 15px | Manrope | 600    | Track titles, subsection headings    |
| Body    | 14px | Manrope | 400    | Descriptions, album notes            |
| Label   | 13px | Manrope | 500    | Field labels, button text            |
| Caption | 12px | Manrope | 400    | Track duration, release date         |
| Small   | 11px | Manrope | 500    | Badges, play counts                  |
| Micro   | 10px | Manrope | 600    | Tab labels (uppercase), micro badges |

### Font Weights

| Weight     | Value | Usage                                  |
| ---------- | ----- | -------------------------------------- |
| Regular    | 400   | Body text, descriptions                |
| Medium     | 500   | Labels, navigation, button text        |
| Semibold   | 600   | Track titles, section headings         |
| Bold       | 700   | Screen titles, album names, display    |
| Extra Bold | 800   | Hero display, featured playlist titles |

### Letter Spacing

- Hero/Display (32-44px): -1.5px (dramatic compression)
- Section headings (18-24px): -0.5px
- Uppercase tab labels: +1.5px
- Body text: 0px

### Line Height

- Hero (44px): 0.9
- Display (32px): 1.0
- Headings (18-24px): 1.2
- Body (14px): 1.5
- Captions (11-12px): 1.4

## Spacing System

### Gap Scale

| Value | Usage                                         |
| ----- | --------------------------------------------- |
| 2px   | Tight inline pairs                            |
| 4px   | Tab icon to label, play icon to track title   |
| 6px   | Artist avatar to name                         |
| 8px   | Compact track list items, mini player content |
| 12px  | Between list items, horizontal scroll cards   |
| 16px  | Card internal sections, album-to-tracks       |
| 20px  | Between cards, playlist sections              |
| 24px  | Section gaps within content                   |
| 32px  | Top-level screen section breaks               |

### Padding Scale

| Value            | Usage                                     |
| ---------------- | ----------------------------------------- |
| [0, 24]          | Screen content wrapper (horizontal only)  |
| [12, 21, 21, 21] | Tab bar section outer padding             |
| 4px              | Tab bar pill inner padding                |
| [8, 16]          | Input fields, compact buttons             |
| [10, 20]         | Standard buttons                          |
| [12, 24]         | Large buttons, CTA buttons                |
| 16px             | Compact card padding, track items         |
| 20px             | Standard card padding                     |
| 24px             | Feature card padding, album hero sections |

### Layout Pattern

- Screen width: 402px (mobile)
- Content wrapper: padding [0, 24], vertical, gap 24-32
- Status bar: 62px, standard iOS
- Tab bar pill: height 62px, cornerRadius 100, padding 4
- Tab items: fill_container, vertical, gap 4, center aligned
- Album grid: 2-column grid, 12px gap, square art with 14px radius
- Now playing bar: elevated surface, full width above tab bar

## Corner Radius

| Value | Usage                                           | Rationale                                      |
| ----- | ----------------------------------------------- | ---------------------------------------------- |
| 8px   | Small buttons, compact badges, progress bars    | Subtle sharpness for functional elements       |
| 14px  | Standard cards, track containers, album art     | Primary container radius, audio-organic        |
| 18px  | Large cards, playlist covers, featured content  | Generous radius for visual content             |
| 20px  | Hero cards, modal bottom sheets, now-playing    | Maximum standard radius                        |
| 100px | Tab bar pill, play/pause button, round controls | Full capsule shape for navigation and controls |

Design rationale: The 14-20px range creates smooth, organic contours inspired by speaker grilles and audio waveforms. Album art at 14px feels modern without losing its photographic edge, while featured content at 18-20px draws the eye with inviting softness. The range avoids clinical sharpness (finance) and excessive bubbiness (kids apps), landing in the confident, contemporary zone that music apps inhabit.

## Icons

### Icon Font

- **Family**: Lucide
- **Style**: Outline, rounded joins and caps

### Commonly Used Icons

home, search, library, music, disc-3, play, pause, skip-forward, skip-back, shuffle, repeat, volume-2, volume-x, heart, plus, download, share-2, list-music, mic-2, radio, headphones, clock, trending-up, user, bell, x, chevron-right

### Icon Sizes

| Size | Usage                                                 |
| ---- | ----------------------------------------------------- |
| 14px | Inline indicators, explicit badges                    |
| 18px | Tab bar icons, track list leading icons               |
| 20px | Card action icons, playback controls                  |
| 24px | Header actions, notification bell, prominent controls |
| 32px | Now-playing main play/pause button                    |

### Icon Color States

| State      | Color   | Usage                                         |
| ---------- | ------- | --------------------------------------------- |
| Active     | #84CC16 | Selected tab, now playing, lime highlight     |
| Visualizer | #22D3EE | Audio visualizer, secondary highlights (cyan) |
| Default    | #A3A3A3 | Inactive tabs, secondary actions              |
| Muted      | #3B3B3B | Disabled states, placeholder icons            |
| On Accent  | #0A0A0A | Icons on lime-colored backgrounds (black)     |
