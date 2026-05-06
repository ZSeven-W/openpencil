---
name: overflow
description: Overflow prevention rules for text and child sizing
phase: [generation]
trigger: null
priority: 16
budget: 500
category: base
---

OVERFLOW PREVENTION（关键）：

- vertical layout 中的 text：width="fill_container" + textGrowth="fixed-width"。horizontal 中：width="fit_content"。
- 绝不要在 layout frames 内的 text 上设置 fixed pixel width（例如 195px card 中 width:378 - 会 overflow！）。
- Fixed-width children 必须 <= parent content area（parent width - padding）。
- Badges：只使用短 labels（CJK <=8 chars / Latin <=16 chars）。

## HORIZONTAL SCROLL ROWS (cards / chips / categories)

当 spec 提到 "horizontal scrolling cards"、"swipeable row"、"chip row" 或类似内容时，必须生成下面这个精确结构 — 不要只在 horizontal layout 中输出 6 张 cards，否则 children 会溢出 page frame。

结构：

- 一个 wrapper frame，设置 `width="fill_container"`、`height="fit_content"`、`layout="vertical"`、`clipContent=true`。
- 内部是一个 row frame，设置 `width="fit_content"`、`height="fit_content"`、`layout="horizontal"`、`gap=12`、`padding=[0,20]`。
- row frame 持有真正的 cards。

row 中的每个 card 必须：

- 拥有固定 numeric `width`（mobile 通常 120-160，desktop 通常 200-260）。不要使用 `fill_container`，不要使用 `fit_content` - 使用 fixed pixels。
- 与 siblings 共享相同 width，以形成视觉节奏。

示例 - 375px-wide mobile page 中的 6 张 workout cards：

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

Anti-patterns（不要输出以下任何一种）：

- 将 5+ cards 直接放进 `layout="horizontal"` 的 page-root frame（它们会溢出 phone width）。
- 在 horizontal row 的 cards 上使用 `fill_container`（它们会被压缩到不可见）。
- 在 cards 上使用 `width="fit_content"` - text-driven widths 不可预测，会破坏节奏。
- 跳过 `clipContent=true` wrapper 并依赖 Skia 裁剪（它不会 — 只有 `clipContent:true` 会启用 clipping）。
