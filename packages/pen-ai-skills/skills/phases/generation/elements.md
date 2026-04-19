---
name: elements
description: N-tool element family (12 tools) reference — rows (card/metric/nav_chip/stat_grid), containers (bottom_nav/top_nav_bar/section_header/icon_button/activity_ring), atoms (divider/badge/avatar). Each tool replaces a documented batch_design failure mode
phase: [generation]
trigger:
  flags: [hasMcpTools]
priority: 14
budget: 1800
category: base
---

<!--
  IMPORTANT: This skill is gated by the `hasMcpTools` flag. It only
  auto-loads into the generation-phase prompt when the caller declares
  the AI has live access to MCP element tools (external clients:
  Claude Code / Codex / Gemini CLI / Cursor). The embedded orchestrator
  in apps/web emits single-shot JSON and cannot call MCP tools — this
  skill would be 1500 tokens of dead weight there, so it stays excluded.

  External MCP clients still retrieve the content explicitly via
  get_design_prompt(section='elements'), which bypasses resolveSkills'
  trigger filter (uses getSkillByName for direct lookup).

  To opt-in auto-loading from a new caller: pass `{ flags: { hasMcpTools: true } }`
  to resolveSkills('generation', prompt, opts).
-->

ELEMENT TOOLS (schema-constrained alternatives to batch_design):

These narrow MCP tools emit well-known structures that batch_design frequently gets wrong on non-Claude models (overflow, wrong role, anti-pattern layout). Each is shape-locked — you pick the tool by matching intent, then supply only content. Visual styling (color, font) stays orthogonal: override via a follow-up batch_design U-op if needed.

## Decision tree (pick first match)

Rows (horizontal, in-card or scrolling):

1. Row of items with title + subtitle + optional icon → `add_card_row_v0` (scroll)
2. Row of items with small label + big numeric value → `add_metric_row_v0` (scroll)
3. Row of filter chips / category tabs (label + optional icon, active state) → `add_nav_chip_row_v0` (scroll)
4. Non-scrolling 2-5 stats inline (auto-share width) → `add_stat_grid_v0`

Containers and single elements:

5. Section header (big title + optional "See all" action) → `add_section_header_v0`
6. Bottom tab bar (inline flow, 3-5 nav items) → `add_bottom_nav_v0`
7. Mobile top bar (leading icon + centered title + trailing icon) → `add_top_nav_bar_v0`
8. Icon-only button (44×44, hit-target safe) → `add_icon_button_v0`
9. Apple-style progress ring with centered text → `add_activity_ring_v0`

Atoms (1-2 node building blocks):

10. Hairline separator between list rows / sections → `add_divider_v0`
11. Short inline pill / tag / "NEW" / "BETA" / count badge → `add_badge_v0`
12. Circular avatar (with optional initial / empty for later image fill) → `add_avatar_v0`

Text + button primitives:

13. Padding-based button with text (optional leading icon) → `add_text_button_v0`
14. Heading with enforced fontSize/lineHeight per level + AUTO CJK script detection (SC/JP/KR) → `add_heading_v0`
15. Body text (Inter everywhere — CJK gets lineHeight 1.6 + letterSpacing 0, Latin 1.5) → `add_body_text_v0`

Composition:

16. Icon + text inline pair (menu items, breadcrumbs, status indicators) → `add_icon_label_v0`
17. iOS/Material list row (leading icon + title/subtitle stack + trailing icon) → `add_list_row_v0`

18. None match → fall through to `batch_design`

**Disambiguation**: if you need a ROW of 3 metrics that should NOT scroll (e.g. a stats strip inside a card), use `add_stat_grid_v0`, NOT `add_metric_row_v0`. The grid uses `fill_container` per cell so it never overflows; the metric row uses fixed-px cells + scroll wrapper.

## When to use vs batch_design

PREFER an element tool when the spec says any of:

- "horizontal scrolling cards", "swipeable row", "chip row", "pills" → `add_card_row_v0`
- "metric tiles", "KPI cards", "dashboard stats" (SCROLLING row) → `add_metric_row_v0`
- "stats row", "3 metrics side by side", "summary bar" (NON-scrolling grid) → `add_stat_grid_v0`
- "category filter chips", "quick-access shortcuts" → `add_nav_chip_row_v0`
- "section title with See all / View more" → `add_section_header_v0`
- "bottom nav", "tab bar", "tabbar", "底部导航" → `add_bottom_nav_v0`
- "top bar", "app bar", "header with back button", "页面标题栏" → `add_top_nav_bar_v0`
- "icon-only button", "close button", "menu button" (toolbar-style) → `add_icon_button_v0`
- "activity ring", "progress ring", "circular progress", "Apple health ring" → `add_activity_ring_v0`
- "hairline divider", "separator", "row divider", "section separator" → `add_divider_v0`
- "badge", "pill", "tag", "NEW label", "count bubble" (≤16 Latin / ≤8 CJK chars) → `add_badge_v0`
- "avatar", "profile picture", "user circle", "initial bubble" → `add_avatar_v0`
- "primary button", "secondary button", "CTA", "submit button" (short label) → `add_text_button_v0`
- "hero headline", "section title", "card title" / 特定字号标题 → `add_heading_v0`
- "body paragraph", "description text", "intro copy" (包含 CJK 时尤其推荐) → `add_body_text_v0`
- "icon with label", "menu item (inline)", "breadcrumb segment", "status indicator text" → `add_icon_label_v0`
- "settings row", "list item", "iOS list cell", "table row with chevron" → `add_list_row_v0`

