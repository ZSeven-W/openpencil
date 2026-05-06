---
name: codegen-svelte
description: Svelte component code generation rules — markup with scoped styles
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---

# Svelte Component 代码生成

生成包含 markup 和 scoped `<style>` blocks 的 Svelte components。

## 输出格式

- Svelte component (`.svelte`)
- `<script lang="ts">` 用于组件逻辑（可选，仅在需要 props/logic 时使用）
- HTML template markup
- `<style>` block 中使用 scoped CSS classes
- 每个节点都需要一个由 `node.name` 派生的唯一且描述性的 CSS class name

## layout 映射

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

## color 与 fill 映射

- Solid fill `#hex` → `background: #hex`
- Variable ref `$name` → `background: var(--name)`
- Text fill → `color: #hex` or `color: var(--name)`
- Linear gradient → `background: linear-gradient(Ndeg, color1 0%, color2 100%)`
- Radial gradient → `background: radial-gradient(circle, color1 0%, color2 100%)`

## border 与 stroke 映射

- `stroke.thickness` → `border-width: Npx; border-style: solid`
- `stroke.color` → `border-color: #hex`
- Variable ref → `border-width: var(--name)`, `border-color: var(--name)`

## cornerRadius

- Uniform → `border-radius: Npx`
- Per-corner `[tl, tr, br, bl]` → `border-radius: TLpx TRpx BRpx BLpx`
- Ellipse → `border-radius: 50%`

## effects

- Drop shadow → `box-shadow: offsetXpx offsetYpx blurpx spreadpx color`
- Inner shadow → `box-shadow: inset offsetXpx offsetYpx blurpx spreadpx color`
- Multiple shadows comma-separated

## typography

- `fontSize` → `font-size: Npx`
- `fontWeight` → `font-weight: N`
- `fontStyle: "italic"` → `font-style: italic`
- `fontFamily` → `font-family: 'Name', sans-serif`
- `lineHeight` → `line-height: value`
- `letterSpacing` → `letter-spacing: Npx`
- `textAlign` → `text-align: left|center|right`
- `underline` → `text-decoration: underline`
- `strikethrough` → `text-decoration: line-through`

## dimensions

- Fixed → `width: Npx; height: Npx`
- `fill_container` → `width: 100%` or `height: 100%`

## image 处理

- `<img class="className" src={src} alt={name} />`
- 根据 `objectFit` property 使用 `object-fit: contain|cover|fill`
- corner radius 通过 CSS class 应用

## opacity 与 transform

- `opacity: N` → `opacity: N`
- `rotation: N` → `transform: rotate(Ndeg)`

## positioning

- Absolute children → `position: absolute; left: Xpx; top: Ypx`

## 语义化 HTML 标签

- Font size >= 32 → `<h1>`
- Font size >= 24 → `<h2>`
- Font size >= 20 → `<h3>`
- Other text → `<p>`
- Lines → `<hr>`
- Use semantic elements (`<nav>`, `<header>`, `<main>`, `<section>`, `<footer>`)

## icon 处理

- Icon font nodes → `<i class="className" data-lucide="icon-name" />`
- 通过 CSS class 设置 `width`、`height` 和 `color`

## Svelte 专用模式

- 列表使用 `{#each items as item}`
- 条件渲染使用 `{#if condition}` / `{:else}`
- 条件 class 使用 `class:active={isActive}` directive
- Props 使用 `export let propName: Type` 声明
- 事件转发使用 `on:click`
- Slot 内容使用 `<slot />` 和 named slots `<slot name="header" />`
- 响应式声明使用 `$:` label
- Styles 会自动 scoped 到当前 component

## variable references

- `$variable` refs → `var(--variable-name)` in CSS
- Background: `background: var(--name)`
- Text color: `color: var(--name)`
- Border: `border-color: var(--name)`
- Define CSS custom properties in `:root` or `:global` style block
