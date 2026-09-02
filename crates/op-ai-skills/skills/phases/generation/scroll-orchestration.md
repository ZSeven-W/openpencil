---
name: scroll-orchestration
description: Deep scroll-progress, sticky-pin, and property-animation recipes for landing and brand pages
phase: [generation]
trigger:
  keywords: [滚动, 视差, parallax, scroll, sticky, 入场动画, 交错, stagger, scrollytelling, scroll-progress]
priority: 24
budget: 3300
category: domain
---

SCROLL ORCHESTRATION is for vertical viewport scroll orchestration and is
not an ordinary horizontal card rail. Basic state, events, and navigation stay
in `interactivity`; this topic composes scroll-progress + sticky-pin + animate.

## Exact runtime contract

- A scroller is a fixed-height, overflowing, `clipContent:true` frame with a
  nonempty `events.onScroll`. Descendants read the nearest `$scroll.offset`,
  `$scroll.maxOffset`, `$scroll.progress` (0..1), and `$scroll.direction`
  (`"up"|"down"|"none"`) in expression strings under `bindings`.
- `$scroll` may drive only PaintOnly targets: `opacity`, `fill`, `stroke`,
  `textColor`, `cornerRadius`, widget `value`, `checked`, `selectedValue`.
  Never bind it to `x`, `y`, `rotation`, `scaleX`, `scaleY`, `width`, `height`,
  `visible`, or `content`. Use a Progress node's `value`, not rectangle width.
- `stickyChildren:["id"]` lists direct children of the scroller; `pin:true`
  holds the authored viewport position. This is unconditional pinning, not
  threshold-based CSS sticky. Prefer direct children; ordinary siblings move.
- `animate` requires literal `target`, registered `property`, literal `to`, and
  positive integer `durationMs`. Optional: literal `from`, `delayMs` (default
  0), `easing`, `iterations` (1..1000), `direction`, `fillMode`. Properties:
  `opacity`, `x`, `y`, `rotation`, `scaleX`, `scaleY`, `fill`, `stroke`,
  `cornerRadius`, `width`, `height`; width/height switch discretely at the end.
  Easings: `linear`, `ease`, `ease_in`, `ease_out`, `ease_in_out`. Directions:
  `normal`, `reverse`, `alternate`, `alternate_reverse`. Fill modes: `none`,
  `forwards`, `backwards`, `both`.
- Put one effect's animates in one action list. They share a clock and do not
  await one another; sequence/stagger uses increasing `delayMs`. Delayed tracks
  need `backwards` or `both`. A later request replaces the same target/property
  track. Stay below 256 live tracks. Animate is event-clocked, never scroll scrub.
- Sticky nav items may call `scroll_to` with literal `target` and `alignment`:
  `start`, `center`, `end`, or `nearest`.

## 1. Fixed-layer faux parallax (A15 boundary-safe form)

Pin one depth layer while tall foreground content scrolls. This is 0x/1x faux
parallax, not arbitrary `y` or scale rates.

```json
{
  "type": "frame", "id": "parallax-scroll", "width": 1440, "height": 800,
  "layout": "none", "clipContent": true,
  "state": { "seen": { "type": "bool", "default": false } },
  "stickyChildren": ["parallax-sky"],
  "events": { "onScroll": [{ "set": { "$state.seen": "true" } }] },
  "children": [
    { "type": "rectangle", "id": "parallax-sky", "pin": true,
      "x": 0, "y": 0, "width": 1440, "height": 800, "opacity": 1,
      "fill": [{ "type": "linear_gradient", "angle": 135, "stops": [{ "offset": 0, "color": "#172554" }, { "offset": 1, "color": "#581C87" }] }],
      "bindings": { "opacity": "1 - $scroll.progress * 0.35" } },
    { "type": "frame", "id": "parallax-foreground", "x": 0, "y": 0,
      "width": 1440, "height": 1600, "layout": "none", "children": [
        { "type": "text", "id": "parallax-title", "x": 120, "y": 260,
          "width": 900, "height": 160, "content": "DEPTH IN MOTION",
          "fontSize": 96, "fontWeight": 700,
          "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
        { "type": "rectangle", "id": "parallax-card", "x": 120, "y": 980,
          "width": 640, "height": 360, "cornerRadius": 32,
          "fill": [{ "type": "solid", "color": "#FFFFFF" }] }
      ] }
  ]
}
```

## 2. Pinned progress value

```json
{
  "type": "frame", "id": "progress-scroll", "width": 1200, "height": 600,
  "layout": "none", "clipContent": true,
  "state": { "seen": { "type": "bool", "default": false } },
  "stickyChildren": ["page-progress"],
  "events": { "onScroll": [{ "set": { "$state.seen": "true" } }] },
  "children": [
    { "type": "rectangle", "id": "progress-content", "x": 0, "y": 0,
      "width": 1200, "height": 1800,
      "fill": [{ "type": "solid", "color": "#F8FAFC" }] },
    { "type": "progress", "id": "page-progress", "pin": true,
      "x": 24, "y": 24, "width": 1152, "height": 8, "max": 100, "value": 0,
      "fill": [{ "type": "solid", "color": "#7C3AED" }],
      "cornerRadius": 4, "bindings": { "value": "$scroll.progress * 100" } }
  ]
}
```