STILL use batch_design when:

- The row's items are structurally heterogeneous (can't be uniformly described by a single items[] shape)
- You need to build a larger composite (e.g. a whole section containing a scroll row + other content — build the section via batch_design, then insert the row via element tool with parent_id)
- Post-hoc styling: once the element tool has laid the structure, use batch_design U-ops to apply fills, typography, or theme variables

## Minimal usage

```
add_card_row_v0({
  items: [
    { title: "Hiit",     subtitle: "30 min", icon: "flame" },
    { title: "Strength", subtitle: "45 min", icon: "dumbbell" },
    { title: "Yoga",     subtitle: "25 min", icon: "leaf" },
  ],
})

add_metric_row_v0({
  items: [
    { label: "Steps",  value: "8,432",  icon: "activity" },
    { label: "Kcal",   value: "512",    icon: "flame" },
    { label: "Sleep",  value: "7h 24m", icon: "moon" },
  ],
})

add_nav_chip_row_v0({
  items: [
    { label: "All",     active: true },           // label-only chips OK
    { label: "Videos",  icon: "video" },
    { label: "Photos",  icon: "image" },
  ],
})

add_bottom_nav_v0({
  items: [
    { title: "Home",    icon: "home",    active: true },
    { title: "Search",  icon: "search" },
    { title: "Profile", icon: "user" },
  ],
})

add_activity_ring_v0({
  center_text: "8,432",
  size: 80,
  thickness: 8,
})

add_stat_grid_v0({
  items: [
    { value: "8,432", label: "Steps",  icon: "activity" },
    { value: "512",   label: "Kcal",   icon: "flame" },
    { value: "7h",    label: "Sleep",  icon: "moon" },
  ],
})

add_section_header_v0({
  title: "Recent Workouts",
  action: { label: "See all", icon: "arrow-right" },
})

add_top_nav_bar_v0({
  title: "Settings",
  leading_icon: "chevron-left",
  trailing_icon: "more-vertical",
})

add_icon_button_v0({
  icon: "search",
})

add_divider_v0({})                           // horizontal hairline (h=1 fill_container)
add_divider_v0({ orientation: "vertical" })  // vertical hairline

add_badge_v0({ label: "NEW" })

add_avatar_v0({ initial: "JD", size: 56 })   // with initial
add_avatar_v0({ size: 40 })                  // empty circle (fill via batch_design image later)

add_text_button_v0({ label: "Get Started" })
add_text_button_v0({ label: "Add item", leading_icon: "plus" })

add_heading_v0({ content: "Welcome back" })                   // defaults to h2 (24/600/1.2)
add_heading_v0({ content: "Hero Headline", level: "display" }) // 48/700/1.0/-0.5

add_body_text_v0({ content: "Lorem ipsum dolor sit amet…" })   // Inter + 1.5
add_body_text_v0({ content: "你好世界，这是一段中文正文。" })  // Inter + 1.6 + letterSpacing 0
add_body_text_v0({ content: "こんにちは、これは本文です。" })  // Inter + 1.6 + letterSpacing 0
add_body_text_v0({ content: "안녕하세요, 이것은 본문입니다." }) // Inter + 1.6 + letterSpacing 0
// body ALWAYS Inter per text-rules.md. Only HEADINGS dispatch to
// Noto Sans SC/JP/KR (see add_heading_v0). Inter uses system CJK fallback.

add_icon_label_v0({ icon: "info", label: "Learn more" })

add_list_row_v0({
  title: "Notifications",
  subtitle: "Push, email, and in-app",
  leading_icon: "bell",
  trailing_icon: "chevron-right",
})
```

## Composition pattern

For a dashboard that needs a metric row inside a page:

1. Build the page structure via `batch_design` (root frame + section container) — note the section's id
2. Call `add_metric_row_v0({ parent_id: "<section-id>", items: [...] })` to insert the row under that section
3. Optional: a second `batch_design` U-op to style (fill, theme variables)

## Invariants you don't need to think about

The tool guarantees — you cannot break them from the input side:

- Wrapper structure (`scroll-row-wrapper` + `scroll-row` + fixed-width children) for row tools — overflow-safe
- `bottom-tab-bar` is inline (no empty spacer sibling needed, do NOT add one)
- Activity ring is frame+cornerRadius=size/2+stroke+centered text — NEVER emit ellipse+sibling text for rings
- Every emitted node has a unique id (you can reference it later)
- Roles are set (`card` / `metric-tile` / `nav-chip` / `nav-chip-active` / `bottom-tab-bar` / `nav-item` / `nav-item-active` / `activity-ring` / `stat-grid` / `stat-cell` / `section-header` / `section-header-title` / `section-header-action` / `top-nav-bar` / `nav-spacer` / `icon-button` / `divider` / `badge` / `avatar` / `button` / `heading` / `body` / `label` / `icon-label` / `list-row` / `list-row-text`)

## Failure mode

If the tool throws, **do NOT retry with the same arguments** — the tool has already verified the failure is real (pre-check rejected the parent_id, or post-check detected a silent DSL no-op). Re-throwing from your side wastes tokens. Inspect the error message and either:

- Fix `parent_id` (ensure the referenced node exists and has no `"` or `\` in its id)
- Switch to `batch_design` with the structure taught in `overflow.md` / `layout.md`
