---
name: jsonl-format
description: Sub-agent flat JSONL output format with node types and rules
phase: [generation]
trigger: null
priority: 0
budget: 1500
category: base
---

PenNode flat JSONL engine. Output a ```json block with ONE node per line.

TYPES:
frame (width,height,layout,gap,padding,justifyContent,alignItems,clipContent,cornerRadius,fill,stroke,effects), rectangle, ellipse, text (content,fontFamily,fontSize,fontWeight,fontStyle,fill,width,textAlign,textGrowth,lineHeight,letterSpacing), icon_font (iconFontName,width,height,fill), path (d,width,height,fill,stroke), image (width,height,imageSearchQuery,imagePrompt). imagePrompt: describe subject+scene+style, NEVER mention background type (transparent/white/plain). Match composition to aspect ratio.
SHARED: id, type, name, role, x, y, opacity
ROLES: section, row, column, divider | navbar, button, icon-button, badge, input, search-bar | card, stat-card, pricing-card, feature-card | heading, subheading, body-text, caption, label | table, table-row, table-header
width/height: number | "fill_container" | "fit_content". padding: number | [v,h] | [T,R,B,L]. Fill=[{"type":"solid","color":"#hex"}].
Stroke: {"thickness":N,"fill":[{"type":"solid","color":"#hex"}]}. Directional: {"thickness":{"bottom":1},"fill":[...]}.

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
- Phone mockup: ONE frame, w=260-300, h=520-580, cornerRadius=32, solid fill + 1px stroke.
- Z-order: Earlier siblings render on top. Overlay elements (badges, indicators, floating buttons) MUST come BEFORE the content they overlap.

FORMAT: _parent (null=root, else parent-id). Parent before children.
```json
{"_parent":null,"id":"root","type":"frame","name":"Hero","width":"fill_container","height":"fit_content","layout":"vertical","gap":24,"padding":[48,24],"fill":[{"type":"solid","color":"#F8FAFC"}]}
{"_parent":"root","id":"header","type":"frame","name":"Header","justifyContent":"space_between","alignItems":"center","width":"fill_container"}
{"_parent":"header","id":"logo","type":"text","name":"Logo","content":"ACME","fontSize":18,"fontWeight":600,"fontFamily":"Space Grotesk","fill":[{"type":"solid","color":"#0D0D0D"}]}
{"_parent":"header","id":"notifBtn","type":"frame","name":"Notification","width":44,"height":44}
{"_parent":"notifBtn","id":"notifIcon","type":"icon_font","name":"Bell","iconFontName":"bell","width":20,"height":20,"fill":"#0D0D0D","x":12,"y":12}
{"_parent":"root","id":"title","type":"text","name":"Headline","content":"Learn Smarter","fontSize":48,"fontWeight":700,"fontFamily":"Space Grotesk","lineHeight":0.95,"fill":[{"type":"solid","color":"#0F172A"}]}
{"_parent":"root","id":"desc","type":"text","name":"Description","content":"AI-powered vocabulary learning that adapts to your pace","fontSize":16,"textGrowth":"fixed-width","width":"fill_container","lineHeight":1.5,"fill":[{"type":"solid","color":"#64748B"}]}
{"_parent":"root","id":"cta","type":"frame","name":"CTA Button","padding":[14,28],"cornerRadius":10,"justifyContent":"center","fill":[{"type":"solid","color":"#2563EB"}]}
{"_parent":"cta","id":"cta-text","type":"text","name":"CTA Label","content":"Get Started","fontSize":16,"fontWeight":600,"fill":[{"type":"solid","color":"#FFFFFF"}]}
```

CRITICAL: Output ONLY the ```json block. Do NOT write any text, explanation, plan, tool calls, or function calls. Do NOT use [TOOL_CALL] or {tool => ...} syntax. Start your response with ```json immediately.
