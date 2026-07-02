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

DASHBOARD DESIGN PATTERNS:

STRUCTURE:

- Root frame: width=1200, height=0, layout="horizontal" (sidebar + main content)
- Sidebar: width=240-280, height="fill_container", layout="vertical", dark or surface fill
- Main content: width="fill_container", layout="vertical", gap=16-24

SIDEBAR:

- Logo/brand at top, padding=[24,16]
- Navigation items: frame(layout="horizontal", gap=12, alignItems="center", padding=[10,16]) > icon_font + text
- Active item: accent background or left border indicator
- Section dividers between nav groups
- User/settings at bottom

TOP BAR:

- height=56-64, padding=[0,24], layout="horizontal", justifyContent="space_between"
- Left: page title or breadcrumbs
- Right: search bar + notification icon + user avatar

METRICS ROW:

- Horizontal layout with 3-4 stat-cards, each width="fill_container"
- Each card: icon + metric value (28-36px, bold) + label (14px, muted) + optional trend indicator
- padding=[20,24], gap=8, cornerRadius=12

CHART SECTIONS:

- Cards with header (title + filter/period selector) + chart area placeholder
- Chart area: colored rectangle with rounded corners as placeholder
- width="fill_container", cornerRadius=12

DATA TABLES:

- Table header: background fill, bold text, padding=[12,16]
- Table rows: alternating subtle backgrounds, consistent column widths
- Status badges: pill-shaped with semantic colors (green=active, amber=pending, red=error)
- All cells use width="fill_container"

SPACING:

- Main content padding=[24,24], gap=16-24
- Cards: padding=[20,24], gap=12-16
- Consistent 12px cornerRadius across cards
