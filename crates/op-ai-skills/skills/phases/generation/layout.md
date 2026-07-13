---
name: layout
description: Auto-layout engine rules (flexbox-based positioning)
phase: [generation]
trigger: null
priority: 10
budget: 2000
category: base
---

LAYOUT ENGINE (flexbox-based):

- Frames with layout: "vertical"/"horizontal" auto-position children via gap, padding, justifyContent, alignItems.
- NEVER set x/y on children inside layout containers.
- CHILD WIDTH RULE: child width must be <= parent content area. Use `width="fill_container"` when in doubt; this advice is for WIDTH only.
- HEIGHT DEFAULT: content-bearing frames, sections, cards, and wrappers use `height="fit_content"` (Hug). Use `height="fill_container"` only for an explicitly designated remainder consumer under a definite-height parent (for example a desktop sidebar/work surface or a clipped scroll viewport), or for cross-axis stretch inside a fixed-height horizontal control/row.
- `space_between`, uneven card content, short page content, or a desire to remove bottom whitespace are NOT sufficient reasons to switch a content container to Full Height. Do not make every sibling card Full Height merely to equalize a row.
- In vertical layout, `width="fill_container"` stretches horizontally. In horizontal layout it fills remaining width; a child's `height="fill_container"` stretches only across that row's definite cross-axis height.
- CLIP CONTENT: clipContent: true clips overflowing children. ALWAYS use on cards with cornerRadius + image.
- ABSOLUTE-STACK Z-ORDER: `layout: "none"` children are front-to-back by array index — `children[0]` is TOPMOST because canvas paint walks the array in reverse. Put badges/labels/controls/scrims BEFORE the full-bleed media or background they must cover. Keep the media as a separate EMPTY frame/rectangle slot for strict `G(...)`; never use the badge-owning stack itself as the image slot.
- CARD HEIGHT: cards holding text + a CTA (promo banners, offer/info cards) MUST use height="fit_content" — NEVER a fixed pixel height. A fixed height + clipContent clips the bottom child (the CTA shows half / not at all). Reserve fixed heights for image-only cards with a known aspect ratio.
- justifyContent: "space_between" (navbars), "center", "start"/"end", "space_around".
- WIDTH CONSISTENCY: siblings must use same width strategy. Don't mix fixed-px and fill_container.
- Never create a main-axis circular dependency by making a child Full Height solely under a Hug Height vertical parent.
- Two-column: horizontal frame - two child frames each "fill_container" width.
- Keep hierarchy shallow: no pointless wrappers. Only use wrappers with visual purpose (fill, padding).
- Section root: width="fill_container", height="fit_content", layout="vertical".
- TRANSPARENT INNER SECTIONS — interior section wrappers (Header, Search Section, Categories Section, Near You Section, etc.) MUST have `fill: []` (transparent / inherit page bg). Adding an explicit fill like `#FFFFFF` on inner sections creates an unwanted "white card" against a colored page bg. Only use an explicit fill when the section is INTENTIONALLY a card with its own surface (a promo banner, an inset card, a different surface tone). Default to `fill: []` and only opt into a fill when you want a visible surface boundary.
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

CATEGORY / ICON-CHIP RAILS — SPREAD TO FILL, DON'T CLUSTER LEFT:

