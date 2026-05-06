---
name: icon-catalog
description: Icon usage rules and available icon names
phase: [generation]
trigger: null
priority: 20
budget: 1000
category: base
---

ICONS：

- 使用 "path" nodes，size 16-24px。只使用 Feather icon names — PascalCase + "Icon" suffix（例如 "SearchIcon"）。
- System 会自动把 names 解析为 SVG paths。"d" 会被自动替换。
- 绝不要使用 emoji 作为 icons。lucide icons 使用 icon_font nodes。

ICON_FONT NODES：

- lucide icons 使用带 iconFontName 的 icon_font type（例如 iconFontName="search", "bell", "user"）。
- Sizes：14/20/24px。Fill 可以是 color string。
- Icon-only buttons：frame(w=44, h=44, layout=none) > icon_font(x=12, y=12)

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
