---
name: mobile-ui
description: Mobile screen chrome + layout guardrails (status bar, content rail, search, nav)
phase: [generation]
trigger:
  flags: [isMobileScreen]
priority: 22
budget: 2800
category: base
---

These rules apply to the mobile screen being generated (≤480px wide).

MOBILE STATUS BAR: A status bar (time, signal, wifi, battery) has already been pre-inserted as the first child of the root page frame. Do NOT generate any status bar, system chrome, or OS-level indicators. Start your content directly.

NO PHONE MOCKUP WRAPPER: The whole design IS a mobile screen. Do NOT wrap your section in a phone-shaped frame. Your section root must use width="fill_container" and contain only this section's content.

MOBILE WIDTH SAFETY: Every visible child must stay inside the 390px screen width. Do not create horizontal rows, chips, cards, or buttons that overflow outside the root; wrap, shrink, or clip horizontal lists instead.

MOBILE SINGLE CONTENT RAIL: The root page may keep 0 horizontal padding so the pre-inserted status bar, integrated bottom navigation, and intentional full-bleed media remain full width. Every ordinary transparent root-direct content section must own the same 24px left/right rail exactly once (`padding: [0,24]`) with width="fill_container" and height="fit_content". Do not repeat that inset on an inner wrapper: a purely structural container (the "… Sheet Container" / "… Section Wrapper" shape) inherits the rail's gutter and must carry `padding: [N,0,N,0]` at most — vertical rhythm only, never left/right. Do not create full-width colored wrapper surfaces just to hold content.

MOBILE SCROLLER RAIL: A clipped horizontal scroller is the exception to section padding. Keep the scroller section full width, inset its header 24px on both sides, and give the clipped viewport a 24px leading inset with a flush 0px trailing edge so its first item aligns to the content rail while the next item can clip.

{{mobileRhythm}}

MOBILE TOP RHYTHM: Keep the title/header group close to the first useful module. Do not leave a large blank band above search, categories, promo, charts, or first cards; if you need breathing room, use 20-32px, not a hero-sized void.

MOBILE BOTTOM BREATHING ROOM: A screen WITHOUT a bottom navigation bar must not end flush against the screen edge. Give the last content section 24-32px of bottom padding (`padding: [0,24,32,24]` on a rail section) or close the root with a 24-32px spacer frame, so the final element rests above the device edge instead of colliding with it. Screens that DO end in a bottom tab bar need none of this — the nav is the closing element and stays flush at the bottom.

MOBILE SEARCH BAR: If generating search, output exactly one search control surface: 48-52px tall, width="fill_container" inside the content rail, neutral/surface fill, subtle 1px border, cornerRadius {{mobileSearchRadius}}, and a 18-20px search icon. Optional filter/sliders is a separate 44-48px square button beside it — give it the SAME neutral/surface fill + 1px border as the search field and an accent-colored icon. Do NOT make it an accent-filled (e.g. solid orange) button with a dark icon — that low-contrast orange-on-dark combo reads poorly; accent-fill is only for a control that is actively selected. Do not nest an input inside another rounded pill, do not use pink/tinted fills, and do not make the search section itself a huge rounded band.

MOBILE SECTION CHROME: Search, filter, and category section roots are structural wrappers only. Keep those section roots transparent: no fill, no stroke, no cornerRadius, and no shadow/effects. Put visual styling only on the actual search control, filter button, chips, cards, or promo modules.

NO BLANK PLACEHOLDERS: Do not use empty gray image placeholders in app UI. If no real image asset is available, use a square colored food/icon tile with icon_font instead.

MOBILE HORIZONTAL LISTS: Use a horizontal list only when its wrapper has width="fill_container" and clipContent=true and its inner row uses width="fit_content". Otherwise wrap into rows or a grid. Do not show random half-clipped chips/cards as a design cue.

MOBILE GRID ALIGNMENT: For category chips and product cards, visible items in the same row must share equal width/height and aligned top/bottom edges. On 390px screens prefer two-column grids with {{mobileGridGap}}px gaps; never let the right card extend past the content rail.

MOBILE CARD OVERLAYS: Heart buttons, badges, and status pills on cards must sit fully inside the card with an 8-12px inset. Do not straddle the card border, use negative x/y, or let floating controls protrude outside rounded corners.

MOBILE IMAGE PRESENTATION: During initial image-query or image-prompt authoring, keep sibling photographic slots coherent in subject category and broad lighting/tone, with a consistent aspect ratio, crop direction, and radius. During automatic screenshot-driven self-check, verify only rendering integrity: each intended photographic slot visibly renders exactly one image and its bounds, crop/fit, clipping, radius, and overlay order display correctly. A deliberately authored icon or illustration tile is valid when it renders as intended. Do not judge or replace a displayed image during self-check based on subject relevance, aesthetics, perceived quality, resolution, tone, stock-photo choice, or search/generation result; an explicit user-requested image edit remains allowed.

MOBILE NAV SURFACE: Bottom navigation must use role="bottom-tab-bar" (not navbar), sit on the current page palette, full width at the bottom, 62-72px tall. Do not create a separate white footer band, nested rounded nav pill, oversized rounded pill, or extra side margins. Direct tab item frames must be transparent: no fill, no stroke, no large rounded tile. EVERY tab item carries BOTH an icon AND a text label — keep them consistent across all tabs; never emit a tab (e.g. cart) with an icon but no label. Show active state with accent icon/label color or a tiny 2-3px indicator only. Never use black or safe-dark fills for nav bars unless the whole root frame background is dark.

MOBILE NAV SHADOW: Do not add a drop shadow, glow, or detached shadow band behind the bottom navigation. If separation is needed, use a quiet 1px divider or subtle tonal difference that belongs to the page palette.

NO FIXED FOOD TEMPLATE: Do not default to the same search + categories + orange promo + two product cards composition. For food, shopping, travel, fitness, finance, and social apps, choose a domain-specific visual concept and vary the first viewport composition.

## Platform facts (iOS HIG · Material 3) — the numbers behind the rules

- Type: screen title 28–34 (HIG Large Title 34 / Title 1 28; M3 headline 32/28), section title 20–22 (Title 3 20 / Title 2 22; M3 title-large 22), body 16–17 (Body 17; M3 body-large 16), secondary 15, caption/label 12–13 (Caption 12; M3 label 12). Line height ≈ 1.25–1.3× for titles, 1.3–1.4× for body. Weights 400/500/600; no 300 on mobile.
- Display numbers: one per screen, 36–48 (M3 display-small 36, display-medium 45); never a second one.
- Touch: 44×44pt (HIG) / 48dp (M3) minimum, 8 between adjacent targets. Buttons 48–52 tall, chips 32–36, list rows 56–72.
- Margins: 16–24 horizontal (HIG 16/20, M3 16); our rail is 24. Card padding 16, radius 12–16 (M3 12; HIG cards 12–20).
- Chrome: status bar 62 here; nav/header row 44–56 (HIG 44, M3 top app bar 64); tab bar 49–62 (HIG 49, M3 navigation bar 80 incl. label); FAB 56.
- Colour roles (M3): one primary for actions and the single hero treatment; surfaces step surface → surface-container-low → surface-container → -high for grouping, so sections read by tone, not by lines.
