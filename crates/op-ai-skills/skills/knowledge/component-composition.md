---
name: component-composition
description: Instantiate available reusable components with ref + descendants instead of rebuilding from scratch
phase: [generation]
trigger:
  flags: [hasReusableComponents]
priority: 20
budget: 1200
category: domain
---

COMPONENT COMPOSITION (this document ships reusable components — prefer instances):

## Priority Rule

When a needed UI element (button, card, input, nav item, badge, avatar, etc.)
matches one of the AVAILABLE COMPONENTS listed in your prompt, INSTANTIATE it
with a `ref` node instead of hand-building the same structure out of
frame/text/icon_font. Instances inherit the component's exact spacing, radius,
fill, and typography, so the design stays visually consistent and on-brand.
Only build from scratch when no available component fits.

## The `ref` Node — match your output protocol

A component instance is a single node that points at the component's id (the id
shown in the AVAILABLE COMPONENTS list). HOW you spell it depends on which
output format you are producing — use the one that matches the rest of your
output, never mix them on one line:

**Raw PenNode / flat `_parent` JSONL** — set `type:"ref"`. A ref is ONE line
like any other node: give it an `id`, set its `_parent` to the container it
belongs in, and `ref:"<componentId>"`. It needs no `children` of its own; the
system expands the master subtree at render time.

```json
{"_parent":"sec-root","id":"sec-cta","type":"ref","ref":"shadcn-btn-primary"}
```

**Element-manifest (`el` lines)** — set `el:"ref"` (NOT `type:"ref"`). A ref is
ONE manifest line like any other element: write `ref:"<componentId>"`, and nest
it under a section with `in:<line>` exactly like a `stat_card` or `button`
element. Do NOT add `id` / `_parent` (the manifest forbids ids — the system
assigns them). It needs no `children`; the master subtree expands at render.

```json
{"el":"section","role":"cta"}
{"el":"ref","in":1,"ref":"shadcn-btn-primary"}
```

## Overriding Content — `descendants`

To customize an instance's text / fill / sizing WITHOUT rebuilding it, add a
`descendants` map: descendant-id → partial node with only the fields you change.
Descendant ids are the ids of nodes INSIDE the master (e.g. the label inside a
button). Override only what differs from the master. `descendants` works
identically in both protocols — only the `type:"ref"` vs `el:"ref"` envelope
differs:

```json
{"type":"ref","ref":"shadcn-btn-primary","descendants":{
  "shadcn-btn-primary-label":{"content":"Get started"}
}}
```

```json
{"el":"ref","in":1,"ref":"shadcn-btn-primary","descendants":{
  "shadcn-btn-primary-label":{"content":"Get started"}
}}
```

Multiple instances of the same component, each with its own copy (raw protocol):

```json
{"_parent":"row","id":"btn-a","type":"ref","ref":"shadcn-btn-primary","descendants":{"shadcn-btn-primary-label":{"content":"Save"}}}
{"_parent":"row","id":"btn-b","type":"ref","ref":"shadcn-btn-secondary","descendants":{"shadcn-btn-secondary-label":{"content":"Cancel"}}}
```

Same, in the element-manifest protocol (nest under a section row via `in`):

```json
{"el":"section","direction":"horizontal","role":"actions"}
{"el":"ref","in":1,"ref":"shadcn-btn-primary","descendants":{"shadcn-btn-primary-label":{"content":"Save"}}}
{"el":"ref","in":1,"ref":"shadcn-btn-secondary","descendants":{"shadcn-btn-secondary-label":{"content":"Cancel"}}}
```

## Rules

- The `ref` value MUST be a component id from the AVAILABLE COMPONENTS list.
  Never invent an id; if nothing matches, build the element normally.
- Use the `ref` envelope that matches your output: `el:"ref"` on `el`-line
  (manifest) output, `type:"ref"` on raw PenNode output. Never write `type:"ref"`
  on an `el` line — it will be misread as a raw escape-hatch node.
- A `ref` node does NOT take its own visual props (fill / cornerRadius / padding)
  — those live on the master. Customize only via `descendants`.
- Keys in `descendants` MUST be ids that exist inside the chosen master.
- Repeating UI (button rows, card grids, list rows, nav items) is the strongest
  signal to reuse one component as several `ref` instances with per-instance
  `descendants`, rather than authoring each copy by hand.