A FIXED small set (3-5) of equal category tiles / icon chips on one mobile row
should span the row's full width, not cluster on the left with a lopsided empty
band on the right (4×56px chips + 3×12px gap = 260px inside a 335px row → 75px
dead space, which reads as unbalanced). When the chips DON'T fill the row, set
the chip row's justifyContent="space_between" so the leftover space becomes
EVEN, larger gaps between chips (the user's "撑不满就把间距放大一点"). Keep each
chip its natural fixed size (a vertical icon-tile + label frame); do not stretch
the tiles. (Two chips are the exception — space_between throws them to opposite
edges, so for exactly two use a normal start gap instead.) Use a fixed-width
scroll rail (justifyContent="start") ONLY when there are clearly more tiles than
fit on one row (6+) — a genuine horizontal scroller.

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

RING / PIE / ARC / DONUT / GAUGE / DISC (Apple Activity Ring, progress ring, pie chart, gauge, avatar):

- Use a native ELLIPSE with arc fields. The renderer carves the shape directly — no frame tricks.
  - innerRadius (0..1 fraction): carves a donut hole. 0 (or omit) = solid disc; 0.6 = thick ring; 0.85 = thin ring.
  - startAngle + sweepAngle (degrees): carve a pie slice / arc / gauge. 0° points right, sweeps clockwise.
    Omit both for a full 360° ring/disc.
- Patterns:
  - SOLID DISC: ellipse(width=80, height=80, fill:[{type:"solid", color:...}])
  - EMPTY RING / progress track: ellipse(width=80, height=80, innerRadius=0.8, fill:[{type:"solid", color:trackColor}])
  - PROGRESS ARC (e.g. 75%): ellipse(..., innerRadius=0.8, startAngle=-90, sweepAngle=270, fill:[{type:"solid", color:progressColor}])
  - PIE SLICE: ellipse(..., startAngle=0, sweepAngle=120, fill:[...]) (no innerRadius)
  - GAUGE (half ring): ellipse(..., innerRadius=0.7, startAngle=180, sweepAngle=180, fill:[...])
  - Do NOT punch a hole with a smaller bg-colored ellipse on top — use innerRadius.
- CAVEAT — ellipse cannot have children. For a ring/circle WITH centered text or icon (badge, avatar
  with initials, progress ring with a "%" label), wrap the ring ellipse + the text as SIBLINGS in a
  layout="horizontal" frame { alignItems:"center", justifyContent:"center" } — or keep the
  frame(cornerRadius=width/2) pattern for that centered-content case only:
  frame(width=80, height=80, layout="horizontal", alignItems="center", justifyContent="center")
  ├── ellipse(width=80, height=80, innerRadius=0.85, fill:[ringColor])   ← the ring
  └── text(content="8,432", fontSize=16, fontWeight=700, fill:[textColor])  ← the centered label
  (When the ring fully encloses the text, the frame+cornerRadius single-node form also works.)
- DO NOT use layout: "none" + nested frame with absolute x/y to overlay text on a circle.
  layout=none + nested children renders unreliably. Use the sibling-in-centered-frame pattern instead.
- textAlignVertical is NOT supported. Use a layout=horizontal/vertical parent + alignItems=center
  - justifyContent=center to center text inside any container.

AESTHETIC HYGIENE — keep these silent (never emit, the post-pass also strips them):

- TEXT NODES NEVER GET: cornerRadius, stroke, effects, rotation. Text is filled glyphs — clip /
  border / shadow / tilt all come out wrong on canvas. (Stroke is for icon-font nodes only.)
- ROTATION on UI frames is almost always wrong. Use rotation=0 (or omit) on cards / buttons /
  containers / labels. The only legitimate rotations are exact 90 / 180 / 270 (vertical text /
  rotated grid) and rotation on path / line / polygon / image (decorative geometry).
- Keep same-role siblings visually consistent: cards in one row should share a cornerRadius and
  padding rather than drifting (8 / 8 / 12 reads as ragged). Pick a value and reuse it per group.
- EXACT USER TOKENS OVERRIDE EXAMPLES. If the prompt specifies exact radius or spacing values,
  apply those exact values to ordinary component `cornerRadius`, group `gap`, and repeated card
  spacing. For example, "圆角 8px / 间距 12px" means same-role controls and cards should use
  cornerRadius=8 and gap=12 unless a tiny inline icon pair clearly needs a smaller micro-gap.
- INNER LAYOUT FRAMES (sections, wrappers, header / body containers inside a card) DO NOT need
  fill, stroke, OR shadow. They inherit from the page / card surface. Only opt into a fill /
  border / shadow on the OUTER card, button, badge, chip — NEVER on the wrapper that holds it.
- ONE PAGE GUTTER, ON THE ROOT. The root frame carries the horizontal gutter (e.g.
  `padding: [0,20]`); EVERY content section uses horizontal padding 0 and only sets vertical
  padding. This is what keeps every section's left edge aligned — if sections each set their own
  h-padding (one 20, one 16, one 0) their content no longer lines up. Hero / banner / image-bleed
  sections sit edge-to-edge by simply NOT adding horizontal padding (the root gutter shows
  through). Never stack both (root gutter + per-section h-padding = a doubled inset).
