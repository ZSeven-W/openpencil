---
name: codegen-react
description: React + Tailwind CSS code generation rules — TSX output with Tailwind utility classes
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---

# React + Tailwind 代码生成

生成使用 Tailwind CSS utility classes 的 React TSX 组件。

## 输出格式

- TypeScript TSX (`.tsx`)
- 使用 `export function ComponentName()` 的函数组件
- 所有样式使用 Tailwind CSS（不使用 inline styles，不使用 CSS modules）

## layout 映射

- `layout: "vertical"` → `flex flex-col`
- `layout: "horizontal"` → `flex flex-row`
- `gap: N` → `gap-[Npx]`
- `padding` → `p-[Npx]`；按边设置时使用 `pt-[N] pr-[N] pb-[N] pl-[N]`
- `padding: [vertical, horizontal]` → `py-[Vpx] px-[Hpx]`
- `justifyContent` → `justify-{start|center|end|between|around}`
- `alignItems` → `items-{start|center|end|stretch}`
- `clipContent: true` → `overflow-hidden`

## color 与 fill 映射

- Solid fill `#hex` → `bg-[#hex]`
- Variable ref `$name` → `bg-[var(--name)]`
- Text fill → `text-[#hex]` or `text-[var(--name)]`
- Gradient fills → 使用 `bg-gradient-to-{direction}` 搭配 `from-[color] to-[color]`

## border 与 stroke 映射

- `stroke.thickness` → `border-[Npx]`
- `stroke.color` → `border-[#hex]`
- Variable ref → `border-[var(--name)]`

## cornerRadius

- Uniform → `rounded-[Npx]`
- Per-corner `[tl, tr, br, bl]` → `rounded-[tl_tr_br_bl]`（Tailwind arbitrary values）
- Ellipse → `rounded-full`

## effects

- Drop shadow → `shadow-[offsetXpx_offsetYpx_blurpx_spreadpx_color]`
- Inner shadow → 使用 `shadow-inner` variant
- Blur → `blur-[Npx]`

## typography

- `fontSize` → `text-[Npx]`
- `fontWeight`（数字）→ `font-[weight]`
- `fontStyle: "italic"` → `italic`
- `fontFamily` → `font-['Family_Name']`（空格替换为下划线）
- `lineHeight` → `leading-[value]`
- `letterSpacing` → `tracking-[Npx]`
- `textAlign` → `text-{left|center|right|justify}`
- `textAlignVertical: "middle"` → `align-middle`
- `textGrowth: "auto"` → `whitespace-nowrap`
- `textGrowth: "fixed-width-height"` → `overflow-hidden`
- `underline` → `underline`
- `strikethrough` → `line-through`

## dimensions

- Fixed → `w-[Npx] h-[Npx]`
- `fill_container` width → `w-full`
- `fill_container` height → `h-full`
- Root component → `max-w-[Npx] w-full mx-auto`，用于响应式居中

## image 处理

- `<img src={src} alt={name} className="w-[N] h-[N] object-{fit}" />`
- `objectFit: "fit"` → `object-contain`
- `objectFit: "crop"` → `object-cover`
- `objectFit: "fill"` → `object-fill`
- 图片上的 corner radius → 添加 `rounded-[Npx]`

## opacity 与 transform

- `opacity: N` → `opacity-[N%]`（乘以 100）
- Variable ref opacity → `opacity-[var(--name)]`
- `rotation: N` → `rotate-[Ndeg]`

## positioning

- Absolute children → `absolute left-[Xpx] top-[Ypx]`

## 语义化 HTML 标签

- Font size >= 32 → `<h1>`
- Font size >= 24 → `<h2>`
- Font size >= 20 → `<h3>`
- Other text → `<p>`
- Lines → `<hr>`
- 适当使用 `<nav>`、`<header>`、`<main>`、`<section>`、`<footer>`、`<article>`
- 当 role 表示可交互元素时，使用 `<button>`、`<a>`、`<input>`

## icon 处理

- Icon font nodes → `<IconName size={N} color="color" />`（kebab-to-PascalCase）

## 响应式设计

- Mobile-first：基础样式面向移动端，`md:` 面向平板，`lg:` 面向桌面端
- 将固定宽度转换为 `max-w-*` 搭配 `w-full`
- 窄视口中的 card grid 使用 `flex-wrap`

## variable references

- `$variable` 引用输出为 `var(--variable-name)` CSS custom properties
- Background: `bg-[var(--name)]`
- Text color: `text-[var(--name)]`
- Border: `border-[var(--name)]`
- Gap/padding with variable: `gap-[var(--name)]`, `p-[var(--name)]`
