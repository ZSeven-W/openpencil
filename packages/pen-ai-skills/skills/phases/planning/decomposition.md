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

CRITICAL — "MOBILE" MEANS MOBILE-SIZED SCREEN, NOT A PHONE MOCKUP:
When the user says "mobile"/"移动端"/"手机" + a screen type (login, profile, settings, etc.), they want a DIRECT mobile-sized screen (375x812) — NOT a desktop landing page containing a phone mockup frame. A "mobile login page" = type 2 (375x812 login screen). Only use phone mockups when the user explicitly asks for a "mockup"/"展示"/"showcase"/"preview" of an app, or when designing a landing page that promotes a mobile app.

FORMAT:
{"rootFrame":{"id":"page","name":"Page","width":1200,"height":0,"layout":"vertical","gap":0,"fill":[{"type":"solid","color":"#F8FAFC"}]},"styleGuide":{"palette":{"background":"#F8FAFC","surface":"#FFFFFF","text":"#0F172A","secondary":"#64748B","accent":"#2563EB","accent2":"#0EA5E9","border":"#E2E8F0"},"fonts":{"heading":"Space Grotesk","body":"Inter"},"aesthetic":"clean modern with blue accents"},"subtasks":[{"id":"nav","label":"Navigation Bar","elements":"logo, nav links (Home, Features, Pricing, Blog), sign-in button, get-started CTA button","region":{"width":1200,"height":72}},{"id":"hero","label":"Hero Section","elements":"headline, subtitle, CTA button, hero illustration or phone mockup","region":{"width":1200,"height":560}},{"id":"features","label":"Feature Cards","elements":"section title, 3 feature cards each with icon + title + description","region":{"width":1200,"height":480}}]}

RULES:
- ELEMENT BOUNDARIES: Each subtask MUST have an "elements" field listing the specific UI elements it contains. Elements must NOT overlap between subtasks — each element belongs to exactly ONE subtask. Example: if "Login Form" has "email input, password input, submit button, forgot-password link", then "Social Login" must NOT repeat the submit button or form inputs.
- STYLE SELECTION: Choose light or dark theme based on user intent. Dark: user mentions dark/cyber/terminal/neon/夜间/暗黑/deep/gaming/noir. Light (default): all other cases — SaaS, marketing, education, e-commerce, productivity, social. Never default to dark unless the content clearly calls for it.
- Detect the design type FIRST, then choose the appropriate structure and subtask count.
- Multi-section pages (type 1): include Navigation Bar as the FIRST subtask, followed by Hero, feature sections, CTA, footer, etc. (6-10 subtasks)
- Single-task screens (type 2): do NOT include Navigation Bar, Hero, CTA, or footer. Only include the actual UI elements needed (1-5 subtasks).
- FORM INTEGRITY: Keep a form's core elements (inputs + submit button) in the same subtask. Splitting inputs into one subtask and the button into another causes duplicate buttons.
- Combine related elements: "Hero with title + image + CTA" = ONE subtask, not three.
- Each subtask generates a meaningful section (~10-30 nodes). Only split if it would exceed 40 nodes.
- REQUIRED: "styleGuide" must ALWAYS be included. Choose a distinctive visual direction (palette, fonts, aesthetic) that matches the product personality and target audience. Never use generic/default colors — each design should have its own identity.
- CJK FONT RULE: If the user's request is in Chinese/Japanese/Korean or the product targets CJK audiences, the styleGuide fonts MUST use CJK-compatible fonts: heading="Noto Sans SC" (Chinese) / "Noto Sans JP" (Japanese) / "Noto Sans KR" (Korean), body="Inter". NEVER use "Space Grotesk" or "Manrope" as heading font for CJK content — they have no CJK character support.
- Root frame fill must use the styleGuide palette background color.
- Root frame gap: Landing pages with distinct section backgrounds - gap=0 (sections flush). Mobile screens and dashboards - gap=16-24 (breathing room between sections). Always include "gap" in rootFrame.
- Root frame height: Mobile (width=375) - set height=812 (fixed viewport). Desktop (width=1200) - set height=0 (auto-expands as sections are generated).
- Landing page height hints: nav 64-80px, hero 500-600px, feature sections 400-600px, testimonials 300-400px, CTA 200-300px, footer 200-300px.
- App screen height hints: status bar 44px, header 56-64px, form fields 48-56px each, buttons 48px, spacing 16-24px.
- If a section is about "App截图"/"XX截图"/"screenshot"/"mockup", plan it as a phone mockup placeholder block, not a detailed mini-app reconstruction.
- For landing pages: navigation sections should preserve good horizontal balance, links evenly distributed in the center group.
- Regions tile to fill rootFrame. vertical = top-to-bottom.
- Mobile: 375x812 (both width AND height are fixed). Desktop: 1200x0 (width fixed, height auto-expands).
- WIDTH SELECTION: Single-task screens (type 2 above) - ALWAYS width=375, height=812 (mobile). Multi-section pages and data-rich workspaces (types 1 & 3) - width=1200, height=0 (desktop). This is mandatory.
- MULTI-SCREEN APPS: When the request involves multiple distinct screens/pages (e.g. "登录页+个人中心", "login and profile"), add "screen":"<name>" to each subtask to group sections that belong to the same page. Use a concise page name (e.g. "登录", "Profile"). Subtasks sharing the same "screen" are placed in one root frame. Single-screen requests don't need "screen". Example: [{"id":"brand","label":"Brand Area","screen":"Login","region":{...}},{"id":"form","label":"Login Form","screen":"Login","region":{...}},{"id":"card","label":"User Card","screen":"Profile","region":{...}}]
- NO explanation. NO markdown. NO tool calls. NO function calls. NO [TOOL_CALL]. JUST the JSON object. Start with {.
