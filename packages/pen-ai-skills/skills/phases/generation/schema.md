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
- image: Props: width, height, cornerRadius, effects, imageSearchQuery (2-3 English keywords)

All nodes share: id, type, name, role, x, y, rotation, opacity
Fill = [{ type: "solid", color: "#hex" }] or [{ type: "linear_gradient", angle, stops: [{ offset, color }] }]
Stroke = { thickness, fill: [...] }  Effects = [{ type: "shadow", offsetX, offsetY, blur, spread, color }]
SIZING: width/height accept number (px), "fill_container", or "fit_content".
PADDING: number (uniform), [v, h], or [top, right, bottom, left].
cornerRadius is a number. fill is ALWAYS an array. Do NOT set x/y on children inside layout frames.
