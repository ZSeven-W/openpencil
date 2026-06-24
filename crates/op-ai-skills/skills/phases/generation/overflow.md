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
- **`add_nav_chip_row_v0`** — items with `label` + optional `icon` + optional `active` flag. Label-only chips supported (plain-text filter tags like "All / Videos / Photos"). Default chip size 72×fit_content.

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

Every **content / product / workout card** in the row MUST:

- Have a FIXED numeric `width` (typically 120-160 for mobile, 200-260 for desktop). Never `fill_container`, never `fit_content` - fixed pixels.
- Share identical width with its siblings for visual rhythm.

**EXCEPTION — nav chips / category chips / filter tags** (icon + short label like "All" / "Pizza" / "Videos"): use `width="fit_content"`, NEVER a fixed 120-160. That fixed width is content-card sizing; a 6-chip category row at 132px each becomes ~800px and scrolls off-screen for what should comfortably fit on one screen. With `fit_content`, a handful of short chips sit on one row (no scroll), and only a genuinely long list scrolls. Keep the same clipContent wrapper + fit_content row — just let each chip hug its content (icon + label + small horizontal padding).

**COUNT CAP for a no-scroll chip row (mobile 375px):** even at `fit_content`, only ~4-5 icon+label chips fit one phone width. For primary mobile category navigation, prefer the top 4 fully visible chips or wrap/grid them — do NOT show a half-clipped fifth chip as decoration. If the design is meant to fit on screen WITHOUT horizontal scrolling, emit only the chips that fit — do NOT pack 6+ chips into the row, the extras render off the right edge of the device. If you genuinely need all categories, you MUST place the row inside the `clipContent` wrapper above so the overflow clips at the screen edge (scroll row) instead of spilling past the phone frame. A bare horizontal frame with 6+ chips and no `clipContent` ancestor is the #1 mobile overflow bug — never emit it.

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
- Using `width="fit_content"` on **content/product cards** - text-driven widths are unpredictable and break rhythm. (Nav / category chips are the EXCEPTION above — those SHOULD use fit_content so a short row fits one screen.)
- Skipping the `clipContent=true` wrapper and relying on Skia to clip (it doesn't — only `clipContent:true` enables clipping).
