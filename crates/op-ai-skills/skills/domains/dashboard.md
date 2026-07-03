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
- Sidebar: width=240-280, height="fill_container", layout="vertical", justifyContent="space_between". HARD ARCHETYPE RULE: a sidebar is a VERTICAL rail (brand block, stacked nav items, footer profile) — NEVER a horizontal navbar, NEVER a hero headline or marketing copy. FOOTER-SINK CONTRACT (pins the user/account card to the very BOTTOM, not floating mid-rail): the sidebar column MUST be height="fill_container" — a "fit_content" column hugs its content so space_between has no free space to distribute and nothing sinks — AND have EXACTLY TWO children: a TOP group {brand, nav groups} and a BOTTOM group {user/account/settings}. NEVER list brand + nav + footer as flat siblings under space_between (it then spreads ALL of them evenly and the nav floats into the middle). Brand top (padding=[24,16]); nav groups with section labels (12px uppercase, muted, letterSpacing≈1). Nav item: frame(horizontal, gap=12, alignItems="center", padding=[10,16]) > icon_font(18-20) + text(14). Active: accent surface tint OR a left accent bar — never both. icon + label always (never icon-only nav). ONE rhythm between brand and nav: a single TOP-group gap of 32-48 — never stack brand padding-bottom + a divider row + the group gap (each interval re-applies the gap; measured 121px of dead rail vs the intended 48). If you insert a divider between brand and nav, drop the group gap to 12-16.
- Main: width="fill_container", layout="vertical", padding=[24,32], gap=24-28.
- Top bar: height=56-64, padding=[0,24], horizontal, justifyContent="space_between". Left: page title / breadcrumbs. Right: search + bell + avatar.

METRIC ROW:

- 3-4 stat cards in one horizontal row, each width="fill_container" (equal width). Card: padding=20, gap=8, cornerRadius=8-12. Stack: label(13-14, muted) → value(28-36, bold) → trend chip. Trend uses semantic color: up=success green, down=destructive red — NOT the brand accent. Numbers in the value/trend use tabular/mono figures if the guide defines a mono family.

CHARTS:

- Card with header (title left + period/filter control right) then plot area. The plot is a placeholder: a frame with a DASHED border and a faint axis hint or label — NOT a solid saturated color block (that reads as a generic AI fill). Charts row = 2 equal columns, gap=24. One insight per chart.
- DONUT / RING anatomy (arc ellipses): ALL arc segments + the center label live in ONE wrapper frame with layout="none" and explicit x/y so they stack CONCENTRICALLY — arc ellipses left in a flex row lay out side by side and the donut falls apart. Wrapper = square (size = outer diameter); each arc x/y = (wrapper - arc)/2; track arc before progress arc. Center value text is centered the same way.
- Legend rows: dot(8) + label + value with gap 6-8 between label and value — never butt "Solar" against "44%".
- Chart tooltip (the floating value callout): a SURFACE — fill $color-surface + hairline stroke + cornerRadius 6 + padding [8,10]; a fill-less tooltip paints bare text over the plot lines.
- Status pill in a table cell: the text lives INSIDE the pill frame (pill > text), never beside it — a childless tinted pill collapses to a smear with the label floating outside.

DATA TABLES (use only when no predefined Table component exists):

- STRICT hierarchy: Table(frame, vertical) > Row(frame, horizontal, width="fill_container") > Cell(frame) > content. Each CELL is its own frame — NEVER put content directly in a row, or columns won't align. Header + every body row repeat the IDENTICAL cell widths.
- Column width by role: identifier 200-250; email/title "fill_container"; status/badge 100-120; date 120-150; number/amount 90-120; actions 80-100.
- Column alignment by data type: text/labels left; numbers/amounts/dates RIGHT (tabular figures for clean decimal alignment); status badge + row actions centered. Header label alignment matches its column.
- Header row: distinct treatment (subtle fill or bottom border), bold 12-13px, often uppercase muted, padding=[12,16], height fixed.
- Body rows: padding=[12,16] (compact) or [10,16); separate with a 1px bottom divider (stroke={"thickness":{"bottom":1}}, hairline color, omit on the LAST row) OR a very subtle alternating row tint — pick ONE, never both. Design the hover and selected row state (tint), not just the default.
- ROWS TOUCH: the Table frame itself has gap=0 and padding=0 — rhythm comes ONLY from each row's own padding + the hairline/tint. NEVER put a gap between rows (gap=16 turns the table into floating stripes with page background bleeding through) and NEVER wrap rows in a padded container (the row's horizontal padding IS the table inset). Round the TABLE frame (cornerRadius 8-12 + clipContent) instead of rounding rows.
- Cell content beyond text: status badge (pill, cornerRadius=full, semantic color: green active / amber pending / red error / muted neutral), avatar+name pair, small action buttons/icons. Identifier cell may stack a primary line + muted secondary line.
- Row actions: trailing cell — 1-2 icon buttons inline (edit/delete) OR a single more-vertical overflow when 3+; do not spray every action across the row.
- Below the table: a footer row with result count + pagination (prev/page-numbers/next). Design the EMPTY state (icon + one-line message + primary CTA) and the LOADING state (skeleton rows mirroring the column widths) — a data table without its empty/loading state is incomplete.
- Responsive: a wide multi-column table does NOT survive a narrow screen — at mobile widths replace the table with a stacked list of CARDS (one card per record, label:value pairs), not a shrunken table.
- No user data → generate realistic dummy values per cell; 4-7 columns is a sane default — but ONLY when the table owns the full content width. In a master-detail split (list pane + detail panel), the LIST PANE gets compact rows (avatar + name/email stack + one meta + status), NEVER a 5-6 column table: five text columns need ≥600px of cell space and a half-width pane cannot fit them.

SPACING (reference-measured constants — copy these, do not improvise):

- Content zone: section gap 28-48 by density (airy/luxury 48, balanced 32, dense 28); content padding [32-48, 40-56]. Inside a section, title→body gap 16-24.
- KPI/metric cards: row gap 20; card = vertical, padding 24-28, gap 20, width fill_container, height fill_container (NOT fit_content — fill lets the row equalize card heights when one label wraps). Card surface: hairline stroke (1px, one step above background) OR a fill — never both, never thick borders. Value 34-40px display font (letterSpacing -1), label 11-12px uppercase muted, change row gap 8. The change chip goes on its OWN line BELOW the value (card vertical: header row / value / change) — never beside a 34-40px value in a ~200px card, it cannot fit; the chip hugs (fit_content), never fill_container.
- Nav count badge (the "12" on a menu item): pill — cornerRadius=full, padding [2,8], 11px semibold, accent tint fill; never a bare square.
- Section wrapper rows (the metrics strip, card rows, chart+list splits) carry NO padding of their own — the content column's padding is the single horizontal inset; padding belongs to CARDS. A padded transparent wrapper misaligns section edges and starves its children.
- Filter chips / small buttons: padding [10,16], icon 14 + text 12 weight 500, inner gap 10, chips row gap 12.
- List (non-table) items: padding [20,24] + 1px bottom hairline, container hairline frame; list section gap 24.
- Pagination: 32x32 squares, gap 4, active page accent fill.
- Cards cornerRadius 8-12 consistently; align everything to an 8px rhythm.

COMPLETENESS (hard bar): a data table / client list carries at least **6 realistic rows** with VARIED data (distinct names, dates, values, statuses) — 2-3 sample rows read as an unfinished skeleton. A dashboard ships its full section set: KPI row + the primary table/list + at least one secondary section (activity feed, upcoming items, or quick actions).

Apply silently through node structure; never emit these as visible text.
