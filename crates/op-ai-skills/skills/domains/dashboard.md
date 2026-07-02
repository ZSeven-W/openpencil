---
name: dashboard
description: Dashboard, admin-panel, and data-table design depth — zone structure, table craft, metric/chart treatment beyond the always-on principles
phase: [generation]
trigger:
  keywords: [dashboard, admin, analytics, data, table, 仪表盘, 后台, 数据表, 报表]
priority: 28
budget: 1800
category: domain
---

DASHBOARD / ADMIN / DATA-TABLE DEPTH

A data-dense product surface. The always-on principles (purpose-first, dominant region, action hierarchy, density, the 5 data states, structural consistency) ALREADY apply — do not restate them. Commit to COMPACT density here. Visual identity (color/type/radius) comes from the selected style guide; these add the structural depth.

ZONE STRUCTURE (adapt — do not force sidebar+table when the purpose differs):

- Root: width=1200-1440, layout="horizontal" — Sidebar + Main.
- Sidebar: width=240-280, height="fill_container", layout="vertical", justifyContent="space_between". FOOTER-SINK CONTRACT (pins the user/account card to the very BOTTOM, not floating mid-rail): the sidebar column MUST be height="fill_container" — a "fit_content" column hugs its content so space_between has no free space to distribute and nothing sinks — AND have EXACTLY TWO children: a TOP group {brand, nav groups} and a BOTTOM group {user/account/settings}. NEVER list brand + nav + footer as flat siblings under space_between (it then spreads ALL of them evenly and the nav floats into the middle). Brand top (padding=[24,16]); nav groups with section labels (12px uppercase, muted, letterSpacing≈1). Nav item: frame(horizontal, gap=12, alignItems="center", padding=[10,16]) > icon_font(18-20) + text(14). Active: accent surface tint OR a left accent bar — never both. icon + label always (never icon-only nav).
- Main: width="fill_container", layout="vertical", padding=[24,32], gap=24-28.
- Top bar: height=56-64, padding=[0,24], horizontal, justifyContent="space_between". Left: page title / breadcrumbs. Right: search + bell + avatar.

METRIC ROW:

- 3-4 stat cards in one horizontal row, each width="fill_container" (equal width). Card: padding=20, gap=8, cornerRadius=8-12. Stack: label(13-14, muted) → value(28-36, bold) → trend chip. Trend uses semantic color: up=success green, down=destructive red — NOT the brand accent. Numbers in the value/trend use tabular/mono figures if the guide defines a mono family.

CHARTS:

- Card with header (title left + period/filter control right) then plot area. The plot is a placeholder: a frame with a DASHED border and a faint axis hint or label — NOT a solid saturated color block (that reads as a generic AI fill). Charts row = 2 equal columns, gap=24. One insight per chart.

DATA TABLES (use only when no predefined Table component exists):

- STRICT hierarchy: Table(frame, vertical) > Row(frame, horizontal, width="fill_container") > Cell(frame) > content. Each CELL is its own frame — NEVER put content directly in a row, or columns won't align. Header + every body row repeat the IDENTICAL cell widths.
- Column width by role: identifier 200-250; email/title "fill_container"; status/badge 100-120; date 120-150; number/amount 90-120; actions 80-100.
- Column alignment by data type: text/labels left; numbers/amounts/dates RIGHT (tabular figures for clean decimal alignment); status badge + row actions centered. Header label alignment matches its column.
- Header row: distinct treatment (subtle fill or bottom border), bold 12-13px, often uppercase muted, padding=[12,16], height fixed.
- Body rows: padding=[12,16] (compact) or [10,16); separate with a 1px bottom divider OR a very subtle alternating row tint — pick ONE, never both. Design the hover and selected row state (tint), not just the default.
- Cell content beyond text: status badge (pill, cornerRadius=full, semantic color: green active / amber pending / red error / muted neutral), avatar+name pair, small action buttons/icons. Identifier cell may stack a primary line + muted secondary line.
- Row actions: trailing cell — 1-2 icon buttons inline (edit/delete) OR a single more-vertical overflow when 3+; do not spray every action across the row.
- Below the table: a footer row with result count + pagination (prev/page-numbers/next). Design the EMPTY state (icon + one-line message + primary CTA) and the LOADING state (skeleton rows mirroring the column widths) — a data table without its empty/loading state is incomplete.
- Responsive: a wide multi-column table does NOT survive a narrow screen — at mobile widths replace the table with a stacked list of CARDS (one card per record, label:value pairs), not a shrunken table.
- No user data → generate realistic dummy values per cell; 4-7 columns is a sane default.

SPACING: cards cornerRadius 8-12 consistently; card padding 20 (16 for dense metric cards); section gap 24-28; align everything to an 8px rhythm.

COMPLETENESS (hard bar): a data table / client list carries at least **6 realistic rows** with VARIED data (distinct names, dates, values, statuses) — 2-3 sample rows read as an unfinished skeleton. A dashboard ships its full section set: KPI row + the primary table/list + at least one secondary section (activity feed, upcoming items, or quick actions).

Apply silently through node structure; never emit these as visible text.
