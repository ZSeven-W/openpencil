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

When the spec explicitly requests horizontal scrolling/swiping cards, filter chips or metric tiles, use ONE of the two paths below. A plain metadata/difficulty badge row is not a scroll request: preserve every label and its font size, pack whole Hug badges into additional horizontal lines, and let the section height grow. Never make the last badge Fill or clip its text to conceal overflow.

**Preferred (MCP tool path)** — if you have MCP tools (external client: Claude Code / Codex / Cursor), call the tool matching what's in the row; all three produce the overflow-safe wrapper+clipContent+fit_content structure and cannot be malformed by schema:

- **`add_card_row_v0`** — items with `title` + optional `subtitle`/`icon` (workout/feature/content cards). Default 140x160.
- **`add_metric_row_v0`** — items with `label` + `value` + optional `icon` (dashboard stats). Default 120x100, value 28/700.
- **`add_nav_chip_row_v0`** — items with `label` + optional `icon`/`active` (plain-text filter tags OK). Default 72xfit_content.

Add per-tile fills/colors afterward with a separate `batch_design` U-op (these tools are style-guide orthogonal, ship colorless on purpose).

**Fallback (no MCP tools — embedded AI / JSON-only output)** — see the `horizontal-scroll-fallback` knowledge skill for the exact wrapper+row+card JSON structure, the fixed-vs-fit_content width rule per card type, and the anti-patterns to avoid. Do NOT freehand 5+ cards inside a bare horizontal frame — they overflow the phone width.
