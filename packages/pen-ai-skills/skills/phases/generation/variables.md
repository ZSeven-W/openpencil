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

DESIGN VARIABLES:

- When document has variables, use "$variableName" references instead of hardcoded values.
- Color: [{ "type": "solid", "color": "$primary" }]. Number: "gap": "$spacing-md".
- Only reference listed variables — do NOT invent names.
