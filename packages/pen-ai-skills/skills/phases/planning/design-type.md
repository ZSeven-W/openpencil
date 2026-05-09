---
name: design-type
description: Design type detection and classification rules
phase: [planning]
trigger: null
priority: 5
budget: 1000
category: base
---

DESIGN TYPE DETECTION:
Classify by the design's PURPOSE — reason about intent, do not keyword-match:

1. Multi-section page — marketing, promotional, or informational content designed to be scrolled (e.g. product sites, portfolios, company pages):
   - Desktop: width=1200, height=0 (scrollable), 6-10 subtasks
   - Structure: navigation - hero - content sections - CTA - footer

2. Single-task screen — functional UI focused on one user task (e.g. authentication, forms, settings, profiles, modals, onboarding):
   - Mobile: width=375, height=812 (fixed viewport), 1-5 subtasks
   - Structure: header + focused content area only, no navigation/hero/footer

3. Data-rich workspace — overview screens with metrics, tables, or management panels (e.g. dashboards, admin consoles, analytics):
   - Desktop: width=1200, height=0, 2-5 subtasks
   - Structure: sidebar or topbar + content panels

WIDTH SELECTION RULES:

- Single-task screens (type 2) - ALWAYS width=375, height=812 (mobile).
- Multi-section pages and data-rich workspaces (types 1 & 3) - width=1200, height=0 (desktop).
- This mapping is mandatory.

MOBILE vs MOCKUP:

- "mobile"/"移动端"/"手机" + screen type (login, profile, settings) = ACTUAL mobile screen (375x812), NOT a desktop page with phone mockup.
- Phone mockups are ONLY for app showcase/marketing sections when the user explicitly asks for a "mockup"/"展示"/"showcase"/"preview".
