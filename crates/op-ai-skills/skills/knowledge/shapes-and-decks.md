---
name: shapes-and-decks
description: Full worked examples for concentric progress rings/donuts/gauges and stacked-card decks (companion depth to the layout skill's compact rules)
phase: [generation]
trigger:
  keywords: [ring, donut, gauge, pie chart, progress ring, activity ring, card deck, stacked deck, stacked card, card stack, swipeable, flashcard, 进度环, 甜甜圈, 仪表盘, 堆叠卡]
priority: 28
budget: 1200
category: knowledge
---

CONCENTRIC RING / DONUT / GAUGE — WORKED EXAMPLE:

The compact rule set lives in the `layout` skill (ABSOLUTE-STACK Z-ORDER +
the RING/PIE/ARC/DONUT/GAUGE/DISC section). This is the full worked tree for
a progress ring with centered content — a fixed-size `layout="none"` wrapper
overlaying same-size track/progress ellipses plus a centered content frame,
NEVER as flex siblings (flex would place them beside each other instead of
making them concentric):

```
frame(width=80, height=80, layout="none")
├── frame(x=10, y=24, width=60, height=32, layout="horizontal", alignItems="center", justifyContent="center")
│   └── text(content="8,432", fontSize=16, fontWeight=700, fill:[textColor])  ← centered content, topmost
├── ellipse(x=0, y=0, width=80, height=80, innerRadius=0.85, startAngle=-90, sweepAngle=270, fill:[progressColor])
└── ellipse(x=0, y=0, width=80, height=80, innerRadius=0.85, fill:[trackColor])
```

Because `layout="none"` paints lower array indexes on top (per the layout
skill's ABSOLUTE-STACK Z-ORDER rule), the center content comes first, then
the progress ellipse, then the track ellipse last (bottom of the stack).
Choose explicit center-child bounds, then compute `x=(wrapperWidth-centerWidth)/2`
and `y=(wrapperHeight-centerHeight)/2` whenever sizes change. textAlignVertical
is not supported — center text/icon inside the explicit center child via
`alignItems="center"` + `justifyContent="center"`, never on the ring ellipses
themselves.

Simple pattern reference (no frame tricks — a single native ellipse):

- SOLID DISC: `ellipse(width=80, height=80, fill:[{type:"solid", color:...}])`
- EMPTY RING / progress track: `ellipse(width=80, height=80, innerRadius=0.8, fill:[{type:"solid", color:trackColor}])`
- PROGRESS ARC (e.g. 75%): `ellipse(..., innerRadius=0.8, startAngle=-90, sweepAngle=270, fill:[{type:"solid", color:progressColor}])`
- PIE SLICE: `ellipse(..., startAngle=0, sweepAngle=120, fill:[...])` (no innerRadius)
- GAUGE (half ring): `ellipse(..., innerRadius=0.7, startAngle=180, sweepAngle=180, fill:[...])`

A SIMPLE FILLED CIRCLE with centered text/icon (badge, avatar initials) does
not need any of the above — it needs no ellipse at all. Use a fixed square
frame with `cornerRadius=width/2`, `layout="horizontal"`, `alignItems="center"`,
`justifyContent="center"` instead.

STACKED CARD / DECK — WORKED EXAMPLE:

The compact rules live in the `layout` skill (STACKED CARD / DECK section).
This is the full worked tree for a two-layer testimonial-card deck:

```
frame(width=311, height=180, layout="none")                    -- deck wrapper, sized for the front card + peek
├── frame(x=0, y=0, width=295, height=164, cornerRadius=16,     -- children[0] = front card, TOPMOST, opaque
│         layout="vertical", padding=20, gap=8,
│         fill=[{type:"solid", color:"$--card"}])
│   ├── text(content="\"Great tool, saved us hours every week.\"", fontSize=15, fontWeight=500)
│   └── text(content="— Jamie Lee, Product Lead", fontSize=13, fill=[{type:"solid", color:"$--muted-foreground"}])
└── frame(x=16, y=16, width=295, height=164, cornerRadius=16,   -- back layer, decorative only, NO children
          fill=[{type:"solid", color:"$--muted"}],
          stroke={thickness:1, fill:[{type:"solid", color:"$--border"}]})
```

Because `layout="none"` paints lower array indexes on top, the front card is
`children[0]` and the empty back layer is `children[1]`, offset down-right by
16px so only its bottom-right edge peeks out. The back layer is a bare
rectangle/frame with cornerRadius + fill (+ optional stroke) and NO content
children — giving it real content produces a second, upside-down-z-order copy
of the card's text fighting the front card for the same pixels. The front
card carries a real opaque `fill` (here `$--card`) so the back layer
only shows at the peeking edge, not through the middle of the design.

For a 3+ card deck, add more back layers AFTER the front card, each with a
slightly larger offset (e.g. 8px then 16px) and no content — never insert a
new layer before the front card or give any of them text.
