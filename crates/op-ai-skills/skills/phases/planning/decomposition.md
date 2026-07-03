---
name: decomposition
description: Orchestrator task decomposition — splits UI requests into cohesive subtasks
phase: [planning]
trigger: null
priority: 0
budget: 3000
category: base
---

Split a UI request into cohesive subtasks. Each subtask = a meaningful UI section or component group. Output ONLY JSON, start with {.

DESIGN TYPE DETECTION:
Classify by the design's PURPOSE — reason about intent, do not keyword-match.

FIRST CHECK — single component vs full screen:
A request that names ONE atomic UI piece (e.g. "profile card", "pricing card", "stat badge", "X chip", "X tile", "X modal") is a Component (Type 0), not a screen, even when the piece's name overlaps a screen type. A "profile card" is NOT a "profile screen". A "pricing card" is NOT a pricing page. Use Type 2 only when the user clearly asks for a whole screen (e.g. "login screen", "settings page", "profile page").

If Type 0:

- width=400, height=0 (auto-expand), 1 subtask
- NO status bar, NO navigation, NO footer, NO page chrome
- DO NOT wrap inside a phone mockup or device shell

OTHERWISE classify by purpose:

1. Multi-section page — marketing, promotional, or informational content designed to be scrolled (e.g. product sites, portfolios, company pages):
   - Desktop: width=1200, height=0 (scrollable), 6-10 subtasks
   - Structure: navigation - hero - content sections - CTA - footer

2. Single-task SCREEN — full functional screen for one user task (e.g. login screen, signup screen, settings page, profile page):
   - Mobile: width=375, height=812 (fixed viewport), 1-5 subtasks
   - Structure: header + focused content area only, no navigation/hero/footer
   - NOT a single card/badge/modal — those are Type 0 components

3. Data-rich workspace — overview screens with metrics, tables, or management panels (e.g. dashboards, admin consoles, analytics):
   - Desktop: width=1200, height=0, 2-5 subtasks
   - Structure: sidebar or topbar + content panels
   - Sidebar subtasks: a sidebar is a VERTICAL rail (brand block, stacked nav items, footer profile) — NEVER a horizontal navbar, NEVER a hero headline or marketing copy.

CRITICAL — "MOBILE" MEANS MOBILE-SIZED SCREEN, NOT A PHONE MOCKUP:
When the user says "mobile"/"移动端"/"手机" + a screen type (login, profile, settings, etc.), they want a DIRECT mobile-sized screen (375x812) — NOT a desktop landing page containing a phone mockup frame. A "mobile login page" = type 2 (375x812 login screen). Only use phone mockups when the user explicitly asks for a "mockup"/"展示"/"showcase"/"preview" of an app, or when designing a landing page that promotes a mobile app.

FORMAT:
{"rootFrame":{"id":"page","name":"Page","width":1200,"height":0,"layout":"vertical","gap":0,"fill":[{"type":"solid","color":"<bg color from selected style guide, shown after bg: in the guide list>"}]},"styleGuideName":"terminal-minimal-dark","subtasks":[{"id":"nav","label":"Navigation Bar","elements":"logo, nav links (Home, Features, Pricing, Blog), sign-in button, get-started CTA button","region":{"width":1200,"height":72}},{"id":"hero","label":"Hero Section","elements":"headline, subtitle, CTA button, hero illustration or phone mockup","region":{"width":1200,"height":560}},{"id":"features","label":"Feature Cards","elements":"section title, 3 feature cards each with icon + title + description","region":{"width":1200,"height":480}}]}

RULES:

- ELEMENT BOUNDARIES: Each subtask MUST have an "elements" field listing the specific UI elements it contains. Elements must NOT overlap between subtasks — each element belongs to exactly ONE subtask. Example: if "Login Form" has "email input, password input, submit button, forgot-password link", then "Social Login" must NOT repeat the submit button or form inputs.
- STYLE SELECTION: Choose light or dark theme based on user intent. Dark: user mentions dark/cyber/terminal/neon/夜间/暗黑/deep/gaming/noir. Light (default): all other cases — SaaS, marketing, education, e-commerce, productivity, social. Never default to dark unless the content clearly calls for it.
- Detect the design type FIRST, then choose the appropriate structure and subtask count.
- Multi-section pages (type 1): include Navigation Bar as the FIRST subtask, followed by Hero, feature sections, CTA, footer, etc. (6-10 subtasks)
- Sidebar subtasks: a sidebar is a VERTICAL rail (brand block, stacked nav items, footer profile) — NEVER a horizontal navbar, NEVER a hero headline or marketing copy.
- Single-task mobile screens (type 2 — login, signup, profile, settings, a single form/detail view): do NOT include Navigation Bar, Hero, CTA, or footer. Only include the actual UI elements needed (1-5 subtasks).
- Mobile app HOME / feed / main / discover screens (type 2 but MULTI-section — a food/shopping/social/delivery app homepage, a dashboard feed, etc.): plan the sections that genuinely fit THIS product and prompt. VARY the composition per app — do NOT default to the same header + search + categories + featured-banner + two-card-list stack every time; choose a domain-specific concept and section set. The LAST subtask MUST be "Bottom Navigation Bar" (icon + label tabs, active state on the first) — an app home/feed/main screen always has persistent top-level destinations. Omit it ONLY for single-task flows (login, a form, one detail view), never for a home screen. The number and kind of sections should follow the product, not a fixed template.
- FORM INTEGRITY: Keep a form's core elements (inputs + submit button) in the same subtask. Splitting inputs into one subtask and the button into another causes duplicate buttons.
- Combine related elements: "Hero with title + image + CTA" = ONE subtask, not three.
- Each subtask generates a meaningful section (~10-30 nodes). Only split if it would exceed 40 nodes.
- REQUIRED: "styleGuideName" must ALWAYS be included. Pick a name from the available style guides listed by the style-guide-selector skill. If none fit, use the closest match. The system will load the full style specifications automatically.
- CJK FONT RULE: If the user's request is in Chinese/Japanese/Korean or the product targets CJK audiences, the styleGuide fonts MUST use CJK-compatible fonts: heading="Noto Sans SC" (Chinese) / "Noto Sans JP" (Japanese) / "Noto Sans KR" (Korean), body="Inter". NEVER use "Space Grotesk" or "Manrope" as heading font for CJK content — they have no CJK character support.
- Root frame fill must use the background color from the selected style guide. Each guide in the list shows its bg color (e.g. bg:#0A0F1C). Use that exact hex value for the rootFrame fill color.
- Root frame gap: Landing pages with distinct section backgrounds - gap=0 (sections flush). Mobile screens and dashboards - gap=16-24 (breathing room between sections). Always include "gap" in rootFrame.
- Root frame height: Mobile (width=375) - set height=812 (fixed viewport). Desktop (width=1200) - set height=0 (auto-expands as sections are generated).
- Landing page height hints: nav 64-80px, hero 500-600px, feature sections 400-600px, testimonials 300-400px, CTA 200-300px, footer 200-300px.
- App screen height hints: status bar is pre-inserted (62px, do NOT plan a "Status Bar" section). Header 56-64px, form fields 48-56px each, buttons 48px, spacing 16-24px.
- If a section is about "App截图"/"XX截图"/"screenshot"/"mockup", plan it as a phone mockup placeholder block, not a detailed mini-app reconstruction.
- For landing pages: navigation sections should preserve good horizontal balance, links evenly distributed in the center group.
- Regions tile to fill rootFrame. vertical = top-to-bottom.
- Mobile: 375x812 (both width AND height are fixed). Desktop: 1200x0 (width fixed, height auto-expands).
- WIDTH SELECTION: Type 0 components - width=400, height=0. Type 2 single-task SCREENS (login screen, profile page, settings page) - width=375, height=812 (mobile). Multi-section pages and data-rich workspaces (types 1 & 3) - width=1200, height=0 (desktop). A "profile card" is Type 0 (width=400), NOT Type 2. This is mandatory.
- MULTI-SCREEN APPS: When the request involves multiple distinct screens/pages (e.g. "登录页+个人中心", "login and profile"), add "screen":"<name>" to each subtask to group sections that belong to the same page. Use a concise page name (e.g. "登录", "Profile"). Subtasks sharing the same "screen" are placed in one root frame. Single-screen requests don't need "screen". Example: [{"id":"brand","label":"Brand Area","screen":"Login","region":{...}},{"id":"form","label":"Login Form","screen":"Login","region":{...}},{"id":"card","label":"User Card","screen":"Profile","region":{...}}]
- NO explanation. NO markdown. NO tool calls. NO function calls. NO [TOOL_CALL]. JUST the JSON object. Start with {.
