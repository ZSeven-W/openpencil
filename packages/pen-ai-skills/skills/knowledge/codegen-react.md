---
name: codegen-react
description: React + Tailwind CSS code generation rules — TSX output with Tailwind utility classes
phase: [generation]
trigger:
  keywords: [react, tsx, tailwind]
priority: 20
budget: 2000
category: knowledge
---

# React + Tailwind Code Generation

Generate React TSX components using Tailwind CSS utility classes.

## Output Format
- TypeScript TSX (`.tsx`)
- Functional components with `export function ComponentName()`
- Tailwind CSS for all styling (no inline styles, no CSS modules)

## Layout Mapping
- `layout: "vertical"` → `flex flex-col`
- `layout: "horizontal"` → `flex flex-row`
- `gap: N` → `gap-[Npx]`
- `padding` → `p-[Npx]` or `pt-[N] pr-[N] pb-[N] pl-[N]` for per-side
- `padding: [vertical, horizontal]` → `py-[Vpx] px-[Hpx]`
- `justifyContent` → `justify-{start|center|end|between|around}`
- `alignItems` → `items-{start|center|end|stretch}`
- `clipContent: true` → `overflow-hidden`

## Color & Fill Mapping
- Solid fill `#hex` → `bg-[#hex]`
- Variable ref `$name` → `bg-[var(--name)]`
- Text fill → `text-[#hex]` or `text-[var(--name)]`
- Gradient fills → `bg-gradient-to-{direction}` with `from-[color] to-[color]`

## Border & Stroke Mapping
- `stroke.thickness` → `border-[Npx]`
- `stroke.color` → `border-[#hex]`
- Variable ref → `border-[var(--name)]`

## Corner Radius
- Uniform → `rounded-[Npx]`
- Per-corner `[tl, tr, br, bl]` → `rounded-[tl_tr_br_bl]` (Tailwind arbitrary values)
- Ellipse → `rounded-full`

## Effects
- Drop shadow → `shadow-[offsetXpx_offsetYpx_blurpx_spreadpx_color]`
- Inner shadow → use `shadow-inner` variant
- Blur → `blur-[Npx]`

## Typography
- `fontSize` → `text-[Npx]`
- `fontWeight` (numeric) → `font-[weight]`
- `fontStyle: "italic"` → `italic`
- `fontFamily` → `font-['Family_Name']` (spaces replaced with underscores)
- `lineHeight` → `leading-[value]`
- `letterSpacing` → `tracking-[Npx]`
- `textAlign` → `text-{left|center|right|justify}`
- `textAlignVertical: "middle"` → `align-middle`
- `textGrowth: "auto"` → `whitespace-nowrap`
- `textGrowth: "fixed-width-height"` → `overflow-hidden`
- `underline` → `underline`
- `strikethrough` → `line-through`

## Dimensions
- Fixed → `w-[Npx] h-[Npx]`
- `fill_container` width → `w-full`
- `fill_container` height → `h-full`
- Root component → `max-w-[Npx] w-full mx-auto` for responsive centering

## Image Handling
- `<img src={src} alt={name} className="w-[N] h-[N] object-{fit}" />`
- `objectFit: "fit"` → `object-contain`
- `objectFit: "crop"` → `object-cover`
- `objectFit: "fill"` → `object-fill`
- Corner radius on images → add `rounded-[Npx]`

## Opacity & Transform
- `opacity: N` → `opacity-[N%]` (multiply by 100)
- Variable ref opacity → `opacity-[var(--name)]`
- `rotation: N` → `rotate-[Ndeg]`

## Positioning
- Absolute children → `absolute left-[Xpx] top-[Ypx]`

## Semantic HTML Tags
- Font size >= 32 → `<h1>`
- Font size >= 24 → `<h2>`
- Font size >= 20 → `<h3>`
- Other text → `<p>`
- Lines → `<hr>`
- Use `<nav>`, `<header>`, `<main>`, `<section>`, `<footer>`, `<article>` appropriately
- Interactive elements: `<button>`, `<a>`, `<input>` where role suggests

## Icon Handling
- Icon font nodes → `<IconName size={N} color="color" />` (kebab-to-PascalCase)

## Responsive Design
- Mobile-first: base styles for mobile, `md:` for tablet, `lg:` for desktop
- Convert fixed widths to `max-w-*` with `w-full`
- Use `flex-wrap` for card grids on narrow viewports

## Variable References
- `$variable` refs are output as `var(--variable-name)` CSS custom properties
- Background: `bg-[var(--name)]`
- Text color: `text-[var(--name)]`
- Border: `border-[var(--name)]`
- Gap/padding with variable: `gap-[var(--name)]`, `p-[var(--name)]`
