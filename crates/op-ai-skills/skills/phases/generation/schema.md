---
name: schema
description: PenNode type definitions and property schemas
phase: [generation]
trigger: null
priority: 0
budget: 2000
category: base
---

PenNode types (the ONLY format you output for designs):

- frame: Container. Props: width, height, layout ('none'|'vertical'|'horizontal'), gap, padding, justifyContent ('start'|'center'|'end'|'space_between'|'space_around'), alignItems ('start'|'center'|'end'), clipContent (boolean), children[], cornerRadius, fill, stroke, effects. layout:"none" is ONLY for overlay stacking (children pile up at the frame origin unless given explicit x/y) — NEVER use it for a row or column of cards/items; use 'horizontal'/'vertical' for those
- rectangle: Props: width, height, cornerRadius, fill, stroke, effects
- ellipse: Props: width, height, fill, stroke, effects, innerRadius (0..1 donut hole), startAngle, sweepAngle (degrees, for pie/arc/gauge). NEVER put a ring's track and progress arc side by side as flex siblings — stack both at the same x/y inside one fixed layout:"none" wrapper
- text: Props: content, fontFamily, fontSize, fontWeight, fontStyle ('normal'|'italic'), fill, width, height, textAlign ('left'|'center'|'right'|'justify' — NEVER 'start'/'end'; those are container-axis values), textGrowth ('auto'|'fixed-width'|'fixed-width-height'), lineHeight (multiplier), letterSpacing (px), textAlignVertical ('top'|'middle'|'bottom')
- path: SVG icon. Props: d (SVG path), width, height, fill, stroke, effects
- image: Props: width, height, cornerRadius, effects, imageSearchQuery (2-3 English keywords UNIQUE per image — derive from the surrounding card/dish/title text; reusing one query across multiple images makes every card render the same photo. For food cards, use prepared-dish queries like "pasta plate", "salmon bowl", "pizza plate", "sushi platter"; avoid ingredient-only, outdoor/grass, raw-object, or novelty queries), imagePrompt (a fuller natural-language description of the SAME subject for AI image generation — e.g. "professional food photography of a pasta plate, warm natural light, shallow depth of field". ALWAYS emit it alongside imageSearchQuery: a configured image-gen model uses imagePrompt for a rich original image, otherwise imageSearchQuery drives the stock-search fallback — so every image element carries both)
IMAGE SLOT CONTRACT: an image slot is an `image` node with `imageSearchQuery` (+ `imagePrompt`); a bare rectangle with an image fill is forbidden.
- text_input: First-class single-line control. Props: width, height, placeholder, value, leadingIcon, trailingIcon, fill, stroke, cornerRadius, effects
- text_area: First-class multi-line control. Props: width, height, placeholder, value, maxVisibleLines, leadingIcon, trailingIcon, fill, stroke, cornerRadius, effects
- select: First-class dropdown. Props: width, height, options: [{value,label}], value, placeholder, fill, stroke, cornerRadius, effects
- switch: First-class toggle. Props: width, height, checked, fill, stroke, cornerRadius, effects
- checkbox: First-class checkbox. Props: width, height, checked, label, fill, stroke, cornerRadius, effects
- slider: First-class range control. Props: width, height, min, max, step, value, fill, stroke, cornerRadius, effects
- radio_group: First-class single-choice control. Props: width, height, options: [{value,label}], value, fill, stroke, cornerRadius, effects
- number_input: First-class numeric control. Props: width, height, placeholder, min, max, step, value, fill, stroke, cornerRadius, effects
- progress: First-class display control. Props: width, height, max, value, indeterminate, fill, stroke, cornerRadius, effects
- tabs: First-class tab container. Props: width, height, tabs: [{value,label}], value, children[] (one panel per tab), fill, stroke, cornerRadius, effects

