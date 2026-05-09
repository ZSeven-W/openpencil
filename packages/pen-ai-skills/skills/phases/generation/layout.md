---
name: layout
description: Auto-layout engine rules (flexbox-based positioning)
phase: [generation]
trigger: null
priority: 10
budget: 1700
category: base
---

LAYOUT ENGINE (flexbox-based):

- Frames with layout: "vertical"/"horizontal" auto-position children via gap, padding, justifyContent, alignItems.
- NEVER set x/y on children inside layout containers.
- CHILD SIZE RULE: child width must be <= parent content area. Use "fill_container" when in doubt.
- In vertical layout: "fill_container" width stretches horizontally. In horizontal: fills remaining space.
- CLIP CONTENT: clipContent: true clips overflowing children. ALWAYS use on cards with cornerRadius + image.
- justifyContent: "space_between" (navbars), "center", "start"/"end", "space_around".
- WIDTH CONSISTENCY: siblings must use same width strategy. Don't mix fixed-px and fill_container.
- NEVER use "fill_container" on children of "fit_content" parent — circular dependency.
- Two-column: horizontal frame - two child frames each "fill_container" width.
- Keep hierarchy shallow: no pointless wrappers. Only use wrappers with visual purpose (fill, padding).
- Section root: width="fill_container", height="fit_content", layout="vertical".
- FORMS: ALL inputs AND primary button MUST use width="fill_container". Vertical layout, gap=16-20.

HORIZONTAL ROW WIDTH MATH (CRITICAL — prevents off-canvas clipping):

When laying out N items horizontally inside a fixed-width parent, the total
width MUST fit. The formula is:

total_width = N × item_width + (N − 1) × gap
parent_inner_width = parent_width − padding_left − padding_right

You MUST verify `total_width ≤ parent_inner_width` before emitting fixed-px
items. The renderer does NOT scale items to fit — it clips them.

Mobile (375px page width) common cases:

- 3 items, 24px gap, 24px page padding, 24px card padding → inner = 327−48 = 279
  Max item width = (279 − 48) / 3 = 77px. Use 76 or 80, NOT 100.
- 4 items, 16px gap, 24px page padding → inner = 327
  Max item width = (327 − 48) / 4 = 69px.
- 2 items, 16px gap, 24px page padding → inner = 327
  Max item width = (327 − 16) / 2 = 155px.

If 3 items don't fit at the size you want, use `fill_container` width on each
(they auto-share space) OR drop one item OR use a 2×2 grid (vertical layout
with two horizontal rows).

Anti-pattern (the activity-rings overflow bug): emitting three 100px rings
with 24px gap inside a card with 24px padding on a 375px-wide page. Total
348px > 279px inner → the third ring is silently clipped on the right edge.

NO FIXED-POSITION LAYOUT — DO NOT EMIT BOTTOM SPACERS:

OpenPencil has no `position: fixed` / `position: sticky`. Bottom navigation
bars are inline children of the page, not floating overlays. You do NOT need
to (and MUST NOT) reserve space for them with empty spacer frames.

Anti-pattern:
page: { layout: vertical, children: [
...content...,
{ role: "bottom-tab-bar", height: 62 },
{ id: "bottom-spacer", width: "fill_container", height: 62, children: [] } // ← WRONG
]}

The trailing spacer adds 62 dead pixels at the bottom of the page for no
visual reason. The bottom-tab-bar is already part of the page flow; the
spacer was reserving space for a fixed positioning pattern that doesn't
exist in this engine. Just omit it.

RING / CIRCLE WITH CENTER CONTENT (Apple Activity Ring, progress ring, badge, avatar with text):

- Use frame(cornerRadius=width/2) AS the ring/circle. NEVER ellipse + sibling text.
  Reason: ellipse cannot have children. Putting text as ellipse's sibling in a vertical/horizontal layout
  parent stacks them — text ends up above/below the ring, NOT in the center.
- Correct pattern:
  frame(width=80, height=80, cornerRadius=40, stroke={thickness:8, fill:[ringColor]}, fill:[],
  layout="horizontal", alignItems="center", justifyContent="center")
  └── text(content="8,432", fontSize=16, fontWeight=700, fill:[textColor])
- For an EMPTY RING (stroke only), set fill: [] on the frame. Do NOT add a smaller "inner" ellipse
  with the parent's background color trying to "punch a hole" — that's a raster-era trick that
  doesn't work in OpenPencil's flex-layout model.
- For SOLID DISC, set frame fill: [{type:"solid", color:...}] and omit stroke.
- DO NOT use layout: "none" + nested frame with absolute x/y to overlay text on a circle.
  layout=none + nested children renders unreliably. Always use frame+cornerRadius with standard
  flex layout instead.
- textAlignVertical is NOT supported. Use a layout=horizontal/vertical parent + alignItems=center
  - justifyContent=center to center text inside any container.
