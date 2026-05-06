---
name: variables
description: Design variable reference rules ($variableName syntax)
phase: [generation]
trigger:
  flags: [hasVariables]
priority: 45
budget: 500
category: base
---

DESIGN VARIABLES：

- 当 document 有 variables 时，使用 "$variableName" references，而不是 hardcoded values。
- Color: [{ "type": "solid", "color": "$primary" }]。Number: "gap": "$spacing-md"。
- 只引用已列出的 variables — 不要发明 names。
