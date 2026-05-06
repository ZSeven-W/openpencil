---
name: codegen-html
description: HTML + CSS code generation rules — semantic HTML5 with CSS classes in style block
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---

# HTML + CSS 代码生成

生成语义化 HTML5 markup，并在 `<style>` block 中定义 CSS classes。不依赖 build tools，也不依赖 framework。

## 输出格式

- HTML5 (`.html`)
- 使用语义化 HTML elements
- 所有样式通过 `<style>` block 中的 CSS classes 实现
- 使用 CSS custom properties 表示 design variables
- 不使用 inline styles、不使用 framework、不依赖 build tools
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
- `textAlignVertical: "middle"` → `vertical-align: middle`
- `textGrowth: "auto"` → `white-space: nowrap`
- `textGrowth: "fixed-width-height"` → `overflow: hidden`
- `underline` → `text-decoration: underline`
- `strikethrough` → `text-decoration: line-through`

## dimensions

- Fixed → `width: Npx; height: Npx`
- `fill_container` → `width: 100%` or `height: 100%`
- Root container → `max-width: Npx; width: 100%; margin: 0 auto`，用于响应式居中

## image 处理

- `<img class="className" src="src" alt="name" />`
- `object-fit: contain|cover|fill` based on `objectFit` property:
  - `objectFit: "fit"` → `object-fit: contain`
  - `objectFit: "crop"` → `object-fit: cover`
  - default → `object-fit: fill`
- corner radius 通过 CSS class 应用

## opacity 与 transform

- `opacity: N` → `opacity: N`
- `rotation: N` → `transform: rotate(Ndeg)`

## positioning

- Absolute children → `position: absolute; left: Xpx; top: Ypx`
- Container → `position: relative`

## 语义化 HTML 标签

- Font size >= 32 → `<h1>`
- Font size >= 24 → `<h2>`
- Font size >= 20 → `<h3>`
- Other text → `<p>`
- Lines → `<hr>`
- 适当使用 `<nav>`、`<header>`、`<main>`、`<section>`、`<footer>`、`<article>`

## icon 处理

- Icon font nodes → `<i class="className" data-lucide="icon-name"></i>`
- 通过 CSS class 设置 `width`、`height` 和 `color`
- 引入 Lucide CDN script 以渲染 icon

## SVG elements

- Path nodes → inline `<svg>` with `<path d="..." fill="color" />`
- 在 SVG element 上设置 `viewBox`、`width`、`height`

## variable references

- `$variable` refs → `var(--variable-name)` CSS custom properties
- 在 `:root { --name: value; }` block 中定义 variables
- Background: `background: var(--name)`
- Text color: `color: var(--name)`
- Border: `border-color: var(--name)`

## 响应式设计

- Fluid containers 使用 `max-width` 搭配 `width: 100%`
- 在常见断点使用 media queries：`@media (min-width: 640px)`、`768px`、`1024px`、`1280px`
- 适当使用相对单位（`em`、`rem`、`%`）
