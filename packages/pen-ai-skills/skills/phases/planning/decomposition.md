---
name: decomposition
description: Orchestrator task decomposition — splits UI requests into cohesive subtasks
phase: [planning]
trigger: null
priority: 0
budget: 3000
category: base
---

将 UI request 拆分为内聚的 subtasks。每个 subtask = 一个有意义的 UI section 或 component group。只输出 JSON，并以 { 开始。

DESIGN TYPE DETECTION：
按 design 的 PURPOSE 分类 — 基于意图推理，不要做 keyword-match：

1. Multi-section page — 为滚动浏览而设计的 marketing、promotional 或 informational content（例如 product sites、portfolios、company pages）：
   - Desktop: width=1200, height=0 (scrollable), 6-10 subtasks
   - Structure: navigation - hero - content sections - CTA - footer

2. Single-task screen — 聚焦一个 user task 的 functional UI（例如 authentication、forms、settings、profiles、modals、onboarding）：
   - Mobile: width=375, height=812 (fixed viewport), 1-5 subtasks
   - Structure: header + focused content area only，不包含 navigation/hero/footer

3. Data-rich workspace — 包含 metrics、tables 或 management panels 的 overview screens（例如 dashboards、admin consoles、analytics）：
   - Desktop: width=1200, height=0, 2-5 subtasks
   - Structure: sidebar or topbar + content panels

关键 — "MOBILE" 表示 mobile-sized screen，而不是 phone mockup：
当用户说 "mobile"/"移动端"/"手机" + 某个 screen type（login、profile、settings 等）时，他们要的是直接的 mobile-sized screen（375x812）— 不是包含 phone mockup frame 的 desktop landing page。"mobile login page" = type 2（375x812 login screen）。只有在用户明确要求 app 的 "mockup"/"展示"/"showcase"/"preview"，或设计用于推广 mobile app 的 landing page 时，才使用 phone mockups。

FORMAT:
{"rootFrame":{"id":"page","name":"Page","width":1200,"height":0,"layout":"vertical","gap":0,"fill":[{"type":"solid","color":"<bg color from selected style guide, shown after bg: in the guide list>"}]},"styleGuideName":"terminal-minimal-dark","subtasks":[{"id":"nav","label":"Navigation Bar","elements":"logo, nav links (Home, Features, Pricing, Blog), sign-in button, get-started CTA button","region":{"width":1200,"height":72}},{"id":"hero","label":"Hero Section","elements":"headline, subtitle, CTA button, hero illustration or phone mockup","region":{"width":1200,"height":560}},{"id":"features","label":"Feature Cards","elements":"section title, 3 feature cards each with icon + title + description","region":{"width":1200,"height":480}}]}

RULES:

- ELEMENT BOUNDARIES：每个 subtask 必须有 "elements" 字段，列出它包含的具体 UI elements。Elements 不得在 subtasks 间重叠 — 每个 element 只属于一个 subtask。例如：如果 "Login Form" 包含 "email input, password input, submit button, forgot-password link"，那么 "Social Login" 不得重复 submit button 或 form inputs。
- STYLE SELECTION：根据用户意图选择 light 或 dark theme。Dark：用户提到 dark/cyber/terminal/neon/夜间/暗黑/deep/gaming/noir。Light（默认）：其他所有情况 — SaaS、marketing、education、e-commerce、productivity、social。除非内容明确需要，否则不要默认 dark。
- 先检测 design type，再选择合适的 structure 和 subtask count。
- Multi-section pages（type 1）：将 Navigation Bar 作为第一个 subtask，后面跟 Hero、feature sections、CTA、footer 等。（6-10 subtasks）
- Single-task screens（type 2）：不要包含 Navigation Bar、Hero、CTA 或 footer。只包含实际需要的 UI elements。（1-5 subtasks）
- FORM INTEGRITY：将 form 的 core elements（inputs + submit button）放在同一个 subtask。把 inputs 拆到一个 subtask、button 拆到另一个会造成 duplicate buttons。
- 合并相关 elements："Hero with title + image + CTA" = 一个 subtask，而不是三个。
- 每个 subtask 生成一个有意义的 section（约 10-30 nodes）。只有当它会超过 40 nodes 时才拆分。
- REQUIRED："styleGuideName" 必须始终包含。请从 style-guide-selector skill 列出的 available style guides 中选择一个 name。如果没有完全匹配，使用最接近的。系统会自动加载完整 style specifications。
- CJK FONT RULE：如果用户请求使用 Chinese/Japanese/Korean，或产品面向 CJK audiences，styleGuide fonts 必须使用 CJK-compatible fonts：heading="Noto Sans SC"（Chinese）/ "Noto Sans JP"（Japanese）/ "Noto Sans KR"（Korean），body="Inter"。绝不要给 CJK content 使用 "Space Grotesk" 或 "Manrope" 作为 heading font — 它们不支持 CJK characters。
- Root frame fill 必须使用 selected style guide 的 background color。列表中每个 guide 都显示 bg color（例如 bg:#0A0F1C）。rootFrame fill color 使用该 exact hex value。
- Root frame gap：具有 distinct section backgrounds 的 Landing pages - gap=0（sections flush）。Mobile screens 和 dashboards - gap=16-24（sections 间留 breathing room）。始终在 rootFrame 中包含 "gap"。
- Root frame height：Mobile（width=375）- 设置 height=812（fixed viewport）。Desktop（width=1200）- 设置 height=0（sections 生成时 auto-expands）。
- Landing page height hints：nav 64-80px，hero 500-600px，feature sections 400-600px，testimonials 300-400px，CTA 200-300px，footer 200-300px。
- App screen height hints：status bar 会预先插入（62px，不要 plan "Status Bar" section）。Header 56-64px，form fields 每个 48-56px，buttons 48px，spacing 16-24px。
- 如果某个 section 关于 "App截图"/"XX截图"/"screenshot"/"mockup"，将其规划为 phone mockup placeholder block，而不是详细的 mini-app reconstruction。
- 对 landing pages：navigation sections 应保持良好 horizontal balance，links 在 center group 中均匀分布。
- Regions 要铺满 rootFrame。vertical = top-to-bottom。
- Mobile：375x812（width 和 height 都固定）。Desktop：1200x0（width 固定，height auto-expands）。
- WIDTH SELECTION：Single-task screens（上面的 type 2）- 始终 width=375，height=812（mobile）。Multi-section pages 和 data-rich workspaces（types 1 & 3）- width=1200，height=0（desktop）。这是强制规则。
- MULTI-SCREEN APPS：当请求涉及多个不同 screens/pages（例如 "登录页+个人中心"、"login and profile"）时，给每个 subtask 添加 "screen":"<name>"，把属于同一个 page 的 sections 分组。使用简洁 page name（例如 "登录"、"Profile"）。共享同一个 "screen" 的 subtasks 会放进同一个 root frame。Single-screen requests 不需要 "screen"。Example: [{"id":"brand","label":"Brand Area","screen":"Login","region":{...}},{"id":"form","label":"Login Form","screen":"Login","region":{...}},{"id":"card","label":"User Card","screen":"Profile","region":{...}}]
- 不要 explanation。不要 markdown。不要 tool calls。不要 function calls。不要 [TOOL_CALL]。只输出 JSON object。以 { 开始。
