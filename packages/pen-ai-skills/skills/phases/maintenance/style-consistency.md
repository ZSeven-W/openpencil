---
name: style-consistency
description: Preserve visual consistency when modifying existing designs
phase: [maintenance]
trigger: null
priority: 10
budget: 1000
category: base
---

STYLE CONSISTENCY RULES：

修改 existing design 时，保持 visual coherence：

COLOR PALETTE：

- 修改前，从 context nodes 中提取 existing palette。
- 除非用户明确要求 new colors，否则 new elements 必须使用 existing palette 中的 colors。
- 保持相同的 accent color usage pattern（primary 用于 CTAs，secondary 用于 highlights）。

TYPOGRAPHY：

- 匹配 existing font families — 除非用户要求，不要引入 new fonts。
- 保持相同的 type scale（heading sizes、body sizes、caption sizes）。
- 保留 existing text nodes 的 lineHeight 和 letterSpacing patterns。

SPACING：

- 添加 new sections 或 elements 时，匹配 existing padding 和 gap values。
- Section padding 应在整个 design 中保持一致。
- Card internal padding 应匹配 sibling cards。

VISUAL TREATMENT：

- similar element types 的 cornerRadius 应保持一致。
- Shadow styles 应匹配同 category 的 existing elements。
- Border/stroke styles 应保持一致（same color、same thickness）。
- clipContent 应匹配 sibling containers。

HIERARCHY：

- 保持相同 nesting depth — 不要添加不必要的 wrapper frames。
- 保持相同 layout pattern（vertical sections with horizontal content rows）。
- Width strategies（fill_container vs fixed）应匹配 siblings。
