---
name: component-composition
description: Component instantiation patterns for documents with reusable components
phase: [generation]
trigger:
  flags: [hasReusableComponents]
priority: 20
budget: 1000
category: domain
---

COMPONENT COMPOSITION (use when document has reusable components):

## Priority Rule

Always use ref instantiation over creating from scratch. Existing components ensure visual consistency.

## Slot System

Frames with a `slot` property contain recommended child component IDs:

- Insert recommended components: I(parentSlotPath, {type: "ref", ref: "recommendedId"})
- Disable unused slots: U(instance+"/slotId", {enabled: false})

## Descendant Overrides

Modify instance content WITHOUT recreating the component:

- Change properties: U(instance+"/childId", {content: "New Text"})
- Replace a node: R(instance+"/slotId", {type: "frame", layout: "vertical", ...})
- Nested instances use / path: instance+"/nestedRef/childId"

## Common Composition Patterns

Sidebar + Content = Dashboard:
layout: horizontal, sidebar width 240-280px, content fill_container

Header + Content = Standard Page:
layout: vertical, header height 64px, content fill_container

Card (3-slot architecture):
Card Header (slot) — title, description
Card Content (slot) — main content, form fields
Card Actions (slot) — buttons, links

Dialog = Card ref + custom header/actions:
descendants: {"headerSlot": {children: [Title, Description]}, "contentSlot": {enabled: false}}

Modal = Card ref + shadow effect:
effect: [{type: "shadow", blur: 20, ...}]

Table hierarchy:
Table → Row (slot) → Cell (frame) → Content
NEVER skip the Cell frame layer

Tabs:
Tabs container (slot) → Tab Item Active / Tab Item Inactive

## Copy Warnings

- Copy creates NEW descendant IDs — do NOT Update a copied node's descendants by old ID
- Use the `descendants` property in the Copy operation itself to override content
- For post-copy modifications, read the new node to get updated IDs first
