---
name: dashboard
description: Dashboard and admin panel design patterns
phase: [generation]
trigger:
  keywords: [dashboard, admin, analytics, data]
priority: 35
budget: 1500
category: domain
---

DASHBOARD DESIGN PATTERNS：

STRUCTURE：

- Root frame：width=1200，height=0，layout="horizontal"（sidebar + main content）
- Sidebar：width=240-280，height="fill_container"，layout="vertical"，使用 dark 或 surface fill
- Main content：width="fill_container"，layout="vertical"，gap=16-24

SIDEBAR：

- Logo/brand 放在顶部，padding=[24,16]
- Navigation items：frame(layout="horizontal", gap=12, alignItems="center", padding=[10,16]) > icon_font + text
- Active item：accent background 或 left border indicator
- nav groups 之间使用 section dividers
- User/settings 放在底部

TOP BAR：

- height=56-64，padding=[0,24]，layout="horizontal"，justifyContent="space_between"
- Left：page title 或 breadcrumbs
- Right：search bar + notification icon + user avatar

METRICS ROW：

- Horizontal layout，包含 3-4 个 stat-cards，每个 width="fill_container"
- 每个 card：icon + metric value（28-36px，bold）+ label（14px，muted）+ optional trend indicator
- padding=[20,24]，gap=8，cornerRadius=12

CHART SECTIONS：

- Cards 包含 header（title + filter/period selector）+ chart area placeholder
- Chart area：带 rounded corners 的 colored rectangle 作为 placeholder
- width="fill_container"，cornerRadius=12

DATA TABLES：

- Table header：background fill，bold text，padding=[12,16]
- Table rows：alternating subtle backgrounds，consistent column widths
- Status badges：pill-shaped，使用 semantic colors（green=active，amber=pending，red=error）
- 所有 cells 使用 width="fill_container"

SPACING：

- Main content padding=[24,24]，gap=16-24
- Cards：padding=[20,24]，gap=12-16
- cards 统一使用 12px cornerRadius
