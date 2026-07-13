---
name: design-system
description: Design-system composition depth — token roles, screen layout contracts, button hierarchy, spacing, composition recipes
phase: [generation]
trigger:
  keywords: [design system, component library, ui kit, tokens, composition, 设计系统, 组件库, 设计规范]
priority: 30
budget: 1600
category: knowledge
---

DESIGN-SYSTEM COMPOSITION

Compose screens from a consistent token + spacing + hierarchy system. Product/density/action laws and the 8px scale ALREADY apply — do not restate them. This adds the composition DEPTH. Apply silently through node structure.

TOKEN ROLES (use $color-* refs when the document carries the semantic palette; else hex):

- Backgrounds: $color-bg-deep = page background (the outermost canvas, slightly tinted, never pure white); $color-surface = cards / modals / panels that sit ON the page; $color-surface-2 = chips, inputs, hover; $color-surface-3 = pressed / nested. A surface always sits one step lighter/darker than its parent — never stack two same-fill surfaces.
- Text ladder (4 steps, pick by importance): $color-text-primary headings & values → $color-text-body paragraphs & nav labels → $color-text-muted secondary / timestamps / placeholders → $color-text-subtle disabled. Never use primary for everything; the ladder IS the hierarchy.
- $color-border for dividers / input strokes / card edges; $color-accent for primary action, active state, focus. Semantic ($color-success / $color-destructive) carry meaning across themes — never swap them for accent.
- Structural wrappers (page bg, section, header) stay on $color-bg-deep / transparent. Only a real card/panel/input gets $color-surface. Boxing every group in a surface reads generic.

SCREEN LAYOUT CONTRACTS (one frame tree, fill_container threads through):

- Sidebar + content: definite-height root layout=horizontal; sidebar fixed 240-280 height=fill_container; main width=fill_container height=fill_container layout=vertical gap=24 padding=32. Sections inside main remain fit_content.
- Header + content: root layout=vertical; header height 56-64 padding=[0,24] layout=horizontal justifyContent=space_between alignItems=center, bottom border only; ordinary content width=fill_container height=fit_content layout=vertical gap=24 padding=32.
- Two-column (2/3 + 1/3): inner layout=horizontal gap=24; main width=fill_container; side fixed 320-360. Main carries the dominant region, side carries supporting context.
- Card grid: layout=horizontal gap=16-24, each card width=fill_container so columns share width evenly (wrap to vertical stacks on narrow widths).

COMPOSITION RECIPES:

- Page header: layout=horizontal justifyContent=space_between alignItems=center — breadcrumbs / title on the left, action buttons on the right (gap=12).
- Form layout: vertical gap=16; pair short related fields (First/Last name) in one horizontal row of two fill_container inputs, then full-width fields (Email, Message) stacked below. Submit actions right-aligned at the bottom.
- Metric card: vertical, padding=[24,24], gap=4 — small $color-text-muted label (14) above a large bold $color-text-primary value (28-36). No icon-padding-stuffing; the number is the focal point.
- Tables follow the strict Table > Row > Cell(frame) > content hierarchy and column-width guidance in the dashboard domain — do not re-derive it.

BUTTON HIERARCHY (one primary action per section; reduce the rest):

- Priority ladder: 1 Primary (Save/Submit/Create, accent fill) → 2 Secondary (alt action) → 3 Outline (Cancel/Back) → 4 Ghost (inline/nav) → 5 Destructive (Delete, $color-destructive). Never give two buttons equal weight in one group.
- Action alignment: cards / modals / forms right-align actions (justifyContent=end). Destructive + Cancel pairing = Cancel on the left, Destructive on the right.

SPACING BY CONTEXT (pick from the scale, never arbitrary):

- Screen sections gap 24-32; card grid gap 16-24; form fields gap 16; button groups gap 12.
- Padding: inside cards 24; inside buttons [10,16]; inside inputs [8,16]; page content area 32; sidebar items [10,16].

DENSITY: one idea per card; 4-7 columns per table; leave breathing room over packing equal-weight blocks. Commit to one density mode per screen.
