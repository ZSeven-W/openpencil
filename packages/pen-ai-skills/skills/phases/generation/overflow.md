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
