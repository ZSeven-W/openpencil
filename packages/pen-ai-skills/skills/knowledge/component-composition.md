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

COMPONENT COMPOSITION（当 document 有 reusable components 时使用）：

## Priority Rule

始终优先使用 ref instantiation，而不是从零创建。Existing components 能确保 visual consistency。

## Slot System

带有 `slot` property 的 frames 包含 recommended child component IDs：

- 插入 recommended components：I(parentSlotPath, {type: "ref", ref: "recommendedId"})
- 禁用 unused slots：U(instance+"/slotId", {enabled: false})

## Descendant Overrides

在不重建 component 的前提下修改 instance content：

- 修改 properties：U(instance+"/childId", {content: "New Text"})
- 替换 node：R(instance+"/slotId", {type: "frame", layout: "vertical", ...})
- Nested instances 使用 / path：instance+"/nestedRef/childId"

## Common Composition Patterns

Sidebar + Content = Dashboard：
layout: horizontal, sidebar width 240-280px, content fill_container

Header + Content = Standard Page：
layout: vertical, header height 64px, content fill_container

Card（3-slot architecture）：
Card Header (slot) — title, description
Card Content (slot) — main content, form fields
Card Actions (slot) — buttons, links

Dialog = Card ref + custom header/actions：
descendants: {"headerSlot": {children: [Title, Description]}, "contentSlot": {enabled: false}}

Modal = Card ref + shadow effect：
effect: [{type: "shadow", blur: 20, ...}]

Table hierarchy：
Table → Row (slot) → Cell (frame) → Content
绝不要跳过 Cell frame layer

Tabs：
Tabs container (slot) → Tab Item Active / Tab Item Inactive

## Copy Warnings

- Copy 会创建新的 descendant IDs — 不要用旧 ID Update copied node 的 descendants
- 使用 Copy operation 本身的 `descendants` property 来 override content
- 对 post-copy modifications，请先读取 new node 获取更新后的 IDs
