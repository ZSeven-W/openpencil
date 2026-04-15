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

## HORIZONTAL SCROLL ROWS (cards / chips / categories)

When the spec says "horizontal scrolling cards", "swipeable row", "chip row", or similar, generate EXACTLY this structure — do NOT just emit 6 cards inside a horizontal layout, the children will spill outside the page frame.

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
