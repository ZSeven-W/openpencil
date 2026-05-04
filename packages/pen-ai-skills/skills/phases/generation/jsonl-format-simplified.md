---
name: jsonl-format-simplified
description: Simplified nested JSON format for basic tier models
phase: [generation]
trigger:
  flags: [isBasicTier]
priority: 0
budget: 1700
category: base
---

CRITICAL: This skill describes the JSONL fallback format. If a separate `OUTPUT FORMAT — EMIT AS TOOL CALL(S)` block (teaching `<op_tool>` tags) appears later in the system prompt, FOLLOW THAT block — emit `<op_tool>` tags whose payloads are element-tool calls or `batch_design`. Use the JSONL form below ONLY when no `<op_tool>` instruction is present. Either way, do NOT mix prose with the structured output, do not use `[TOOL_CALL]` or `{tool => ...}` legacy syntax, and do not output JSON outside its block.

Generate a UI section as a nested JSON tree. Output a ```json block with a single root object containing nested "children" arrays.

TYPES:
frame (width,height,layout,gap,padding,justifyContent,alignItems,cornerRadius,fill,children), rectangle (width,height,cornerRadius,fill), text (content,fontFamily,fontSize,fontWeight,fill,width,textAlign), icon_font (iconFontName,width,height,fill)
SHARED: id, type, name

RULES:

- Root: type="frame", width="fill_container", height="fit_content", layout="vertical".
- Children go in "children" arrays. No x/y on layout children.
- width/height: number | "fill_container" | "fit_content".
- fill: [{"type":"solid","color":"#hex" | "$color-*"}].
- Text: never set height. Use width="fill_container" for wrapping text.
- Icons: use icon_font with iconFontName (lucide names: search, bell, user, heart, star, plus, x, check, chevron-right, settings). Sizes: 16/20/24px.
- Buttons: frame with padding=[12,24] containing a text child.
- No emoji characters. No markdown. No explanation. No tool calls.

DESIGN SYSTEM TOKENS — prefer refs over literals; the renderer resolves them against the user's seeded palette (or a default light palette when un-seeded), so refs are SAFE even on a fresh document.

- COLORS: `$color-{bg-deep|surface|surface-2|surface-3|border|border-strong|text-primary|text-body|text-muted|text-subtle|accent|destructive|success|scrim|info-bg|info-text|success-bg|success-text|warning-bg|warning-text|danger-bg|danger-text|chart-1..6}`. Light defaults: bg-deep `#F8FAFC`, surface `#FFFFFF`, text-primary `#0F172A`, text-body `#334155`, text-muted `#64748B`, accent `#2563EB`, border `#E2E8F0`.
- TYPOGRAPHY: `$type-{display|h1|h2|h3|body|caption}-{size|weight|line-height}`. Defaults: display 64/700/1.0, h1 24/600/1.2, h2 20/600/1.25, h3 16/600/1.3, body 14/400/1.5, caption 12/400/1.4.
- SPACING / RADIUS: `$spacing-{1|2|3|4|5}` = 4/8/12/16/24 px. `$radius-{sm|md|lg}` = 4/8/12 px.

USE refs for standard semantic colors and typography sizes/weights/line-heights that match the scale above. KEEP literal hex / numbers for brand-specific colors not in the palette, off-scale pixel values, and "white text on accent" (`#FFFFFF`).

EXAMPLE:

```json
{
  "id": "root",
  "type": "frame",
  "name": "Hero",
  "width": "fill_container",
  "height": "fit_content",
  "layout": "vertical",
  "gap": 24,
  "padding": [48, 24],
  "fill": [{ "type": "solid", "color": "$color-bg-deep" }],
  "children": [
    {
      "id": "title",
      "type": "text",
      "name": "Headline",
      "content": "Learn Smarter",
      "fontSize": 48,
      "fontWeight": 700,
      "fontFamily": "Space Grotesk",
      "fill": [{ "type": "solid", "color": "$color-text-primary" }]
    },
    {
      "id": "desc",
      "type": "text",
      "name": "Description",
      "content": "AI-powered learning",
      "fontSize": "$type-body-size",
      "width": "fill_container",
      "fill": [{ "type": "solid", "color": "$color-text-muted" }]
    },
    {
      "id": "cta",
      "type": "frame",
      "name": "CTA",
      "padding": [14, 28],
      "cornerRadius": 10,
      "justifyContent": "center",
      "fill": [{ "type": "solid", "color": "$color-accent" }],
      "children": [
        {
          "id": "cta-text",
          "type": "text",
          "content": "Get Started",
          "fontSize": 16,
          "fontWeight": 600,
          "fill": [{ "type": "solid", "color": "#FFFFFF" }]
        }
      ]
    }
  ]
}
```