INTERACTIVE CONTROL CONTRACT: Emit the native types above directly; NEVER build a frame/rectangle/text lookalike or a new role-marked frame. Supply every semantic prop listed for the chosen control (`options`/`tabs` and `value`, `checked`, or `min`/`max`/`step`/`value`) rather than depending on defaults. Every control MUST explicitly carry design-system-derived `fill`, `stroke`, and `cornerRadius`: `fill` is the active/accent paint (or the field surface), while `stroke.fill` is the inactive track/border paint.

PROPERTY NAMES are camelCase and are ALWAYS one unbroken identifier. Never split one at a word boundary: `justify.content` ✗ / `justify_content` ✗ / `"justify-content"` ✗ — write `justifyContent` ✓. Same for `alignItems`, `cornerRadius`, `fontSize`, `lineHeight`, `clipContent`, `imageSearchQuery`. A dotted key is a syntax error that throws away the entire script, not just that one property.
All nodes share: id, type, name, role, x, y, rotation, opacity
NODE NAMING: every node carries a meaningful `name` saying what it is (e.g. "cta-button", "hero-image"); an anonymous box is a defect — never emit `name: ""`.
Interactivity (multi-screen apps only): frame accepts `screen` (a top-level frame's route path — `"/"` for the entry screen, `"/slug"` for every other screen, unique across the document) and `events` (`{ onTap: [ {"replace": "\"/path\""} | {"push": "\"/path\""} | {"pop": null} ] }` — the action body is the literal JSON STRING `"\"/path\""`, quote characters included, since it compiles as an expression; a bare `/path` fails). Never write `route` — it is schema-only metadata the tap dispatcher ignores.
Fill = [{ type: "solid", color: "#hex" }] or [{ type: "linear_gradient", angle, stops: [{ offset, color }] }] or [{ type: "radial_gradient", stops: [{ offset, color }] }] or [{ type: "mesh_gradient", rows, cols, stops: [{ row, col, color }] }]
- radial_gradient example (concentric glow): `fill: [{ type: "radial_gradient", cx: 0.5, cy: 0.5, radius: 0.7, stops: [{ offset: 0, color: "#6d28d9" }, { offset: 1, color: "#0b0614" }] }]` — cx/cy are 0..1 fractions of the box (0.5 = centre); radius is a 0..1 fraction of max(w,h).
- mesh_gradient example (smooth four-corner blend): `fill: [{ type: "mesh_gradient", rows: 2, cols: 2, stops: [{ row: 0, col: 0, color: "#ec4899" }, { row: 0, col: 1, color: "#8b5cf6" }, { row: 1, col: 0, color: "#3b82f6" }, { row: 1, col: 1, color: "#06b6d4" }] }]` — a rows×cols vertex grid Gouraud-interpolated across the box; each stop pins one vertex colour at (row, col). Use it for rich multi-hue hero/background panels instead of stacking layers.
- shader fill (advanced, generative noise/aurora/glow): `fill: [{ type: "shader", sksl: "half4 main(float2 p){ ... }", uniforms: {...} }]` — native SkSL, render-only. See the `shader-fill` knowledge skill (triggers on shader/glsl/sksl/generative/noise/aurora). PREFER mesh_gradient/linear_gradient unless the intent is explicitly generative; a failed shader degrades to a flat solid.
Stroke = { thickness, fill: [...] } — thickness accepts number (uniform) | [T, R, B, L] | { top, right, bottom, left } (per-side, e.g. { bottom: 1 } for an underline). Effects = [{ type: "shadow", offsetX, offsetY, blur, spread, color }]
SIZING: width/height accept number (px), "fill_container", or "fit_content".
RAIL CARDS: cards in a horizontal scroll row must ALL share the same fixed pixel width — never "fill_container" (it crushes them to a sliver beside a fixed sibling).
PADDING: number (uniform), [v, h], or [top, right, bottom, left].
cornerRadius is a number. fill is ALWAYS an array. Do NOT set x/y on children inside layout frames.
