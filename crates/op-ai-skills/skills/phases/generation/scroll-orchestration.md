---
name: scroll-orchestration
description: Page-scroll parallax, sticky-pin, stagger reveal and property-animation recipes for landing and brand pages
phase: [generation]
trigger:
  keywords: [滚动, 视差, parallax, scroll, sticky, 入场动画, 交错, stagger, scrollytelling, scroll-progress]
priority: 24
budget: 3300
category: domain
---

SCROLL ORCHESTRATION is for vertical viewport scroll orchestration and is
not an ordinary horizontal card rail. Basic state, events, and navigation stay
in `interactivity`; this topic composes page scroll + pin + bindings + animate.

## Page-scroll contract (the page IS the scroller)

- In preview the page root scrolls as a whole, like `window.scrollY`. Any
  `$scroll` reference with no explicit scroller ancestor reads the PAGE scroll:
  `$scroll.offset` (px), `$scroll.maxOffset`, `$scroll.progress` (0..1, stays 0
  while the page has no overflow), `$scroll.direction` (`"up"|"down"|"none"`).
- NEVER wrap sections in a fixed-height clipped frame with a scroll handler to
  get scrolling on a landing/brand page — that builds a second scroller inside
  the page and steals `$scroll` from the sections. Write sections as usual and
  put the motion on `bindings`.
- `$scroll` may drive only PaintOnly targets: `translateX`, `translateY`,
  `opacity`, `fill`, `stroke`, `textColor`, `cornerRadius`, widget `value`,
  `checked`, `selectedValue`. Never bind it to `x`, `y`, `rotation`, `scaleX`,
  `scaleY`, `width`, `height`, `visible`, or `content` — the runtime drops them.
  `translateX/Y` are visual offsets on top of the solved layout (a CSS
  `transform: translate`): parallax, rise-in and progress bars all use them.
- Sticky / fixed nav: give the section's ROOT frame `pin: true` when it is a
  direct child of the page root, flush with the page top or bottom edge. Pin is
  unconditional viewport pinning, not threshold CSS sticky. No container, no
  `stickyChildren` on the page root, no CSS `position` vocabulary.
- An explicit scroller (recipe 6: a fixed-height clipped frame with a scroll
  handler) is ONLY for a scroll area inside an app screen. Inside it, `$scroll`
  means that container and `stickyChildren:["id"]` pins.

## DSL form

Generation output is `I(parent, {...})` — `bindings`, `pin` and `events` are
plain props of the second argument, values exactly as in the JSON recipes:

```js
I(sec, {type:"frame", name:"Hero BG", width:"fill_container", height:480, bindings:{translateY:"$scroll.offset * -0.3"}})
I(page, {type:"frame", name:"Nav", width:"fill_container", height:72, pin:true})
```

## Animate contract

- `animate` requires literal `target`, registered `property`, literal `to`, and
  positive integer `durationMs`. Optional: `from`, `delayMs` (default 0),
  `easing`, `iterations` (1..1000), `direction`, `fillMode`. Properties:
  `opacity`, `translateX`, `translateY`, `x`, `y`, `rotation`, `scaleX`,
  `scaleY`, `fill`, `stroke`, `cornerRadius`, `width`, `height` (width/height
  switch discretely at the end). Easings: `linear`, `ease`, `ease_in`,
  `ease_out`, `ease_in_out`. Directions: `normal`, `reverse`, `alternate`,
  `alternate_reverse`. Fill modes: `none`, `forwards`, `backwards`, `both`.
- One effect's animates go in one action list; they share a clock, so
  sequence/stagger uses increasing `delayMs` and delayed tracks need
  `backwards`/`both`. Animate is event-clocked, never a scroll scrub — scroll
  scrubbing is always a `bindings` expression.

## 1. Hero parallax + title fade

Background layer drifts slower than the page (negative factor), title fades
out over the first 320 px.

