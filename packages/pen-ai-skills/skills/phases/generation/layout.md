---
name: layout
description: Auto-layout engine rules (flexbox-based positioning)
phase: [generation]
trigger: null
priority: 10
budget: 1700
category: base
---

LAYOUT ENGINE（基于 flexbox）：

- 带有 layout: "vertical"/"horizontal" 的 frames 会通过 gap、padding、justifyContent、alignItems 自动定位 children。
- 绝不要给 layout containers 内的 children 设置 x/y。
- CHILD SIZE RULE：child width 必须 <= parent content area。不确定时使用 "fill_container"。
- 在 vertical layout 中："fill_container" width 会水平拉伸。在 horizontal 中：填充剩余空间。
- CLIP CONTENT：clipContent: true 会裁剪溢出的 children。带有 cornerRadius + image 的 cards 始终使用它。
- justifyContent: "space_between"（navbars）、"center"、"start"/"end"、"space_around"。
- WIDTH CONSISTENCY：siblings 必须使用相同的 width strategy。不要混用 fixed-px 和 fill_container。
- 绝不要在 "fit_content" parent 的 children 上使用 "fill_container" — 这会产生 circular dependency。
- Two-column：horizontal frame - 两个 child frames 都使用 "fill_container" width。
- 保持层级浅：不要使用无意义 wrappers。只有在具备视觉目的（fill、padding）时才使用 wrappers。
- Section root：width="fill_container"，height="fit_content"，layout="vertical"。
- FORMS：所有 inputs 和 primary button 必须使用 width="fill_container"。Vertical layout，gap=16-20。

HORIZONTAL ROW WIDTH MATH（关键 — 防止 off-canvas clipping）：

在 fixed-width parent 内横向布局 N 个 items 时，total width 必须能放下。公式是：

total_width = N × item_width + (N − 1) × gap
parent_inner_width = parent_width − padding_left − padding_right

在输出 fixed-px items 前，你必须验证 `total_width ≤ parent_inner_width`。Renderer 不会缩放 items 以适配 — 它会直接裁剪。

Mobile（375px page width）常见情况：

- 3 items, 24px gap, 24px page padding, 24px card padding → inner = 327−48 = 279
  Max item width = (279 − 48) / 3 = 77px。使用 76 或 80，不要用 100。
- 4 items, 16px gap, 24px page padding → inner = 327
  Max item width = (327 − 48) / 4 = 69px.
- 2 items, 16px gap, 24px page padding → inner = 327
  Max item width = (327 − 16) / 2 = 155px.

如果 3 个 items 无法按你想要的尺寸放下，请给每个 item 使用 `fill_container` width（它们会自动共享空间），或者减少一个 item，或者使用 2×2 grid（vertical layout 中包含两个 horizontal rows）。

Anti-pattern（activity-rings overflow bug）：在 375px-wide page 上，一个 padding 为 24px 的 card 内输出三个 100px rings，并设置 24px gap。Total 348px > 279px inner → 第三个 ring 会在右侧边缘被静默裁剪。

NO FIXED-POSITION LAYOUT — 不要输出 BOTTOM SPACERS：

OpenPencil 没有 `position: fixed` / `position: sticky`。Bottom navigation bars 是 page 的 inline children，不是 floating overlays。你不需要（也绝不能）用空 spacer frames 为它们预留空间。

Anti-pattern：
page: { layout: vertical, children: [
...content...,
{ role: "bottom-tab-bar", height: 62 },
{ id: "bottom-spacer", width: "fill_container", height: 62, children: [] } // ← WRONG
]}

尾部 spacer 会在 page 底部无视觉理由地增加 62 dead pixels。bottom-tab-bar 已经是 page flow 的一部分；spacer 是在为这个 engine 中不存在的 fixed positioning pattern 预留空间。直接省略它。

RING / CIRCLE WITH CENTER CONTENT（Apple Activity Ring、progress ring、badge、avatar with text）：

- 使用 frame(cornerRadius=width/2) 作为 ring/circle。绝不要使用 ellipse + sibling text。
  原因：ellipse 不能有 children。把 text 作为 ellipse 的 sibling 放进 vertical/horizontal layout parent 时，会被堆叠布局 — text 会出现在 ring 上方/下方，而不是中心。
- 正确模式：
  frame(width=80, height=80, cornerRadius=40, stroke={thickness:8, fill:[ringColor]}, fill:[],
  layout="horizontal", alignItems="center", justifyContent="center")
  └── text(content="8,432", fontSize=16, fontWeight=700, fill:[textColor])
- 对于 EMPTY RING（仅 stroke），在 frame 上设置 fill: []。不要添加一个更小的 "inner" ellipse 并使用 parent 的 background color 试图“打孔” — 这是 raster-era trick，在 OpenPencil 的 flex-layout model 中不起作用。
- 对于 SOLID DISC，设置 frame fill: [{type:"solid", color:...}] 并省略 stroke。
- 不要使用 layout: "none" + 带 absolute x/y 的 nested frame 在 circle 上叠加文本。
  layout=none + nested children 渲染不可靠。始终改用 frame+cornerRadius 与标准 flex layout。
- textAlignVertical 不受支持。使用 layout=horizontal/vertical parent + alignItems=center + justifyContent=center，在任意 container 内居中文本。
