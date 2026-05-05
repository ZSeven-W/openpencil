---
name: jsonl-format
description: Sub-agent flat JSONL output format with node types and rules
phase: [generation]
trigger: null
priority: 0
budget: 1700
category: base
---

CRITICAL — OUTPUT-MODE PRIORITY: If a separate `OUTPUT FORMAT — EMIT AS TOOL CALL(S)` block (teaching `<op_tool>` tags) appears later in the system prompt, FOLLOW THAT block — emit `<op_tool>` tags whose payloads are element-tool calls or `batch_design`. Use the JSONL form (the FORMAT + ```json example sections at the bottom of this skill) ONLY when no `<op_tool>`instruction is present. Either way, never use`[TOOL_CALL]`or`{tool => ...}` legacy syntax, and never mix prose with the structured output.

The TYPES / RULES / DESIGN SYSTEM TOKENS sections below describe the underlying PenNode schema and apply to EITHER mode — they tell you the shape of node arguments inside an `<op_tool>` call, and the shape of nodes in raw JSONL.

TYPES:
frame (width,height,layout,gap,padding,justifyContent,alignItems,clipContent,cornerRadius,fill,stroke,effects), rectangle, ellipse, text (content,fontFamily,fontSize,fontWeight,fontStyle,fill,width,textAlign,textGrowth,lineHeight,letterSpacing), icon_font (iconFontName,width,height,fill), path (d,width,height,fill,stroke), image (width,height,imageSearchQuery,imagePrompt). imagePrompt: describe subject+scene+style, NEVER mention background type (transparent/white/plain). Match composition to aspect ratio.
SHARED: id, type, name, role, x, y, opacity
ROLES: section, row, column, divider | navbar, button, icon-button, badge, input, search-bar | card, stat-card, pricing-card, feature-card | heading, subheading, body-text, caption, label | table, table-row, table-header
width/height: number | "fill_container" | "fit_content". padding: number | [v,h] | [T,R,B,L]. Fill=[{"type":"solid","color":"#hex" | "$color-*"}].
Stroke: {"thickness":N,"fill":[{"type":"solid","color":"#hex" | "$color-*"}]}. Directional: {"thickness":{"bottom":1},"fill":[...]}.

RULES:

- Section root: width="fill_container", height="fit_content", layout="vertical".
- No x/y on children in layout frames. All nodes descend from section root.
- Width consistency: siblings in vertical layout use the SAME width strategy.
- Never "fill_container" inside "fit_content" parent.
- clipContent: true on cards with cornerRadius + image children.
- Text: NEVER set height. Short text (titles, labels, buttons) — omit textGrowth. Long text (>15 chars wrapping) — textGrowth="fixed-width", width="fill_container", lineHeight=1.4-1.6.
- lineHeight: Display 40-56px - 0.9-1.0. Heading 20-36px - 1.0-1.2. Body - 1.4-1.6. letterSpacing: -0.5 to -1 for headlines, 1-3 for uppercase.
- Icons: ALWAYS use icon_font nodes with iconFontName (lucide names: search, bell, user, heart, star, plus, x, check, chevron-right, settings, etc). Sizes: 14/20/24px. NEVER use emoji characters as icon substitutes — they cannot render on canvas.
- CJK fonts: "Noto Sans SC"/"Noto Sans JP"/"Noto Sans KR" for headings. CJK lineHeight: 1.3-1.4 headings, 1.6-1.8 body.
- Buttons: frame(padding=[12,24], justifyContent="center") > text. Icon+text: frame(layout="horizontal", gap=8, alignItems="center", padding=[8,16]).
- Card rows: ALL cards width="fill_container" + height="fill_container".
- FORMS: ALL inputs AND button use width="fill_container". gap=16-20.
- Z-order: Earlier siblings render on top. Overlay elements (badges, indicators, floating buttons) MUST come BEFORE the content they overlap.

DESIGN SYSTEM TOKENS — prefer refs over literals so output respects the user's design system. The renderer resolves refs against `doc.variables` (or a default light palette when un-seeded), so refs are SAFE even when the doc has no design system seeded yet.

- COLORS: `$color-{bg-deep|surface|surface-2|surface-3|border|border-strong|text-primary|text-body|text-muted|text-subtle|accent|destructive|success|scrim|info-bg|info-text|success-bg|success-text|warning-bg|warning-text|danger-bg|danger-text|chart-1..6}`. Light defaults: bg-deep `#F8FAFC`, surface `#FFFFFF`, surface-2 `#F1F5F9`, border `#E2E8F0`, text-primary `#0F172A`, text-body `#334155`, text-muted `#64748B`, text-subtle `#94A3B8`, accent `#2563EB`, destructive `#EF4444`, success `#10B981`.
- TYPOGRAPHY: `$type-{display|h1|h2|h3|body|caption}-{size|weight|line-height}`. Defaults: display 64/700/1.0, h1 24/600/1.2, h2 20/600/1.25, h3 16/600/1.3, body 14/400/1.5, caption 12/400/1.4. Plus `$type-display-letter-spacing` (-0.5), `$type-uppercase-label-letter-spacing` (1.5).
- SPACING / RADIUS: `$spacing-{1|2|3|4|5}` = 4/8/12/16/24 px. `$radius-{sm|md|lg}` = 4/8/12 px.

USE refs for: standard semantic colors (page bg, surfaces, text levels, borders, accent, alerts, charts), and typography sizes/weights/line-heights that match the scale above. KEEP literal hex / numbers for: brand-specific colors not in the palette (custom logo color, off-palette accent), pixel values that don't match the typography scale, and "white text on accent" (`#FFFFFF`).

— JSONL FALLBACK MODE — the section below applies ONLY when there is no `<op_tool>` instruction earlier in the prompt; if `<op_tool>` mode is in effect, ignore the FORMAT section and the ```json example below and emit `<op_tool>` tags instead.

FORMAT: \_parent (null=root, else parent-id). Parent before children. Output a single ```json block with ONE node per line.

```json
{"_parent":null,"id":"root","type":"frame","name":"Hero","width":"fill_container","height":"fit_content","layout":"vertical","gap":24,"padding":[48,24],"fill":[{"type":"solid","color":"$color-bg-deep"}]}
{"_parent":"root","id":"header","type":"frame","name":"Header","justifyContent":"space_between","alignItems":"center","width":"fill_container"}
{"_parent":"header","id":"logo","type":"text","name":"Logo","content":"ACME","fontSize":18,"fontWeight":600,"fontFamily":"Space Grotesk","fill":[{"type":"solid","color":"$color-text-primary"}]}
{"_parent":"header","id":"notifBtn","type":"frame","name":"Notification","width":44,"height":44}
{"_parent":"notifBtn","id":"notifIcon","type":"icon_font","name":"Bell","iconFontName":"bell","width":20,"height":20,"fill":"$color-text-primary","x":12,"y":12}
{"_parent":"root","id":"title","type":"text","name":"Headline","content":"Learn Smarter","fontSize":48,"fontWeight":700,"fontFamily":"Space Grotesk","lineHeight":0.95,"fill":[{"type":"solid","color":"$color-text-primary"}]}
{"_parent":"root","id":"desc","type":"text","name":"Description","content":"AI-powered vocabulary learning that adapts to your pace","fontSize":16,"textGrowth":"fixed-width","width":"fill_container","lineHeight":1.5,"fill":[{"type":"solid","color":"$color-text-muted"}]}
{"_parent":"root","id":"cta","type":"frame","name":"CTA Button","padding":[14,28],"cornerRadius":10,"justifyContent":"center","fill":[{"type":"solid","color":"$color-accent"}]}
{"_parent":"cta","id":"cta-text","type":"text","name":"CTA Label","content":"Get Started","fontSize":16,"fontWeight":600,"fill":[{"type":"solid","color":"#FFFFFF"}]}
```
