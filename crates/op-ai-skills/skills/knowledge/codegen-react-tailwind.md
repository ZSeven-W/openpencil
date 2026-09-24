---
name: codegen-react-tailwind
description: React + Tailwind CSS + shadcn/ui export conventions — design-token classes, spacing scale, shadcn component mapping, theme files
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---

# React + Tailwind + shadcn/ui Export

OpenPencil's `react-tailwind` target is a deterministic generator (no model): the same `.op` always yields the same TSX. Follow the same conventions when hand-writing or reviewing code for an OpenPencil design.

## Design tokens → utility classes

- `$--background` / `$--primary` / … fills → `bg-background`, `bg-primary`; text fills → `text-foreground`, `text-muted-foreground`; strokes → `border-border`
- Fill opacity → opacity modifier: `bg-primary/90`
- Literal colours only when no token applies: `bg-[#f8fafc]`, `text-white`
- Tokens bind in `app/globals.css` (Tailwind v4: `:root` / `.dark` blocks + `@theme inline { --color-primary: var(--primary) }`) or `tailwind.config.ts` (v3: `colors: { primary: "var(--primary)" }`)

## Layout and spacing

- `layout: vertical` → `flex flex-col`; `horizontal` (or unset) → `flex`; `none` → `relative grid` with children stacked via `col-start-1 row-start-1`
- Unset `alignItems` is start in OpenPencil (CSS stretches) → always emit `items-start`
- `justifyContent` → `justify-center|end|between|around`; `gap`/`padding` → spacing scale
- Spacing scale first (`p-4` = 16px, `gap-2` = 8px, `h-10` = 40px); arbitrary values (`gap-[13px]`) only when off-scale
- `fill_container`: main axis → `flex-1 min-w-0`; cross axis → `self-stretch`; outside flex → `w-full`
- Fixed width+height inside flex → `shrink-0` (the canvas never squeezes fixed boxes)
- Nodes with explicit `x`/`y` → `absolute left-* top-*`, parent gets `relative`

## Radius and typography

- `rounded-md` 6 · `rounded-lg` 8 · `rounded-xl` 12 · `rounded-2xl` 16; with a `--radius` token, `lg` = `var(--radius)` (shadcn ladder `sm`/`md`/`xl` = −4/−2/+4px)
- `fontSize` → `text-xs|sm|base|lg|xl|2xl…` when on-scale, else `text-[15px]`; `fontWeight` → `font-medium|semibold|bold`; `lineHeight` → `leading-tight|normal|relaxed` or `leading-[1.4]`
- Gradients: `.op` angle + 90° = CSS angle (`.op` 90° = top→bottom = `180deg`)

## shadcn/ui components

- Instances of the built-in shadcn kit become components imported from `@/components/ui/*`: buttons → `<Button variant="outline|secondary|ghost|destructive|link">`, Basic/Stats Card → `Card` + `CardHeader`/`CardTitle`/`CardDescription`/`CardContent`, Badge, Input, Textarea, Checkbox + Label, Switch, RadioGroup, Tabs, Breadcrumb, Alert, Avatar, Separator, Select, Slider, Progress, Skeleton, Table
- Widget nodes map the same way (`text_input` → `<Input>`, `select` → `Select` + `SelectItem`s, `tabs` → `Tabs` + `TabsContent`), authored state as `defaultValue` / `defaultChecked`
- Only map when the instance still has the kit's shape; custom content stays plain markup
- Icons: `icon_font` (lucide) → `lucide-react` `<SearchIcon className="w-4 h-4 text-muted-foreground" />`