```json
{
  "type": "frame", "id": "hero", "width": 1440, "height": 720, "layout": "none",
  "fill": [{ "type": "solid", "color": "#0B1020" }],
  "children": [
    { "type": "frame", "id": "hero-bg", "x": 0, "y": 0, "width": 1440, "height": 720,
      "layout": "none", "bindings": { "translateY": "$scroll.offset * -0.3" },
      "children": [
        { "type": "ellipse", "id": "hero-glow", "x": 880, "y": 80, "width": 420, "height": 420,
          "fill": [{ "type": "solid", "color": "#7C3AED66" }] }
      ] },
    { "type": "text", "id": "hero-title", "x": 120, "y": 240, "width": 900, "height": 160,
      "content": "DEPTH IN MOTION", "fontSize": 96, "fontWeight": 700,
      "fill": [{ "type": "solid", "color": "#FFFFFF" }],
      "bindings": { "opacity": "clamp(1 - $scroll.offset / 320, 0, 1)",
                    "translateY": "$scroll.offset * 0.15" } }
  ]
}
```

## 2. Pinned header = nav row + scroll progress bar

ONE pinned block, first child of the page root, flush with the top. Pinned
things never get their own section: a progress bar or rail that is a standalone
section still takes flow space and pushes the page down. Put it inside the
header. Side rails are not supported — use this top bar. The nav surface
solidifies once the page has scrolled; the bar slides in with `translateX`
(never bind `width`, it is Relayout).

```json
{
  "type": "frame", "id": "header", "pin": true, "width": 1440, "height": 76,
  "layout": "vertical", "gap": 0,
  "fill": [{ "type": "solid", "color": "#0B102000" }],
  "bindings": { "fill": "$scroll.offset > 24 ? \"#0B1020E6\" : \"#0B102000\"" },
  "children": [
    { "type": "frame", "id": "nav", "width": 1440, "height": 72,
      "layout": "horizontal", "padding": [0, 48], "alignItems": "center",
      "justifyContent": "space_between",
      "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": "#33415500" }] },
      "bindings": { "stroke": "$scroll.offset > 24 ? \"#334155\" : \"#33415500\"" },
      "children": [
        { "type": "text", "id": "nav-brand", "width": "fit_content", "height": 32,
          "content": "NORTHSTAR", "fontSize": 20, "fontWeight": 700,
          "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
        { "type": "text", "id": "nav-pricing", "width": "fit_content", "height": 32,
          "content": "Pricing", "fontSize": 16, "fill": [{ "type": "solid", "color": "#FFFFFF" }],
          "events": { "onTap": [{ "scroll_to": { "target": "pricing", "alignment": "start" } }] } }
      ] },
    { "type": "frame", "id": "progress-rail", "width": 1440, "height": 4, "layout": "none",
      "children": [
        { "type": "rectangle", "id": "progress-fill", "x": 0, "y": 0, "width": 1440, "height": 4,
          "fill": [{ "type": "solid", "color": "#7C3AED" }],
          "bindings": { "translateX": "-(1 - $scroll.progress) * 1440" } }
      ] }
  ]
}
```

## 3. Staggered rise-in cards

Card k opens its window at `t = 0.18 + k * 0.06` of page progress: fades in
and rises 40 px. Same pattern for any list; keep windows ≥ 0.06 apart.

```json
{
  "type": "frame", "id": "features", "width": 1440, "height": 520, "layout": "horizontal",
  "padding": [80, 120], "gap": 32, "alignItems": "start",
  "children": [
    { "type": "rectangle", "id": "card-1", "width": 368, "height": 360, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#EDE9FE" }],
      "bindings": { "opacity": "clamp(($scroll.progress - 0.18) / 0.08, 0, 1)",
                    "translateY": "(1 - clamp(($scroll.progress - 0.18) / 0.08, 0, 1)) * 40" } },
    { "type": "rectangle", "id": "card-2", "width": 368, "height": 360, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#DBEAFE" }],
      "bindings": { "opacity": "clamp(($scroll.progress - 0.24) / 0.08, 0, 1)",
                    "translateY": "(1 - clamp(($scroll.progress - 0.24) / 0.08, 0, 1)) * 40" } },
    { "type": "rectangle", "id": "card-3", "width": 368, "height": 360, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#DCFCE7" }],
      "bindings": { "opacity": "clamp(($scroll.progress - 0.30) / 0.08, 0, 1)",
                    "translateY": "(1 - clamp(($scroll.progress - 0.30) / 0.08, 0, 1)) * 40" } }
  ]
}
```

## 4. Scrollytelling crossfade stage

A pinned stage (direct child of the page root, flush top) crossfades chapters
as the page scrolls beneath it.

