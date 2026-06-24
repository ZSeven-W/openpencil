---
name: jian-components
description: Interactive widget family — emit role-marked frames the promotion pass collapses into real text_input / switch / select / checkbox / slider / text_area nodes
phase: [generation]
trigger: null
priority: 5
budget: 1200
category: base
---

INTERACTIVE WIDGETS (jian component family):

When a design has a form field, toggle, dropdown, slider, or multi-line
text box, emit a `frame` and set its `role` to one of the markers below.
A promotion pass collapses each marked frame into a real widget node — so
the output `.op` carries a true `text_input` / `switch` / … node, not a
mockup frame. ONLY these exact `role` strings are honoured; any other
value stays a plain frame.

ROLE → WIDGET (the only honoured markers):

- `role: "input"` or `role: "form-input"` → text_input (single-line field)
- `role: "textarea"` or `role: "text-area"` → text_area (multi-line)
- `role: "select"` or `role: "dropdown"` → select (option picker)
- `role: "switch"` or `role: "toggle"` → switch (on/off)
- `role: "checkbox"` → checkbox (label + box)
- `role: "slider"` → slider (range)

(Alternatively `semantics: { role: "input" }` also promotes to text_input.)

CHILD STRUCTURE the promotion reads (put these INSIDE the marked frame):

- Placeholder text: a `text` child whose fill is a MUTED grey
  (e.g. `#9CA3AF`). The first muted text becomes the widget `placeholder`.
- Value text: a `text` child with any non-muted fill becomes the `value`
  (for checkbox it becomes the `label`).
- Leading / trailing icons: `icon_font` children. The FIRST `icon_font` is
  the leading icon (e.g. `mail`), a SECOND is the trailing icon (e.g. an
  `eye` password reveal). They are carried onto the promoted text_input.
- Style (fill / stroke / cornerRadius / effects) and width/height on the
  frame are carried verbatim onto the widget. Other children are dropped —
  widgets are leaves.

EXAMPLE (an email field that becomes a text_input):

{"type":"frame","id":"emailField","role":"input","width":320,"height":48,
 "cornerRadius":12,"fill":[{"type":"solid","color":"#F3F4F6"}],"children":[
   {"type":"icon_font","id":"mailIcon","iconFontName":"mail","width":20,"height":20},
   {"type":"text","id":"ph","content":"you@example.com","fill":[{"type":"solid","color":"#9CA3AF"}]}
 ]}

A `role: "switch"` frame needs no children; a `role: "checkbox"` frame
takes one non-muted `text` child as its label.
