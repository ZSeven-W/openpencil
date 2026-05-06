---
name: design-type
description: Design type detection and classification rules
phase: [planning]
trigger: null
priority: 5
budget: 1000
category: base
---

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

WIDTH SELECTION RULES：

- Single-task screens（type 2）- 始终 width=375，height=812（mobile）。
- Multi-section pages 和 data-rich workspaces（types 1 & 3）- width=1200，height=0（desktop）。
- 这个映射是强制规则。

MOBILE vs MOCKUP：

- "mobile"/"移动端"/"手机" + screen type（login、profile、settings）= 实际 mobile screen（375x812），不是带 phone mockup 的 desktop page。
- Phone mockups 只用于 app showcase/marketing sections，并且用户明确要求 "mockup"/"展示"/"showcase"/"preview" 时才使用。