```json
{
  "type": "frame", "id": "story-stage", "pin": true, "width": 1440, "height": 720,
  "layout": "none", "fill": [{ "type": "solid", "color": "#09090B" }],
  "children": [
    { "type": "text", "id": "chapter-a", "x": 120, "y": 280, "width": 1200, "height": 140,
      "content": "DISCOVER", "fontSize": 88, "fill": [{ "type": "solid", "color": "#FFFFFF" }],
      "bindings": { "opacity": "clamp(1 - $scroll.progress * 3, 0, 1)" } },
    { "type": "text", "id": "chapter-b", "x": 120, "y": 280, "width": 1200, "height": 140,
      "content": "COMPOSE", "fontSize": 88, "fill": [{ "type": "solid", "color": "#C4B5FD" }],
      "bindings": { "opacity": "clamp(1 - abs($scroll.progress - 0.5) * 4, 0, 1)" } },
    { "type": "text", "id": "chapter-c", "x": 120, "y": 280, "width": 1200, "height": 140,
      "content": "SHIP", "fontSize": 88, "fill": [{ "type": "solid", "color": "#67E8F9" }],
      "bindings": { "opacity": "clamp(($scroll.progress - 0.66) * 3, 0, 1)" } }
  ]
}
```

## 5. Tap-triggered animate stagger

Time-based entrance lives on an event, never on scroll. Three cards rise and
fade in with increasing `delayMs`.

```json
{
  "type": "frame", "id": "entry", "width": 1440, "height": 480, "layout": "horizontal",
  "padding": [60, 120], "gap": 32,
  "events": { "onTap": [
    { "animate": { "target": "entry-card-1", "property": "translateY", "from": 40, "to": 0, "durationMs": 360, "delayMs": 0, "easing": "ease_out", "iterations": 1, "direction": "normal", "fillMode": "both" } },
    { "animate": { "target": "entry-card-2", "property": "opacity", "from": 0, "to": 1, "durationMs": 360, "delayMs": 90, "easing": "ease_out", "iterations": 1, "direction": "normal", "fillMode": "both" } },
    { "animate": { "target": "entry-card-3", "property": "opacity", "from": 0, "to": 1, "durationMs": 360, "delayMs": 180, "easing": "ease_out", "iterations": 1, "direction": "normal", "fillMode": "both" } }
  ] },
  "children": [
    { "type": "rectangle", "id": "entry-card-1", "width": 368, "height": 300, "cornerRadius": 24,
      "fill": [{ "type": "solid", "color": "#DDD6FE" }] },
    { "type": "rectangle", "id": "entry-card-2", "width": 368, "height": 300, "opacity": 0,
      "cornerRadius": 24, "fill": [{ "type": "solid", "color": "#BFDBFE" }] },
    { "type": "rectangle", "id": "entry-card-3", "width": 368, "height": 300, "opacity": 0,
      "cornerRadius": 24, "fill": [{ "type": "solid", "color": "#A7F3D0" }] }
  ]
}
```

## 6. Explicit scroller — APP SCREENS ONLY

The one place a scroll handler belongs: a fixed-height list inside an app
screen. Landing pages never use this shape.

```json
{
  "type": "frame", "id": "feed-scroll", "width": 390, "height": 600,
  "layout": "none", "clipContent": true,
  "state": { "seen": { "type": "bool", "default": false } },
  "stickyChildren": ["feed-progress"],
  "events": { "onScroll": [{ "set": { "$state.seen": "true" } }] },
  "children": [
    { "type": "rectangle", "id": "feed-content", "x": 0, "y": 0, "width": 390, "height": 1800,
      "fill": [{ "type": "solid", "color": "#F8FAFC" }] },
    { "type": "progress", "id": "feed-progress", "pin": true, "x": 16, "y": 12,
      "width": 358, "height": 6, "max": 100, "value": 0, "cornerRadius": 3,
      "fill": [{ "type": "solid", "color": "#7C3AED" }],
      "bindings": { "value": "$scroll.progress * 100" } }
  ]
}
```

Do not invent scroll-bound layout scrub, velocity, element-enter observers,
springs, keyframes, image-frame selection, or dynamic text. Horizontal /
infinite galleries need missing append/x-axis primitives and are not
approximated here.

END SCROLL ORCHESTRATION
