---
name: mobile-app
description: Mobile app three-section architecture with enforced Blueprint
phase: [generation]
trigger:
  keywords: [mobile, phone, ios, android, 移动, 手机]
priority: 25
budget: 2000
category: domain
---

MOBILE APP — MANDATORY THREE-SECTION ARCHITECTURE:

Every mobile screen is composed as a vertical stack of exactly three sections.
You MUST define all three before generating any content.

Screen-height contract: use numeric 390-393×844 as a temporary construction seed so an empty skeleton is visible. Before finishing, a normal content-driven mobile page switches its root to `height="fit_content"` (Hug), matching its completed flow. Keep a numeric viewport only when the user explicitly requested that viewport/device frame or the design deliberately contains one clipped viewport body that must consume remaining height.

## 1) STATUS BAR (OS-controlled) — PRE-INSERTED

The status bar (time, signal, wifi, battery) is **automatically pre-inserted** by the orchestrator as the first child of the root frame. It is a fixed 62px-tall frame with hardcoded path icons.

- **DO NOT generate a status bar** — it already exists
- **DO NOT delete or modify** the pre-inserted status bar
- Your first section should start BELOW the status bar (it occupies ~62px)

## 2) APP CONTENT (your layout)

Chip rows (filter/date/guests pills): each chip HUGS (width fit_content, single-line text, height 36-44, cornerRadius=full); the ROW clips overflow (clipContent) instead of squeezing chips — never let a pill's text wrap. A badge/pill/button frame ALWAYS carries its content (text or icon) — an empty decorated frame renders as a mystery blob.

ALL content elements must sit inside ONE wrapper container (vertical stack).

The ordinary content wrapper is `width="fill_container"`, `height="fit_content"`, `layout="vertical"`. Its child sections/cards also Hug Height. Only a deliberately clipped scroll viewport under an explicit fixed-height root may use `height="fill_container"`; in that case it is the ONE named remainder consumer, not a sizing mode copied onto its sections.

Wrapper provides:

- Consistent left/right padding: 16-28px (applied ONCE at wrapper level; reference-measured screens use [0,24] on the wrapper with a 402px root)
- Gap-based vertical spacing between sections (use gap, NOT margins): 24-40 by density
- padding-bottom equal to the gap value for bottom space (NOT spacer elements)

Content stacking order inside the wrapper:

1. Top context: title / navigation header / search / filters
2. Primary content: the main "job to be done" for this screen
3. Supporting content: secondary modules, help text, empty states
4. Floating actions (optional): FAB or sticky CTA

Rules:

- One primary intent per screen. Everything else is subordinate.
- First 1-2 elements must answer "where am I" + "what can I do here"
- Mobile top rhythm: keep header/title close to the first useful control or content; use 20-32px, not an empty hero-sized band.
- Section header actions: prefer a 20px `chevron-right` / `arrow-right` icon, not visible "See all", "View all", "查看全部", or "查看更多" text.
- Category sections: section root and chip row both use height="fit_content". Use a header row, then chip row/grid. Chip row uses gap 12 and justifyContent start even when there are only two categories; never space_between/space_around. Each category item frame contains icon + label. Show four full chips or wrap; no half-clipped item.
- Product card rows: two equal `fill_container` cards, gap 12, inside the content rail; no fixed-width clipped second card.
- List rows of [thumbnail, text stack]: alignItems="center" on the row — a missing alignItems top-pins the text against a taller thumbnail and leaves a dead band under it.
- Corner badge on an image ("-35%", "NEW"): a CHILD of the image's wrapper frame with explicit x/y (e.g. x=8, y=8) — never a card-level sibling between the image and the content column (it renders straddling the seam).
- Price + unit ("$1,170" + "/ person", "$29" + "/mo"): ONE hugging row, gap 4-8, alignItems baseline/end — price 20-24px bold accent, unit 12-13px muted right beside it. Never space_between them across the card (the unit ends up orphaned at the far edge). Strikethrough original price: smaller muted line above or inline before the deal price.
- Header cart/notification controls are neutral icon buttons; counts are tiny circular badges, not square number blocks.
- Exact user mobile tokens win: "圆角 8px / 间距 12px" means ordinary radius=8 and repeated gaps=12.
- Title font size must be uniform across ALL screens in the app
- Design for one-handed use: primary actions in lower half
- When the screen is explicitly a fixed viewport, use at most one clipped content viewport; otherwise keep the content wrapper Hug Height. Avoid nested scrolls.
- Touch targets: minimum 44x44px
- Do not repeat the same predictable mobile stack of search + categories + orange promo + two cards. Choose a distinct concept for the domain and make one signature moment carry the personality.

DO NOT:

- Add per-section horizontal padding (wrapper handles it)
- Use spacer elements for bottom space (use padding-bottom)
- Cram multiple competing sections above the fold

## 3) BOTTOM TAB BAR — OPTIONAL, INTEGRATED

Do not force bottom navigation into every mobile screen. Use a bottom tab bar only when the product clearly has persistent top-level destinations (Home, Search, Orders, Profile, etc.). If the screen is a single-task flow, omit bottom navigation. 3-5 tabs, top-level destinations only. Make it the LAST child in the screen's vertical stack — never absolutely positioned.

Pick ONE of two idioms and commit:

IDIOM A — INTEGRATED BAR (Android / utility / data-dense apps):

- Full screen width, part of the page flow, role="bottom-tab-bar"
- Height 62-72px; background = same page palette or a subtle tonal surface
- Separation: quiet 1px divider or tonal contrast only; no detached shadow band
- Not a floating pill, not a nested rounded capsule, not a separate footer band; direct tab item frames stay transparent (no fill / stroke / rounded tile)

IDIOM B — FLOATING CAPSULE (iOS-native / premium / consumer apps):

- A capsule that floats inset from the edges: ~16px sides, ~12px above the bottom — never flush. Give the screen stack ~12px bottom padding so the capsule clears the edge.
- ~56px tall, cornerRadius = half the height (true capsule ends), ~6px inner padding
- Frosted look: tonal surface fill at ~70% opacity + one soft shadow (the only shadowed element on the screen)
- Selected item sits on a soft accent-tinted capsule highlight; inactive items are transparent

Tab Items (both idioms):

- Width / height fill_container; layout vertical, gap 4, centered both axes
- Icon ~18-22px above a label (10-11px, weight 500-600, sentence case, letterSpacing 0)
- Selected: accent icon+label, FILLED icon variant. Inactive: muted neutral, OUTLINE icon variant. The fill/outline swap is the primary state signal, not color alone.

Rules: tab switching preserves each tab's state; app content must never be obscured by the bar.

## BLUEPRINT (internal planning)

Before generating nodes, mentally verify these three layers are accounted for:

1. Status Bar: standard or edge-to-edge?
2. App Content: what is the header, primary content, action placement, scroll behavior?
3. Bottom Bar: None, integrated bar, or floating capsule (which idiom, which tabs)?

Do NOT output this blueprint as text. Apply it silently through your node structure.
Your output must remain valid JSON/JSONL only.
