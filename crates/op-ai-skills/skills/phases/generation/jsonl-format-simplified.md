---
name: jsonl-format-simplified
description: Simplified flat _parent JSONL format for basic tier models
phase: [generation]
trigger:
  flags: [isBasicTier]
priority: 0
budget: 1700
category: base
---

CRITICAL — OUTPUT-MODE PRIORITY: If a separate `OUTPUT FORMAT — EMIT AS TOOL CALL(S)` block (teaching `<op_tool>` tags) appears later in the system prompt, FOLLOW THAT block — emit `<op_tool>` tags whose payloads are element-tool calls or `batch_design`. Use the JSONL form (the EXAMPLE block at the bottom of this skill) ONLY when no `<op_tool>` instruction is present. Either way, never use `[TOOL_CALL]` or `{tool => ...}` legacy syntax, and never mix prose with the structured output.

The TYPES / RULES / DESIGN SYSTEM TOKENS sections below describe the underlying PenNode schema and apply to EITHER mode — they tell you the shape of node arguments inside an `<op_tool>` call, and the shape of nodes in raw JSONL.

TYPES:
frame (width,height,layout,gap,padding,justifyContent,alignItems,cornerRadius,fill), rectangle (width,height,cornerRadius,fill), text (content,fontFamily,fontSize,fontWeight,fill,width,textAlign), icon_font (iconFontName,width,height,fill)
SHARED: id, type, name, _parent

RULES:

- Root: type="frame", width="fill_container", height="fit_content", layout="vertical", _parent=null.
- Every node carries "_parent" — null for the root, else its parent's id. No x/y on layout children.
- width/height: number | "fill_container" | "fit_content".
- fill: [{"type":"solid","color":"#hex" | "$color-*"}].
- Text: never set height. Use width="fill_container" for wrapping text.
- Icons: use icon_font with iconFontName (lucide names: search, bell, user, heart, star, plus, x, check, chevron-right, settings). Sizes: 16/20/24px.
- Buttons: frame with padding=[12,24] containing a text child.
- No emoji characters. No markdown. No explanation outside the structured output.

DESIGN SYSTEM TOKENS — prefer refs over literals; the renderer resolves them against the user's seeded palette (or a default light palette when un-seeded), so refs are SAFE even on a fresh document.

- COLORS: `$color-{bg-deep|surface|surface-2|surface-3|border|border-strong|text-primary|text-body|text-muted|text-subtle|accent|destructive|success|scrim|info-bg|info-text|success-bg|success-text|warning-bg|warning-text|danger-bg|danger-text|chart-1..6}`. Light defaults: bg-deep `#F8FAFC`, surface `#FFFFFF`, text-primary `#0F172A`, text-body `#334155`, text-muted `#64748B`, accent `#2563EB`, border `#E2E8F0`.
- TYPOGRAPHY: `$type-{display|h1|h2|h3|body|caption}-{size|weight|line-height}`. Defaults: display 64/700/1.0, h1 24/600/1.2, h2 20/600/1.25, h3 16/600/1.3, body 14/400/1.5, caption 12/400/1.4.
- SPACING / RADIUS: `$spacing-{1|2|3|4|5}` = 4/8/12/16/24 px. `$radius-{sm|md|lg}` = 4/8/12 px.

USE refs for standard semantic colors and typography sizes/weights/line-heights that match the scale above. KEEP literal hex / numbers for brand-specific colors not in the palette, off-scale pixel values, and "white text on accent" (`#FFFFFF`).

— JSONL FALLBACK MODE — the section below applies ONLY when there is no `<op_tool>` instruction earlier in the prompt; if `<op_tool>` mode is in effect, ignore the EXAMPLE below and emit `<op_tool>` tags instead.

Output one JSON object per line (NO enclosing [ ] array, NO "children" field). Each line carries "_parent" — null for the root, else its parent's id (which appears on an earlier line). Express the WHOLE tree via _parent; a flat list of siblings with no _parent links renders BROKEN (collapses into a vertical stack).

EXAMPLE:

```json
{"_parent":null,"id":"root","type":"frame","name":"Hero","width":"fill_container","height":"fit_content","layout":"vertical","gap":24,"padding":[48,24],"fill":[{"type":"solid","color":"$color-bg-deep"}]}
{"_parent":"root","id":"title","type":"text","name":"Headline","content":"Learn Smarter","fontSize":48,"fontWeight":700,"fontFamily":"Space Grotesk","fill":[{"type":"solid","color":"$color-text-primary"}]}
{"_parent":"root","id":"desc","type":"text","name":"Description","content":"AI-powered learning","width":"fill_container","fill":[{"type":"solid","color":"$color-text-muted"}]}
{"_parent":"root","id":"cta","type":"frame","name":"CTA","padding":[14,28],"cornerRadius":10,"justifyContent":"center","fill":[{"type":"solid","color":"$color-accent"}]}
{"_parent":"cta","id":"cta-text","type":"text","content":"Get Started","fontSize":16,"fontWeight":600,"fill":[{"type":"solid","color":"#FFFFFF"}]}
```
