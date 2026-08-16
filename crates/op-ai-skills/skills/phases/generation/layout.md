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
- CHILD WIDTH RULE: child width must be <= parent content area. Use `width="fill_container"` when in doubt.
- HEIGHT DEFAULT: content-bearing frames/sections/cards/wrappers use `height="fit_content"` (Hug). Use `height="fill_container"` only for an explicit remainder consumer under a definite-height parent (e.g. a sidebar/work surface, a clipped scroll viewport) or cross-axis stretch inside a fixed-height horizontal row.
- `space_between`, uneven card content, or wanting to remove bottom whitespace are NOT reasons to switch a container to Full Height — don't make every sibling card Full Height just to equalize a row.
- In vertical layout, `width="fill_container"` stretches horizontally; in horizontal layout it fills remaining width. A child's `height="fill_container"` stretches only across a row's definite cross-axis height.
- CLIP CONTENT: clipContent: true clips overflowing children. ALWAYS use on cards with cornerRadius + image.
- ABSOLUTE-STACK Z-ORDER: `layout: "none"` children are front-to-back by array index — `children[0]` is TOPMOST because canvas paint walks the array in reverse. Put badges/labels/controls/scrims BEFORE the full-bleed media or background they must cover. Keep the media as a separate EMPTY frame/rectangle slot for strict `G(...)`; never use the badge-owning stack itself as the image slot.
- CARD HEIGHT: cards holding text + a CTA (promo/offer/info cards) MUST use height="fit_content" — NEVER a fixed pixel height, which + clipContent clips the CTA. Reserve fixed heights for image-only cards with a known aspect ratio.
- justifyContent: "space_between" (navbars), "center", "start"/"end", "space_around".
- SIBLING ISOMORPHISM: one width strategy + one structure — same nesting/indent/type ladder; only copy differs. Don't mix fixed-px and fill_container.
- Never create a main-axis circular dependency by making a child Full Height solely under a Hug Height vertical parent.
- Two-column: horizontal frame - two child frames each "fill_container" width.
- Keep hierarchy shallow: no pointless wrappers. Only use wrappers with visual purpose (fill, padding).
- Section root: width="fill_container", height="fit_content", layout="vertical".
- MARGIN FLOOR: root h-pad ≥64 (1920)/≥48 (1080); no text at canvas edge (full-bleed bg image is the only exception).
- TRANSPARENT INNER SECTIONS — interior wrappers (Header, Search Section, Categories, etc.) MUST have `fill: []` (inherit page bg); an explicit fill like #FFFFFF creates an unwanted white card against a colored page. Only opt into a fill when the section IS intentionally a card with its own surface (a promo banner, inset card).
- FORMS: ALL inputs AND primary button MUST use width="fill_container". Vertical layout, gap=16-20.

HORIZONTAL ROW WIDTH MATH (CRITICAL — prevents off-canvas clipping): before emitting N fixed-px items in a row, verify `total_width = N × item_width + (N−1) × gap ≤ parent_inner_width` (`parent_width − padding_left − padding_right`) — the renderer clips oversized rows, it does NOT shrink items. Mobile 375px page, 3 items / 24px gap / 24px padding → max ≈76-80px each. If items don't fit: use `fill_container` width, drop an item, or wrap into a 2×2 grid instead of overflowing.

CATEGORY / ICON-CHIP RAILS — SPREAD TO FILL, DON'T CLUSTER LEFT: a fixed small set (3-5) of equal category tiles / icon chips on one mobile row should span the row's full width, not cluster left with dead space on the right. When chips don't fill the row, set `justifyContent="space_between"` so the leftover becomes EVEN, larger gaps — keep each chip its natural fixed size, don't stretch tiles (exactly two chips is the exception, use a normal start gap instead). Use a fixed-width scroll rail (`justifyContent="start"`) only for 6+ tiles — a genuine horizontal scroller.

NO FIXED-POSITION LAYOUT — DO NOT EMIT BOTTOM SPACERS: OpenPencil has no `position: fixed` / `position: sticky`. A bottom nav bar is an inline page child, not a floating overlay — never emit a trailing empty spacer frame after it. It already sits in normal page flow; a spacer just adds dead pixels at the bottom. Omit it.

