---
name: variables
description: Design variable reference rules ($variableName syntax)
phase: [generation]
trigger:
  flags: [hasVariables]
priority: 45
budget: 800
category: base
---

DESIGN VARIABLES:

- With document variables, use "$variableName" refs, not hardcoded values.
- Color: [{ "type": "solid", "color": "$--primary" }]. Number: "gap": "$spacing-2".
- Only reference listed variables — never invent names.

## shadcn token palette (hasSemanticPalette=true)

A document seeded with the semantic palette carries the shadcn-convention
token set, paired Light + Dark under the `Mode` axis. PREFER these over hex
literals when the intent is theme-aware (dark mode, system-follow,
toggleable) — the color tracks `themes.Mode` at paint time.

| Token | Light | Dark | Use for |
|---|---|---|---|
| `$--background` | `#F8FAFC` | `#0F172A` | Page background, skeleton hosts |
| `$--foreground` | `#0F172A` | `#F1F5F9` | Headlines, primary text |
| `$--card` | `#FFFFFF` | `#1E293B` | Card / modal bg |
| `$--card-foreground` | `#0F172A` | `#F1F5F9` | Text on cards |
| `$--popover` | `#FFFFFF` | `#1E293B` | Popover / tooltip bg |
| `$--popover-foreground` | `#0F172A` | `#F1F5F9` | Text in popovers |
| `$--primary` | `#2563EB` | `#60A5FA` | Primary brand, CTA, active pill, focus |
| `$--primary-foreground` | `#FFFFFF` | `#0F172A` | Text on primary |
| `$--secondary` | `#F1F5F9` | `#334155` | Secondary surface (chip / hover) |
| `$--secondary-foreground` | `#0F172A` | `#F1F5F9` | Text on secondary |
| `$--muted` | `#F1F5F9` | `#334155` | Muted bg (input / well) |
| `$--muted-foreground` | `#64748B` | `#94A3B8` | Secondary text, timestamps |
| `$--accent` | `#F3F4F6` | `#475569` | Hover surface (shadcn accent = subtle bg, NOT brand) |
| `$--accent-foreground` | `#0F172A` | `#F1F5F9` | Text on accent surfaces |
| `$--destructive` | `#EF4444` | `#F87171` | Delete actions, error, trending-down |
| `$--destructive-foreground` | `#FFFFFF` | `#0F172A` | Text on destructive fills |
| `$--border` | `#E2E8F0` | `#334155` | Dividers, card border |
| `$--input` | `#E2E8F0` | `#334155` | Input strokes |
| `$--ring` | `#2563EB` | `#60A5FA` | Focus ring |
| `$--scrim` | `#00000080` | `#00000099` | Modal backdrop |

Status colours are SOLID signal fills with a white/near-black foreground
pair (meaning > harmony — never swap them for `$--primary`):

| Token | Light | Dark |
|---|---|---|
| `$--color-success` / `-foreground` | `#10B981` / `#FFFFFF` | `#34D399` / `#0F172A` |
| `$--color-warning` / `-foreground` | `#F59E0B` / `#FFFFFF` | `#FBBF24` / `#0F172A` |
| `$--color-error` / `-foreground` | `#EF4444` / `#FFFFFF` | `#F87171` / `#0F172A` |
| `$--color-info` / `-foreground` | `#3B82F6` / `#FFFFFF` | `#60A5FA` / `#0F172A` |

Sidebar chrome has its own 8-token set: `$--sidebar` (`#FFFFFF`/`#1E293B`),
`$--sidebar-foreground`, `$--sidebar-primary(-foreground)`,
`$--sidebar-accent(-foreground)`, `$--sidebar-border`, `$--sidebar-ring`.
Charts use `$--chart-1..6` (blue/violet/pink/teal/amber/orange). Radii use
`$--radius-none/xs/m/l/pill` (0/4/8/12/999).

Without the seeded palette (default state), fall back to hex literals —
don't emit `$--*` refs blindly, they'd render as raw string text.
