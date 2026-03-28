---
name: codegen-html
description: HTML + CSS code generation rules — semantic HTML5 with CSS classes in style block
phase: [generation]
trigger:
  keywords: [html, css, vanilla, static]
priority: 20
budget: 2000
category: knowledge
---

# HTML + CSS Code Generation

Generate semantic HTML5 markup with CSS classes defined in a `<style>` block. No build tools, no framework dependencies.

## Output Format
- HTML5 (`.html`)
- Semantic HTML elements
- All styling via CSS classes in a `<style>` block
- CSS custom properties for design variables
- No inline styles, no framework, no build tools
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
- `textAlignVertical: "middle"` → `vertical-align: middle`
- `textGrowth: "auto"` → `white-space: nowrap`
- `textGrowth: "fixed-width-height"` → `overflow: hidden`
- `underline` → `text-decoration: underline`
- `strikethrough` → `text-decoration: line-through`

## Dimensions
- Fixed → `width: Npx; height: Npx`
- `fill_container` → `width: 100%` or `height: 100%`
- Root container → `max-width: Npx; width: 100%; margin: 0 auto` for responsive centering

## Image Handling
- `<img class="className" src="src" alt="name" />`
- `object-fit: contain|cover|fill` based on `objectFit` property:
  - `objectFit: "fit"` → `object-fit: contain`
  - `objectFit: "crop"` → `object-fit: cover`
  - default → `object-fit: fill`
- Corner radius applied via CSS class

## Opacity & Transform
- `opacity: N` → `opacity: N`
- `rotation: N` → `transform: rotate(Ndeg)`

## Positioning
- Absolute children → `position: absolute; left: Xpx; top: Ypx`
- Container → `position: relative`

## Semantic HTML Tags
- Font size >= 32 → `<h1>`
- Font size >= 24 → `<h2>`
- Font size >= 20 → `<h3>`
- Other text → `<p>`
- Lines → `<hr>`
- Use `<nav>`, `<header>`, `<main>`, `<section>`, `<footer>`, `<article>` appropriately

## Icon Handling
- Icon font nodes → `<i class="className" data-lucide="icon-name"></i>`
- Set `width`, `height`, and `color` via CSS class
- Include Lucide CDN script for icon rendering

## SVG Elements
- Path nodes → inline `<svg>` with `<path d="..." fill="color" />`
- Set `viewBox`, `width`, `height` on SVG element

## Variable References
- `$variable` refs → `var(--variable-name)` CSS custom properties
- Define variables in `:root { --name: value; }` block
- Background: `background: var(--name)`
- Text color: `color: var(--name)`
- Border: `border-color: var(--name)`

## Responsive Design
- Use `max-width` with `width: 100%` for fluid containers
- Media queries at common breakpoints: `@media (min-width: 640px)`, `768px`, `1024px`, `1280px`
- Use relative units where appropriate (`em`, `rem`, `%`)
