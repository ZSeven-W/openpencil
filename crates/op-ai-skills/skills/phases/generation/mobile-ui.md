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

MOBILE SINGLE CONTENT RAIL: All non-chrome sections must align to the same 24px left/right content rail. Section roots should stay transparent with width="fill_container" and height="fit_content"; apply padding once at the section root or first content wrapper. Do not create full-width colored wrapper surfaces just to hold content.

{{mobileRhythm}}

MOBILE TOP RHYTHM: Keep the title/header group close to the first useful module. Do not leave a large blank band above search, categories, promo, charts, or first cards; if you need breathing room, use 20-32px, not a hero-sized void.

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
