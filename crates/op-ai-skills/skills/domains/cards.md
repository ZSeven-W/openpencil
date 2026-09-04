---
name: cards
description: Portrait card-board contract — root margin ownership, one item template, ornament discipline (XHS 小红书 / knowledge cards)
phase: [generation]
trigger:
  keywords: [card, 卡片, 知识卡, 封面, 小红书, cover]
priority: 26
budget: 800
category: domain
---

CARDS CONTRACT

Holds for every standalone portrait card board — XHS 小红书 3:4 (1080×1440), the 1:1 square card (1080×1080), knowledge cards 知识卡片, card covers 封面. A card is ONE fixed board: it is neither a scroll page nor a deck slide. Content that does not fit is cut, or moved to the next card in the series — never shrunk to fit, never clipped.

## Hard rules

1. `MARGIN OWNERSHIP: the card root itself carries horizontal padding ≥48px (1080-wide) — never delegate page margins to sections; sections stay padding-free horizontally.`

2. `ITEM TEMPLATE: define ONE item structure, then copy it N times — same nesting, same ornament, same name pattern; only the copy differs. Five items must not have five structures.`

3. `ORNAMENT DISCIPLINE: pick one numbering/ornament treatment and repeat it verbatim on every item.`

4. `VERTICAL RHYTHM: content fills the card's full height — top margin ≥64px, footer lands inside the bottom margin, trailing void ≤15%; grow type scale or section gaps rather than leaving the lower half empty.`

5. `TEXT-ONLY IMAGE GATE: when the user supplies text but does not explicitly request a photo, image, illustration, texture, or other raster artwork, create NO image node, image slot, imageSearchQuery, imagePrompt, stock-search background, or generated-image background. The fixed board itself is the card — use typography, colour fields, vector paths, iconFont, rules, and repeated shapes for visual energy instead of placing a white card on an unrelated photo.`
