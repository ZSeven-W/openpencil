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
- ellipse: Props: width, height, fill, stroke, effects
- text: Props: content, fontFamily, fontSize, fontWeight, fontStyle ('normal'|'italic'), fill, width, height, textAlign, textGrowth ('auto'|'fixed-width'|'fixed-width-height'), lineHeight (multiplier), letterSpacing (px), textAlignVertical ('top'|'middle'|'bottom')
- path: SVG icon. Props: d (SVG path), width, height, fill, stroke, effects
- image: Props: width, height, cornerRadius, effects, imageSearchQuery (2-3 English keywords UNIQUE per image — derive from the surrounding card/dish/title text; reusing one query across multiple images makes every card render the same photo)

Interactive widgets (first-class nodes — emit these directly, NOT a frame with role):

- text_input: single-line input. Props: placeholder, value, width, height, fill, stroke, cornerRadius. Two-way bind via bindings "bind:value".
- text_area: multi-line input. Props: placeholder, value, maxVisibleLines, + same style props as text_input.
- number_input: numeric input with steppers. Props: placeholder, value, min, max, step.
- select: dropdown. Props: placeholder, value (selected), options: [{ value, label }].
- radio_group: single choice. Props: value (selected), options: [{ value, label }].
- switch: on/off toggle. Props: checked.
- checkbox: Props: checked, label.
- slider: Props: min, max, step, value.
- progress: progress indicator (display-only, not focusable). Props: value, max, indeterminate.
- tabs: tabbed panels (container). Props: tabs: [{ value, label }], value (active tab), children[] (one panel subtree per tab).

Widget interaction state binds to the app state graph via bindings "bind:value" → "$state.<key>" (e.g. an input bound to $state.email). Auto-derived hover/pressed/focused/disabled visuals; optional per-state overrides via `states`.

All nodes share: id, type, name, role, x, y, rotation, opacity
Fill = [{ type: "solid", color: "#hex" }] or [{ type: "linear_gradient", angle, stops: [{ offset, color }] }]
Stroke = { thickness, fill: [...] } Effects = [{ type: "shadow", offsetX, offsetY, blur, spread, color }]
SIZING: width/height accept number (px), "fill_container", or "fit_content".
PADDING: number (uniform), [v, h], or [top, right, bottom, left].
cornerRadius is a number. fill is ALWAYS an array. Do NOT set x/y on children inside layout frames.
