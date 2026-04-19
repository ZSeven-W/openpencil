---
name: overflow
description: Overflow prevention rules for text and child sizing
phase: [generation]
trigger: null
priority: 16
budget: 500
category: base
---

OVERFLOW PREVENTION (CRITICAL):

- Text in vertical layout: width="fill_container" + textGrowth="fixed-width". In horizontal: width="fit_content".
- NEVER set fixed pixel width on text inside layout frames (e.g. width:378 in 195px card - overflows!).
- Fixed-width children must be <= parent content area (parent width - padding).
- Badges: short labels only (CJK <=8 chars / Latin <=16 chars).

## HORIZONTAL SCROLL ROWS (cards / chips / categories / metric tiles)

When the spec says "horizontal scrolling cards", "swipeable row", "chip row", "metric tiles", or similar, use ONE of the two paths below.

### Preferred (MCP tool path): pick one of 3 narrow row tools

If you have access to MCP tools (external client: Claude Code / Codex / Gemini CLI / Cursor), call the tool matching what's in the row — all three produce the overflow-safe wrapper+clipContent+fit_content structure and cannot be made incorrectly by schema.

- **`add_card_row_v0`** — items with `title` + optional `subtitle` + optional `icon` (workout cards, feature tiles, content cards). Default card size 140×160.
- **`add_metric_row_v0`** — items with `label` + `value` + optional `icon` (dashboard stats: Steps/Kcal/Sleep/Revenue). Default tile size 120×100, value rendered 28/700.
- **`add_nav_chip_row_v0`** — items with `label` + `icon` + optional `active` flag (filter chips, category tabs). Default chip size 72×fit_content.

Example:

```
add_metric_row_v0({
  items: [
    { label: "Steps",  value: "8,432",  icon: "activity" },
    { label: "Kcal",   value: "512",    icon: "flame" },
    { label: "Sleep",  value: "7h 24m", icon: "moon" },
  ],
})
```

Add per-tile fills / colors afterwards with a separate `batch_design` U-op (these tools are style-guide orthogonal and ship colorless on purpose).

### Fallback (hand-built JSON path)

ONLY when the MCP tool is unavailable (embedded AI flow / JSON-only output), generate EXACTLY this structure — do NOT just emit 6 cards inside a horizontal layout, the children will spill outside the page frame.

Structure:

- A wrapper frame with `width="fill_container"`, `height="fit_content"`, `layout="vertical"`, `clipContent=true`.
- Inside it, a row frame with `width="fit_content"`, `height="fit_content"`, `layout="horizontal"`, `gap=12`, `padding=[0,20]`.
- The row frame holds the actual cards.

Every card in the row MUST:

- Have a FIXED numeric `width` (typically 120-160 for mobile, 200-260 for desktop). Never `fill_container`, never `fit_content` - fixed pixels.
- Share identical width with its siblings for visual rhythm.

Example - 6 workout cards inside a 375px-wide mobile page:

```json
{
  "id": "cards-scroll",
  "type": "frame",
  "name": "Workouts Scroll",
  "width": "fill_container",
  "height": "fit_content",
  "layout": "vertical",
  "clipContent": true,
  "children": [
    {
      "id": "cards-row",
      "type": "frame",
      "name": "Workouts Row",
      "width": "fit_content",
      "height": "fit_content",
      "layout": "horizontal",
      "gap": 12,
      "padding": [0, 20],
      "children": [
        {
          "id": "card-hiit",
          "type": "frame",
          "width": 140,
          "height": 160,
          "cornerRadius": 20,
          "layout": "vertical",
          "gap": 8,
          "padding": 16,
          "fill": [{ "type": "solid", "color": "#1a1a1a" }],
          "children": []
        },
        {
          "id": "card-strength",
          "type": "frame",
          "width": 140,
          "height": 160,
          "cornerRadius": 20,
          "layout": "vertical",
          "gap": 8,
          "padding": 16,
          "fill": [{ "type": "solid", "color": "#1a1a1a" }],
          "children": []
        }
      ]
    }
  ]
}
```

Anti-patterns (do NOT emit any of these):

- Putting 5+ cards directly inside a `layout="horizontal"` page-root frame (they overflow the phone width).
- Using `fill_container` on cards in a horizontal row (they squish down to invisibility).
- Using `width="fit_content"` on cards - text-driven widths are unpredictable and break rhythm.
- Skipping the `clipContent=true` wrapper and relying on Skia to clip (it doesn't — only `clipContent:true` enables clipping).