## 3. Scroll-window opacity stagger (A6 boundary-safe form)

Each card gets a later `clamp` window. This is container-progress choreography,
not an element-enter observer.

```json
{
  "type": "frame", "id": "reveal-scroll", "width": 1200, "height": 640,
  "layout": "none", "clipContent": true,
  "state": { "seen": { "type": "bool", "default": false } },
  "events": { "onScroll": [{ "set": { "$state.seen": "true" } }] },
  "children": [
    { "type": "rectangle", "id": "reveal-card-1", "x": 96, "y": 500,
      "width": 1008, "height": 260, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#EDE9FE" }],
      "bindings": { "opacity": "clamp(($scroll.progress - 0.05) / 0.18, 0, 1)" } },
    { "type": "rectangle", "id": "reveal-card-2", "x": 96, "y": 860,
      "width": 1008, "height": 260, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#DBEAFE" }],
      "bindings": { "opacity": "clamp(($scroll.progress - 0.25) / 0.18, 0, 1)" } },
    { "type": "rectangle", "id": "reveal-card-3", "x": 96, "y": 1220,
      "width": 1008, "height": 260, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#DCFCE7" }],
      "bindings": { "opacity": "clamp(($scroll.progress - 0.45) / 0.18, 0, 1)" } }
  ]
}
```

## 4. Sticky navigation style shift (A8 boundary-safe form)

```json
{
  "type": "frame", "id": "nav-scroll", "width": 1440, "height": 760,
  "layout": "none", "clipContent": true,
  "state": { "seen": { "type": "bool", "default": false } },
  "stickyChildren": ["sticky-nav"],
  "events": { "onScroll": [{ "set": { "$state.seen": "true" } }] },
  "children": [
    { "type": "frame", "id": "nav-content", "x": 0, "y": 0,
      "width": 1440, "height": 2100, "layout": "none", "children": [
        { "type": "rectangle", "id": "section-1", "x": 0, "y": 0,
          "width": 1440, "height": 700, "fill": [{ "type": "solid", "color": "#020617" }] },
        { "type": "rectangle", "id": "section-2", "x": 0, "y": 700,
          "width": 1440, "height": 700, "fill": [{ "type": "solid", "color": "#111827" }] },
        { "type": "rectangle", "id": "section-3", "x": 0, "y": 1400,
          "width": 1440, "height": 700, "fill": [{ "type": "solid", "color": "#312E81" }] }
      ] },
    { "type": "frame", "id": "sticky-nav", "pin": true,
      "x": 48, "y": 24, "width": 1344, "height": 72, "layout": "horizontal",
      "padding": [0, 28], "alignItems": "center", "cornerRadius": 12,
      "fill": [{ "type": "solid", "color": "#10131A00" }],
      "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": "#334155" }] },
      "bindings": {
        "fill": "$scroll.offset > 24 ? \"#10131ACC\" : \"#10131A00\"",
        "stroke": "$scroll.direction == \"down\" ? \"#7C3AED\" : \"#334155\"",
        "cornerRadius": "$scroll.offset > 24 ? 24 : 12"
      },
      "children": [
        { "type": "text", "id": "nav-brand", "width": "fit_content", "height": 32,
          "content": "NORTHSTAR", "fontSize": 20, "fontWeight": 700,
          "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
        { "type": "text", "id": "nav-chapter-2", "width": "fit_content", "height": 32,
          "content": "Chapter 2", "fontSize": 16,
          "fill": [{ "type": "solid", "color": "#FFFFFF" }],
          "events": { "onTap": [{ "scroll_to": { "target": "section-2", "alignment": "start" } }] } },
        { "type": "text", "id": "nav-chapter-3", "width": "fit_content", "height": 32,
          "content": "Chapter 3", "fontSize": 16,
          "fill": [{ "type": "solid", "color": "#FFFFFF" }],
          "events": { "onTap": [{ "scroll_to": { "target": "section-3", "alignment": "start" } }] } }
      ] }
  ]
}
```

## 5. Sticky scrollytelling crossfade (A9/B17)

