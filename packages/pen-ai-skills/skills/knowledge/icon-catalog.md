---
name: icon-catalog
description: Icon usage rules and available icon names
phase: [generation]
trigger: null
priority: 20
budget: 1000
category: base
---

ICONS:

- Use "path" nodes, size 16-24px. ONLY use Feather icon names — PascalCase + "Icon" suffix (e.g. "SearchIcon").
- System auto-resolves names to SVG paths. "d" is replaced automatically.
- NEVER use emoji as icons. Use icon_font nodes for lucide icons.

ICON_FONT NODES:

- Use icon_font type with iconFontName for lucide icons (e.g. iconFontName="search", "bell", "user").
- Sizes: 14/20/24px. Fill can be a color string.
- Icon-only buttons: frame(w=44, h=44, layout=none) > icon_font(x=12, y=12)

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
alert-circle, alert-triangle, info, help-circle, check-circle, x-circle
