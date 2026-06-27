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

- frame: Container. Props: width, height, layout ('none'|'vertical'|'horizontal'), gap, padding, justifyContent ('start'|'center'|'end'|'space_between'|'space_around'), alignItems ('start'|'center'|'end'), clipContent (boolean), children[], cornerRadius, fill, stroke, effects
- rectangle: Props: width, height, cornerRadius, fill, stroke, effects
- ellipse: Props: width, height, fill, stroke, effects, innerRadius (0..1 donut hole), startAngle, sweepAngle (degrees, for pie/arc/gauge)
- text: Props: content, fontFamily, fontSize, fontWeight, fontStyle ('normal'|'italic'), fill, width, height, textAlign, textGrowth ('auto'|'fixed-width'|'fixed-width-height'), lineHeight (multiplier), letterSpacing (px), textAlignVertical ('top'|'middle'|'bottom')
- path: SVG icon. Props: d (SVG path), width, height, fill, stroke, effects
- image: Props: width, height, cornerRadius, effects, imageSearchQuery (2-3 English keywords UNIQUE per image — derive from the surrounding card/dish/title text; reusing one query across multiple images makes every card render the same photo. For food cards, use prepared-dish queries like "pasta plate", "salmon bowl", "pizza plate", "sushi platter"; avoid ingredient-only, outdoor/grass, raw-object, or novelty queries)

All nodes share: id, type, name, role, x, y, rotation, opacity
Fill = [{ type: "solid", color: "#hex" }] or [{ type: "linear_gradient", angle, stops: [{ offset, color }] }] or [{ type: "radial_gradient", stops: [{ offset, color }] }] or [{ type: "mesh_gradient", rows, cols, stops: [{ row, col, color }] }]
- radial_gradient example (concentric glow): `fill: [{ type: "radial_gradient", cx: 0.5, cy: 0.5, radius: 0.7, stops: [{ offset: 0, color: "#6d28d9" }, { offset: 1, color: "#0b0614" }] }]` — cx/cy are 0..1 fractions of the box (0.5 = centre); radius is a 0..1 fraction of max(w,h).
- mesh_gradient example (smooth four-corner blend): `fill: [{ type: "mesh_gradient", rows: 2, cols: 2, stops: [{ row: 0, col: 0, color: "#ec4899" }, { row: 0, col: 1, color: "#8b5cf6" }, { row: 1, col: 0, color: "#3b82f6" }, { row: 1, col: 1, color: "#06b6d4" }] }]` — a rows×cols vertex grid Gouraud-interpolated across the box; each stop pins one vertex colour at (row, col). Use it for rich multi-hue hero/background panels instead of stacking layers.
- shader fill (advanced, generative noise/aurora/glow): `fill: [{ type: "shader", sksl: "half4 main(float2 p){ ... }", uniforms: {...} }]` — native SkSL, render-only. See the `shader-fill` knowledge skill (triggers on shader/glsl/sksl/generative/noise/aurora). PREFER mesh_gradient/linear_gradient unless the intent is explicitly generative; a failed shader degrades to a flat solid.
Stroke = { thickness, fill: [...] } — thickness accepts number (uniform) | [T, R, B, L] | { top, right, bottom, left } (per-side, e.g. { bottom: 1 } for an underline). Effects = [{ type: "shadow", offsetX, offsetY, blur, spread, color }]
SIZING: width/height accept number (px), "fill_container", or "fit_content".
PADDING: number (uniform), [v, h], or [top, right, bottom, left].
cornerRadius is a number. fill is ALWAYS an array. Do NOT set x/y on children inside layout frames.