```json
{
  "type": "frame", "id": "story-scroll", "width": 1200, "height": 700,
  "layout": "none", "clipContent": true,
  "state": { "seen": { "type": "bool", "default": false } },
  "stickyChildren": ["story-stage"],
  "events": { "onScroll": [{ "set": { "$state.seen": "true" } }] },
  "children": [
    { "type": "rectangle", "id": "story-track", "x": 0, "y": 0,
      "width": 1200, "height": 2400,
      "fill": [{ "type": "solid", "color": "#09090B" }] },
    { "type": "frame", "id": "story-stage", "pin": true,
      "x": 0, "y": 0, "width": 1200, "height": 700, "layout": "none",
      "fill": [{ "type": "solid", "color": "#09090B" }], "children": [
        { "type": "text", "id": "chapter-a", "x": 120, "y": 240,
          "width": 960, "height": 140, "content": "DISCOVER", "fontSize": 88,
          "fill": [{ "type": "solid", "color": "#FFFFFF" }],
          "bindings": { "opacity": "clamp(1 - $scroll.progress * 3, 0, 1)" } },
        { "type": "text", "id": "chapter-b", "x": 120, "y": 240,
          "width": 960, "height": 140, "content": "COMPOSE", "fontSize": 88,
          "fill": [{ "type": "solid", "color": "#C4B5FD" }],
          "bindings": { "opacity": "clamp(1 - abs($scroll.progress - 0.5) * 4, 0, 1)" } },
        { "type": "text", "id": "chapter-c", "x": 120, "y": 240,
          "width": 960, "height": 140, "content": "SHIP", "fontSize": 88,
          "fill": [{ "type": "solid", "color": "#67E8F9" }],
          "bindings": { "opacity": "clamp(($scroll.progress - 0.66) * 3, 0, 1)" } }
      ] }
  ]
}
```

## 6. First-scroll guarded animate stagger

Guard `onScroll`; otherwise every wheel sample restarts the time tracks.

```json
{
  "type": "frame", "id": "animate-scroll", "width": 1200, "height": 640,
  "layout": "none", "clipContent": true,
  "state": { "revealed": { "type": "bool", "default": false } },
  "events": { "onScroll": [{ "if": {
    "expr": "!$state.revealed",
    "then": [
      { "animate": { "target": "entry-card-1", "property": "opacity", "from": 0, "to": 1, "durationMs": 360, "delayMs": 0, "easing": "ease_out", "iterations": 1, "direction": "normal", "fillMode": "both" } },
      { "animate": { "target": "entry-card-2", "property": "opacity", "from": 0, "to": 1, "durationMs": 360, "delayMs": 90, "easing": "ease_out", "iterations": 1, "direction": "normal", "fillMode": "both" } },
      { "animate": { "target": "entry-card-3", "property": "opacity", "from": 0, "to": 1, "durationMs": 360, "delayMs": 180, "easing": "ease_out", "iterations": 1, "direction": "normal", "fillMode": "both" } },
      { "set": { "$state.revealed": "true" } }
    ]
  } }] },
  "children": [
    { "type": "rectangle", "id": "entry-card-1", "x": 80, "y": 520,
      "width": 320, "height": 300, "opacity": 0, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#DDD6FE" }] },
    { "type": "rectangle", "id": "entry-card-2", "x": 440, "y": 520,
      "width": 320, "height": 300, "opacity": 0, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#BFDBFE" }] },
    { "type": "rectangle", "id": "entry-card-3", "x": 800, "y": 520,
      "width": 320, "height": 900, "opacity": 0, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#A7F3D0" }] }
  ]
}
```

## 7. Pinned footer opacity reveal + theme shift (B24/B25)

```json
{
  "type": "frame", "id": "footer-scroll", "width": 1440, "height": 760,
  "layout": "none", "clipContent": true,
  "state": { "seen": { "type": "bool", "default": false } },
  "stickyChildren": ["footer-drawer"],
  "events": { "onScroll": [{ "set": { "$state.seen": "true" } }] },
  "children": [
    { "type": "rectangle", "id": "footer-content", "x": 0, "y": 0,
      "width": 1440, "height": 2100,
      "fill": [{ "type": "solid", "color": "#F8FAFC" }] },
    { "type": "frame", "id": "footer-drawer", "pin": true,
      "x": 0, "y": 580, "width": 1440, "height": 180, "layout": "horizontal",
      "padding": [0, 72], "alignItems": "center", "opacity": 0,
      "fill": [{ "type": "solid", "color": "#111827" }],
      "bindings": {
        "opacity": "clamp(($scroll.progress - 0.82) / 0.18, 0, 1)",
        "fill": "$scroll.progress > 0.92 ? \"#F4F0FF\" : \"#111827\"",
        "cornerRadius": "$scroll.progress > 0.92 ? 28 : 0"
      },
      "children": [{ "type": "text", "id": "footer-label", "width": "fit_content",
        "height": 48, "content": "THE NEXT CHAPTER", "fontSize": 36,
        "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        "bindings": { "textColor": "$scroll.progress > 0.92 ? \"#312E81\" : \"#FFFFFF\"" } }] }
  ]
}
```

Do not invent scroll-bound transform/layout scrub, velocity, element-enter
observers, springs, keyframes, image-frame selection, or dynamic text. The D
group's infinite/horizontal galleries need missing append/x-axis primitives and
are not approximated here.

END SCROLL ORCHESTRATION
