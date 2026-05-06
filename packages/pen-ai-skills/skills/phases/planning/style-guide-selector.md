---
name: style-guide-selector
description: Select a pre-built visual style guide based on user request
phase: [planning]
trigger: null
priority: 3
budget: 500
category: base
---

## Style Guide Selection

你可以访问一组 pre-built visual style guides。请根据用户请求选择一个。

Available style guides：
{{availableStyleGuides}}

### Selection Rules

1. 如果用户明确点名某种 style（例如 "brutalist"、"minimal"、"terminal"），按 name 或 primary tag 匹配。
2. 否则，从请求中推断 3-5 个 tags：
   - Platform: "mobile app" → mobile, "dashboard" → webapp, "landing page" → landing-page
   - Visual: "clean" → minimal, "dark" → dark-mode, "luxurious" → elegant + luxury
   - Industry: "developer tool" → developer + monospace, "finance app" → fintech
3. 通过 tag overlap 选择最匹配的 style guide。
4. 在 plan output 中用 `styleGuideName` 包含被选中的 style guide name。
5. 如果没有 guide 匹配良好，省略 `styleGuideName`，系统会使用 defaults。
