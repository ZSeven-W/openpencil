---
name: elements
description: N-tool element family reference — when and how to call add_card_row_v0 / add_metric_row_v0 / add_nav_chip_row_v0 / add_bottom_nav_v0 / add_activity_ring_v0 instead of hand-building via batch_design
phase: [generation]
trigger:
  flags: [hasMcpTools]
priority: 14
budget: 1500
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

1. Row of items with title + subtitle + optional icon → `add_card_row_v0`
2. Row of items with small label + big numeric value → `add_metric_row_v0`
3. Row of filter chips / category tabs (label + optional icon, active state) → `add_nav_chip_row_v0`
4. Bottom tab bar (inline flow, 3-5 nav items) → `add_bottom_nav_v0`
5. Apple-style progress ring with centered text → `add_activity_ring_v0`
6. None match → fall through to `batch_design`

## When to use vs batch_design

PREFER an element tool when the spec says any of:

- "horizontal scrolling cards", "swipeable row", "chip row", "pills"
- "metric tiles", "KPI cards", "dashboard stats", "Steps / Kcal / Sleep" row
- "category filter chips", "quick-access shortcuts"
- "bottom nav", "tab bar", "tabbar", "底部导航"
- "activity ring", "progress ring", "circular progress", "Apple health ring"

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
- Roles are set (`card` / `metric-tile` / `nav-chip` / `nav-chip-active` / `bottom-tab-bar` / `nav-item` / `nav-item-active` / `activity-ring`)

## Failure mode

If the tool throws, **do NOT retry with the same arguments** — the tool has already verified the failure is real (pre-check rejected the parent_id, or post-check detected a silent DSL no-op). Re-throwing from your side wastes tokens. Inspect the error message and either:

- Fix `parent_id` (ensure the referenced node exists and has no `"` or `\` in its id)
- Switch to `batch_design` with the structure taught in `overflow.md` / `layout.md`
