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

## Semantic palette (14 tokens, when hasSemanticPalette=true)

A document that has been seeded with `applySemanticPalette(doc)`
carries these 14 tokens with paired Light + Dark values under the
`Mode` theme axis. PREFER these refs over hex literals when the
user's intent is theme-aware (dark-mode design, system-follow app,
user-toggleable theme), because the rendered color tracks
`themes.Mode` at paint time.

| Token                  | Light       | Dark        | Use for                              |
| ---------------------- | ----------- | ----------- | ------------------------------------ |
| `$color-surface`       | `#FFFFFF`   | `#1E293B`   | Primary card / modal / tooltip bg    |
| `$color-surface-2`     | `#F1F5F9`   | `#334155`   | Secondary bg (chip / input / hover)  |
| `$color-surface-3`     | `#F3F4F6`   | `#475569`   | Tertiary bg (pressed / nested)       |
| `$color-bg-deep`       | `#F8FAFC`   | `#0F172A`   | Page background, skeleton hosts      |
| `$color-border`        | `#E2E8F0`   | `#334155`   | Dividers, input strokes, card border |
| `$color-border-strong` | `#CBD5E1`   | `#475569`   | Dashed chart-placeholder border      |
| `$color-text-primary`  | `#0F172A`   | `#F1F5F9`   | Headlines, active page numbers       |
| `$color-text-body`     | `#334155`   | `#CBD5E1`   | Body paragraphs, nav labels          |
| `$color-text-muted`    | `#64748B`   | `#94A3B8`   | Secondary text, timestamps           |
| `$color-text-subtle`   | `#94A3B8`   | `#64748B`   | Tertiary text, disabled states       |
| `$color-accent`        | `#2563EB`   | `#60A5FA`   | Primary brand, active pill, focus    |
| `$color-destructive`   | `#EF4444`   | `#F87171`   | Delete actions, error, trending-down |
| `$color-success`       | `#10B981`   | `#34D399`   | Trending-up, online status           |
| `$color-scrim`         | `#00000080` | `#00000099` | Modal backdrop                       |

When document does NOT have the semantic palette (default
`createEmptyDocument()` state), fall back to hex literals. Do NOT
emit `$color-*` refs blindly — they'd resolve to undefined and
render as raw string text.

Semantic colors override theme: `$color-success` stays green in
both light and dark modes (semantic meaning > visual harmony). Do
not swap `$color-success` for `$color-accent` "because the app is
dark-themed" — those carry different meanings.
