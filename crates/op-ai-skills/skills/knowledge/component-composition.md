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

## The `ref` Node — script-gen (your default output protocol)

script-gen is THE output protocol on the full generation attempt: you write a
JavaScript program that builds the document by calling the global
`I(parent, node)`. A component instance under script-gen is ONE `I(...)` call
— `type:"ref"` plus the component id from the AVAILABLE COMPONENTS list. It
needs no children of its own; the system expands the master subtree at render
time, and the call returns the new instance's id like any other `I(...)` call:

```js
const cta = I(sec, {"type":"ref","ref":"shadcn-btn-primary"});
```

### Overriding content — `descendants`

To customize an instance's text / fill / sizing WITHOUT rebuilding it, add a
`descendants` map: descendant-id → partial node with only the fields you
change. Descendant ids are the ids of nodes INSIDE the master (e.g. the label
inside a button) — copy them from the master's own subtree, never invent one.
Override only what differs from the master:

```js
const cta = I(sec, {"type":"ref","ref":"shadcn-btn-primary","descendants":{
  "shadcn-btn-primary-label":{"content":"Get started"}
}});
```

Multiple instances of the same component, each with its own copy — call
`I(...)` once per instance (or loop, same as any other repeated structure):

```js
const row = I(sec, {"type":"frame","layout":"horizontal","width":"fill_container","gap":12});
const btnA = I(row, {"type":"ref","ref":"shadcn-btn-primary","descendants":{"shadcn-btn-primary-label":{"content":"Save"}}});
const btnB = I(row, {"type":"ref","ref":"shadcn-btn-secondary","descendants":{"shadcn-btn-secondary-label":{"content":"Cancel"}}});
```

## Rules

- The `ref` value MUST be a component id from the AVAILABLE COMPONENTS list.
  Never invent an id; if nothing matches, build the element normally.
- Use script-gen's `I(parent, {"type":"ref",...})` form on every subagent
  generation attempt. Reduced-complexity and minimal-skills retries narrow the
  skill set only; they do not switch output protocol.
- A `ref` node does NOT take its own visual props (fill / cornerRadius /
  padding) — those live on the master. Customize only via `descendants`.
- Keys in `descendants` MUST be ids that exist inside the chosen master.
- Repeating UI (button rows, card grids, list rows, nav items) is the
  strongest signal to reuse one component as several `ref` instances with
  per-instance `descendants`, rather than authoring each copy by hand.
