---
name: codegen-vue
description: Vue 3 SFC code generation rules — single file component with scoped CSS
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---

# Vue 3 Single File Component Code Generation

Generate Vue 3 Single File Components with `<script setup>`, `<template>`, and `<style scoped>`.

## Output Format

- Vue 3 SFC (`.vue`)
- `<script setup lang="ts">` for component logic
- `<template>` with semantic HTML markup
- `<style scoped>` with CSS classes (no Tailwind, no inline styles)
- Each node gets a unique, descriptive CSS class name derived from `node.name`

## Layout Mapping

- `layout: "vertical"` → `display: flex; flex-direction: column`
- `layout: "horizontal"` → `display: flex; flex-direction: row`
- `gap: N` → `gap: Npx`
- `padding: N` → `padding: Npx`
- `padding: [t, r, b, l]` → `padding: Tpx Rpx Bpx Lpx`
- `justifyContent: "start"` → `justify-content: flex-start`
- `justifyContent: "center"` → `justify-content: center`
- `justifyContent: "end"` → `justify-content: flex-end`
- `justifyContent: "space_between"` → `justify-content: space-between`
- `justifyContent: "space_around"` → `justify-content: space-around`
- `alignItems: "start"` → `align-items: flex-start`
- `alignItems: "center"` → `align-items: center`
- `alignItems: "end"` → `align-items: flex-end`
- `clipContent: true` → `overflow: hidden`

## Color & Fill Mapping

- Solid fill `#hex` → `background: #hex`
- Variable ref `$name` → `background: var(--name)`
- Text fill → `color: #hex` or `color: var(--name)`
- Linear gradient → `background: linear-gradient(Ndeg, color1 0%, color2 100%)`
- Radial gradient → `background: radial-gradient(circle, color1 0%, color2 100%)`

## Border & Stroke Mapping

- `stroke.thickness` → `border-width: Npx; border-style: solid`
- `stroke.color` → `border-color: #hex`
- Variable ref → `border-width: var(--name)`, `border-color: var(--name)`

## Corner Radius

- Uniform → `border-radius: Npx`
- Per-corner `[tl, tr, br, bl]` → `border-radius: TLpx TRpx BRpx BLpx`
- Ellipse → `border-radius: 50%`

## Effects

- Drop shadow → `box-shadow: offsetXpx offsetYpx blurpx spreadpx color`
- Inner shadow → `box-shadow: inset offsetXpx offsetYpx blurpx spreadpx color`
- Multiple shadows comma-separated

## Typography

- `fontSize` → `font-size: Npx`
- `fontWeight` → `font-weight: N`
- `fontStyle: "italic"` → `font-style: italic`
- `fontFamily` → `font-family: 'Name', sans-serif`
- `lineHeight` → `line-height: value`
- `letterSpacing` → `letter-spacing: Npx`
- `textAlign` → `text-align: left|center|right`
- `underline` → `text-decoration: underline`
- `strikethrough` → `text-decoration: line-through`

## Dimensions

- Fixed → `width: Npx; height: Npx`
- `fill_container` → `width: 100%` or `height: 100%`

## Image Handling

- `<img class="className" :src="src" :alt="name" />`
- `object-fit: contain|cover|fill` based on `objectFit` property
- Corner radius applied via CSS class

## Opacity & Transform

- `opacity: N` → `opacity: N`
- `rotation: N` → `transform: rotate(Ndeg)`

## Positioning

- Absolute children → `position: absolute; left: Xpx; top: Ypx`

## Semantic HTML Tags

- Font size >= 32 → `<h1>`
- Font size >= 24 → `<h2>`
- Font size >= 20 → `<h3>`
- Other text → `<p>`
- Lines → `<hr>`
- Use semantic elements (`<nav>`, `<header>`, `<main>`, `<section>`, `<footer>`)

## Icon Handling

- Icon font nodes → `<i class="className" data-lucide="icon-name" />`
- Set `width`, `height`, and `color` via CSS class

## Vue-Specific Patterns

- Use `v-for` for repeated items: `<div v-for="item in items" :key="item.id">`
- Use `v-if` / `v-else` for conditional rendering
- Use `:class` binding for dynamic classes
- Use `:style` binding sparingly (prefer CSS classes)
- Props defined with `defineProps<{ ... }>()`
- Emits defined with `defineEmits<{ ... }>()`

## Variable References

- `$variable` refs → `var(--variable-name)` in CSS
- Background: `background: var(--name)`
- Text color: `color: var(--name)`
- Border: `border-color: var(--name)`
- Define CSS custom properties in `:root` or scoped style block