RING / PIE / ARC / DONUT / GAUGE / DISC (progress ring, pie chart, gauge, avatar): native ELLIPSE with arc fields, no frame tricks. innerRadius (0..1, donut hole; 0/omit=solid, 0.6=thick ring, 0.85=thin ring) + startAngle/sweepAngle (deg, 0=right, clockwise; omit both = full ring). SOLID DISC=fill only. TRACK=innerRadius 0.8. PROGRESS ARC=innerRadius 0.8 + startAngle -90 + sweepAngle 270. PIE SLICE=startAngle+sweepAngle only. GAUGE=innerRadius 0.7 + startAngle 180 + sweepAngle 180. Never fake a hole with a second ellipse — use innerRadius. Ellipse has no children: a filled circle with centered text/icon (badge/avatar) uses a square frame instead — cornerRadius=width/2, layout="horizontal", alignItems/justifyContent="center". A ring WITH centered content needs a fixed-size layout="none" wrapper overlaying same-size track + progress ellipses plus a centered content frame at explicit x/y — never as flex siblings (they would sit beside each other, not concentric); z-order (index 0 = top): content, progress, track. textAlignVertical is unsupported — center via the content frame, not the ellipses. Full worked example: the `shapes-and-decks` knowledge skill.

STACKED CARD / DECK ("cards behind a card", swipeable stack peek): 1) BACK LAYER IS DECORATIVE ONLY — bare rectangle/frame, cornerRadius + fill (+ optional stroke), NEVER text/icon/content children. 2) OFFSET IS A PEEK, NOT A RELAYOUT — shift it 8-16px on one/both axes, not far enough to read as a second card. 3) FRONT LAYER MUST BE OPAQUE — real `fill` (e.g. `$color-surface`) so the back layer only shows at the peeking edge. Build as `layout="none"`: front card = `children[0]` (topmost, per ABSOLUTE-STACK Z-ORDER above); back layer(s) = later children, offset further and empty for 3+ cards — never insert one before the front card or give any of them text. Full worked example: the `shapes-and-decks` knowledge skill.

AESTHETIC HYGIENE — keep these silent (never emit, the post-pass also strips them):

- TEXT NODES NEVER GET: cornerRadius, stroke, effects, rotation — text is filled glyphs, clip/border/shadow/tilt all render wrong. (Stroke is for icon-font nodes only.)
- ROTATION on UI frames is almost always wrong — use rotation=0 (or omit) on cards/buttons/containers/labels. Only legitimate rotations: exact 90/180/270 (vertical text, rotated grid) and decorative geometry (path/line/polygon/image).
- Keep same-role siblings consistent: cards in one row share a cornerRadius and padding rather than drifting (8/8/12 reads ragged) — pick one value, reuse it per group.
- EXACT USER TOKENS OVERRIDE EXAMPLES. If the prompt specifies exact radius/spacing values, apply them to ordinary component cornerRadius, group gap, and repeated card spacing (e.g. "圆角 8px / 间距 12px" → cornerRadius=8, gap=12) unless a tiny inline icon pair clearly needs a smaller micro-gap.
- INNER LAYOUT FRAMES (sections, wrappers, header/body containers inside a card) need NO fill/stroke/shadow — they inherit from the page/card surface. Only the OUTER card/button/badge/chip opts into a fill/border/shadow, never the wrapper holding it.
- MOBILE CONTENT RAIL LIVES ON ORDINARY SECTIONS, NOT THE PAGE ROOT. A mobile root may keep horizontal padding 0 so the status bar, integrated bottom nav, and full-bleed media stay full width. Every ordinary transparent root-direct content section owns the same 24px left/right rail once (`padding: [0,24]`) — do not repeat it on an inner wrapper. Exception: a clipped horizontal scroller stays full width, insets its header 24px both sides, and gives the viewport a 24px leading inset with a flush 0px trailing edge. Never stack root+section or section+inner-wrapper padding.
