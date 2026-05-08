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
Classify by the design's PURPOSE — reason about intent, do not keyword-match.

FIRST CHECK — single component vs full screen:
A request that names ONE atomic UI piece is a Component (Type 0), not a screen, even if the piece's name overlaps a screen type. Component triggers (any of):

- "X card" / "X 卡片" — profile card, pricing card, stat card, event card, user card
- "X badge", "X chip", "X tag", "X tile", "X label", "X row", "X item"
- "X button", "X toggle", "X switch", "X selector"
- "X modal", "X dialog", "X tooltip", "X popover", "X sheet" (when standalone)
- "X widget", "X panel" when no surrounding screen is implied
- A single visualization: "a chart", "a pie chart", "a stat", "a metric"

A "profile card" is a Component (Type 0), NOT a "profile screen" (Type 2).
A "pricing card" is a Component, NOT a pricing page.
Use Type 2 only when the user clearly asks for a whole screen (e.g. "login screen", "settings page", "profile page", "onboarding flow").

If Type 0:

- width=400, height=0 (auto-expand), 1 subtask
- NO status bar, NO bottom nav, NO page chrome — output is a self-contained component
- DO NOT wrap inside a phone mockup, browser frame, or any device shell

OTHERWISE classify by purpose:

1. Multi-section page — marketing, promotional, or informational content designed to be scrolled (e.g. product sites, portfolios, company pages):
   - Desktop: width=1200, height=0 (scrollable), 6-10 subtasks
   - Structure: navigation - hero - content sections - CTA - footer

2. Single-task screen — full functional SCREEN focused on one user task (e.g. login screen, signup screen, settings page, profile page, full onboarding flow):
   - Mobile: width=375, height=812 (fixed viewport), 1-5 subtasks
   - Structure: header + focused content area only, no navigation/hero/footer
   - NOT a single card/badge/modal — those are Type 0 components

3. Data-rich workspace — overview screens with metrics, tables, or management panels (e.g. dashboards, admin consoles, analytics):
   - Desktop: width=1200, height=0, 2-5 subtasks
   - Structure: sidebar or topbar + content panels

WIDTH SELECTION RULES:

- Type 0 components — width=400, height=0
- Type 2 single-task SCREEN (login screen, profile page) — width=375, height=812
- Types 1 & 3 (multi-section / dashboard) — width=1200, height=0
- This mapping is mandatory.

MOBILE vs MOCKUP:

- "mobile"/"移动端"/"手机" + screen type (login, profile, settings) = ACTUAL mobile screen (375x812), NOT a desktop page with phone mockup.
- Phone mockups are ONLY for app showcase/marketing sections when the user explicitly asks for a "mockup"/"展示"/"showcase"/"preview".
- Components (Type 0) are NEVER wrapped in a phone mockup.
