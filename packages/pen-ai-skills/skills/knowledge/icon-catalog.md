---
name: icon-catalog
description: Icon usage rules and available icon names
phase: [generation]
trigger: null
priority: 20
budget: 1000
category: base
---

ICONS — ALWAYS USE icon_font, NEVER `path` NODES:

```json
{
  "type": "icon_font",
  "name": "Search Icon",
  "iconFontName": "search",
  "iconFontFamily": "lucide",
  "width": 20,
  "height": 20,
  "fill": [{ "type": "solid", "color": "#64748B" }]
}
```

- Sizes: 14 / 20 / 24px. `fill` is the icon color (string or fill array).
- Icon-only buttons: `frame(w=44, h=44, layout=horizontal, alignItems=center, justifyContent=center)` containing one `icon_font`.
- Use lucide names from the list below. NEVER invent names — unknown names fall back to a small circle on canvas.

DO NOT use `path` nodes for icons. The legacy "PascalCase + Icon suffix on a path node" pattern is bug-prone:
when the model wraps it in a frame with a generic child name like "Icon Path" or "Search Icon Path", the resolver
cannot recover the iconic word and the node renders as a placeholder circle. Stick to `icon_font`.

ROLE → ICON NAME MAP (use these exact names — common slips below):

- Cart tab / shopping cart → `shopping-cart` (NEVER `shopping-bag` for a checkout/cart action)
- Bag / tote / package → `shopping-bag`
- Price / money / currency → `dollar-sign` (NOT `dollar`, NOT `currency`)
- Search → `search` (NOT `magnifier`, `magnifying-glass`, `find`)
- Profile / account → `user` (NOT `profile`, `account`)
- Home / house → `house` or `home`
- Orders / receipts → `clipboard-list` or `receipt`
- Notifications → `bell` (NOT `notification`)
- Filter → `filter` or `sliders` (NOT `funnel`)
- Location pin → `map-pin` (NOT `pin`, NOT `location`)
- Time / delivery time → `clock` (NOT `timer`)
- Rating → `star` (NOT `rating`)
- Favorites → `heart` (NOT `favorite`, NOT `like`)
- Pizza category → `pizza`. Sushi → `fish`. Burger → `hamburger`. Healthy → `salad`. Dessert/cake → `cake`. Coffee → `coffee`. Drink → `cup-soda`. Restaurant → `utensils-crossed`. Food (generic) → `utensils`.

COMMON LUCIDE ICON NAMES:
search, bell, user, heart, star, plus, x, check, chevron-right, chevron-left, chevron-down, chevron-up,
settings, home, mail, phone, calendar, clock, map-pin, link, external-link,
eye, eye-off, lock, unlock, key, shield,
arrow-right, arrow-left, arrow-up, arrow-down, arrow-up-right,
menu, more-horizontal, more-vertical, filter, sliders,
image, camera, video, file, folder, download, upload, share, copy, trash,
edit, pen-tool, type, bold, italic, underline, align-left, align-center, align-right,
grid, list, layout, columns, maximize, minimize,
sun, moon, cloud, zap, activity, trending-up, trending-down, bar-chart, pie-chart,
users, user-plus, user-check, message-circle, message-square, send,
shopping-cart, shopping-bag, credit-card, dollar-sign, gift, tag, bookmark,
play, pause, skip-forward, skip-back, volume-2, mic,
github, twitter, instagram, facebook, linkedin, youtube,
globe, wifi, bluetooth, monitor, smartphone, tablet, cpu, database, server, hard-drive,
code, terminal, git-branch, git-commit, git-pull-request,
alert-circle, alert-triangle, info, help-circle, check-circle, x-circle,
pizza, fish, hamburger, salad, cake, coffee, cup-soda, utensils, utensils-crossed, beef, croissant, apple, cookie, ice-cream-cone, banana, carrot, wheat, soup, donut
