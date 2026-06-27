---
name: web-app
description: Functional web-app product depth — product-UI laws beyond the always-on product-principles (CRM, analytics, admin, tools, SaaS)
phase: [generation]
trigger:
  keywords: [web app, webapp, saas, crm, admin, console, internal tool, product, workspace, app interface, 后台, 控制台, 管理后台, 应用]
priority: 30
budget: 1400
category: domain
---

WEB APP — FUNCTIONAL PRODUCT DEPTH

You are designing a FUNCTIONAL PRODUCT interface, not a marketing page. The always-on product-principles (purpose-first, dominant region, action hierarchy, entity integrity, density, constraint-over-decoration, structural consistency, system-status states) ALREADY apply — do not restate them. Visual identity (color, type, radius, shadow) comes from the SELECTED STYLE GUIDE; concrete dashboard/sidebar layout lives in the `dashboard` domain. These laws add the product DEPTH on top:

1. UNDERSTANDABILITY — labels are explicit; icons never replace essential text (icon + label on primary nav/actions); current state is always visible. If a control needs guessing, redesign it.
2. PROGRESSIVE DISCLOSURE — show essentials first; put advanced controls behind context (detail views, expanders, overflow). Complexity is allowed; confusion is not.
3. RECOGNITION OVER RECALL — keep controls in consistent, predictable positions; surface relevant actions in context; never rely on the user remembering a prior state.
4. SPATIAL LOGIC — one dominant layout axis per screen; prefer two structural zones before three; separate zones with whitespace, not decorative dividers; avoid nested scroll regions.
5. FEEDBACK — every interactive control shows its states (hover / active / selected / disabled present in the design); destructive actions get a confirmation step; no interaction is a dead end.
6. RESPONSIVE HIERARCHY — hierarchy survives width: narrow = one dominant column with secondary content stacked; wide = multi-zone permitted. Clarity scales, never breaks.
7. SCALABILITY — more rows, items, or features must EXTEND the existing pattern, not collapse the hierarchy. Design both the populated and the overflowing case.
8. ADAPTATION — infer the product type from the request, then decide the dominant region, the primary action, the density, and the disclosure depth. Do NOT default to sidebar+table unless the purpose calls for it. Structure emerges from utility.

CONCRETE CONTRACTS — turn the laws above into emittable structure (these are what a generated screen must actually contain):

ZONE ARCHITECTURE — pick ONE dominant axis per screen, then commit:
- One dominant region (the work surface) + at most one subordinate zone. Prefer 2 zones before reaching for 3. The dominant zone gets `width=fill_container`; chrome (nav/sidebar/toolbar) gets a fixed width or height.
- Common skeletons: rail+work (sidebar 240-280 + main fill); topbar+work (bar height 56-64 + main fill); list+detail (master 320-400 + detail fill); single-column (centered work ≤960 on a tinted page bg).
- No nested scroll containers: one scroll region per axis. Separate zones with whitespace and a single hairline border, not stacked dividers.

ENTITY SURFACE — any row/card/header that represents a record (user, order, file, ticket) must carry, in this read order: name (strongest weight) → status (semantic badge: green active / amber pending / red error / muted neutral) → 1-3 key metadata fields (muted, 13-14px) → primary action (one button; rare actions in an overflow `more-vertical` icon-button). Never an entity with no visible status and no action.

DATA-SURFACE STATES — every list, table, or fetched panel is designed for its real states, not just the happy path. At minimum emit the populated case AND one of: empty state (icon + one-line reason + primary CTA to create the first item) when the surface can be empty; or a skeleton row when latency is the story. Error and restricted states reuse the empty-state frame with a distinct icon + message. No blank panel.

DENSITY → METRICS (one mode per screen, do not mix):
- Compact (tables, dashboards, monitoring): row/cell padding=[8,12], gap 8, body 13-14, controls height 32-36.
- Medium (default product screens): padding=[12,16], gap 12-16, body 14-15, controls height 40-44.
- Airy (settings, onboarding, wizards): padding=[20,24]+, gap 20-24, body 16, controls height 48, generous section gaps 32-48.

RESPONSIVE STRUCTURE — hierarchy must survive width:
- Narrow: collapse to the single dominant column; the subordinate zone becomes a stacked section or a sheet, never a squeezed second column. One owner of horizontal padding; inner sections don't re-add gutters.
- Wide: the multi-zone skeleton and higher density are permitted. The primary action and dominant region stay in the same relative place across widths (recognition over recall).

Output discipline: functional product UI only — no hero sections, no marketing copy. Reach for a container (card/panel) only when it groups related content for a real structural reason; structural wrappers (page bg, section, top bar) stay transparent, not filled white cards. Apply these silently through node structure.
