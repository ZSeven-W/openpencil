# Jeans: Opinionated Content Creation Workflow

**Date:** 2026-03-10
**Status:** Brainstorm
**Author:** Shawn + Claude

## What We're Building

Jeans layers an opinionated content creation workflow on top of OpenPencil's canvas engine. Three pillars:

### 1. Vibe Kit — Design Primitive System

A complete, swappable visual identity system. Templates pull ALL styling from the Vibe Kit — swapping kits completely restyles everything, like a WordPress theme for social content.

**Primitive categories:**

| Category | Examples |
|----------|---------|
| **Typography stack** | Semantic styles (H1-H6, body, caption) + editorial styles (eyebrow, pull quote, kicker, subhead) |
| **Color palettes** | Primary, secondary, accent, neutral, semantic (success/warning/error) |
| **Graphic sets** | Open-source icon libraries (Lucide, etc.) |
| **Texture palettes** | Background textures, patterns, gradients |
| **LUT filters** | Color grading presets for images/video |
| **SFX** | Sound effects for video/animation content |
| **Animations** | Motion presets (fade, slide, scale, bounce — extends Phase 1 animation engine) |
| **Transitions** | Between-slide/page transitions for carousels and sequences |
| **Strokes** | Border styles, decorative lines, dividers |
| **Size tokens** | Base sizing scale, component sizes |
| **Space tokens** | Margin/padding/gap scale |

Starting with LinkedIn content formats, designed to be format-agnostic (Instagram, X, newsletters, etc.).

### 2. Token Extraction Skill

A Claude Code skill that:
- Scrapes any website and extracts design tokens (colors, fonts, spacing, shadows, radii, textures)
- Maps extracted tokens to the Vibe Kit's variable system
- **Brand consistency:** Extract your own site to match your identity
- **Style inspiration:** Borrow the vibe from any site (Stripe, Linear, etc.)
- Restyles all templates instantly by swapping the Vibe Kit

### 3. Fluid Grid System

Web-based layout logic that enables automatic content reflow across formats:

- Content auto-reflows when switching canvas format (1080x1080 square -> 1080x1920 story -> 1200x628 link preview)
- Grid is fluid — blocks resize and reposition based on available space
- **Always fits everything** — the grid scales and restacks to fit all content; nothing gets hidden
- Makes it trivial to repurpose content across platforms without manual re-layout

## Why This Approach

**Approach A: PenNode Extension** — Build the Vibe Kit as an extension of the existing design variables system, and templates as opinionated PenNodes.

Rejected alternatives:
- **Separate Content Layer** — Adds a second model to sync; YAGNI for a solo-creator-focused tool.
- **Web Component Renderer** — True CSS reflow but massive architectural divergence; breaks the freeform escape hatch.

Rationale:
- The existing `VariableDefinition` system already handles tokens with themed values — the Vibe Kit extends this with structured categories
- Auto-layout engine already speaks flexbox semantics (gap, padding, justify, align)
- Templates are PenNodes with roles — canvas, sync, code generation, MCP all work unchanged
- The "escape hatch" is natural: templates ARE PenNodes, users can break out to freeform
- Phase 1 animation engine provides the foundation for animation/transition primitives

## Target User

Solo creator / founder making their own social content. Needs speed, good defaults, minimal design skill required. The system should be opinionated by default — make the common case effortless.

## Key Decisions

1. **Vibe Kit = total restyling** — A Vibe Kit is a complete visual identity. Swapping it transforms every template entirely. Not selective overrides.
2. **Full spectrum from V1** — Typography, colors, textures, icons, animations, transitions, LUTs, SFX, strokes, size/space tokens all in the Vibe Kit system from day one.
3. **Templates styled by primitives** — Templates are layout structures that pull 100% of their styling from the active Vibe Kit.
4. **Format-agnostic** — LinkedIn formats are the starting point, but the system isn't platform-specific.
5. **Auto-reflow, always fit** — Switching canvas dimensions automatically reflows content. The grid always fits everything — scales and restacks, never hides.
6. **Block-first with escape hatch** — Default workflow is template/grid, but power users can break out to freeform PenNode editing.
7. **PenNode extension** — No separate content model. Vibe Kit extends the existing variables system; templates extend PenNodes.

## Resolved Questions

1. **Primitives scope:** Full spectrum from day one — not just static tokens but also animations, transitions, LUTs, SFX.
2. **Reflow behavior:** Always fit everything. Scale and restack, never hide blocks.
3. **Vibe Kit vs templates:** Vibe Kit owns ALL styling. Templates are pure structure. Swapping kits = total restyling.
4. **Token extraction scope:** Full extraction including textures/patterns, not just colors/fonts/spacing.
5. **V1 format presets:** LinkedIn only — carousel, video, and post formats.
6. **Default Vibe Kit:** No default kit. Onboarding flow has users extract from their own site or pick from a generated gallery. "User picks on first run."
7. **Asset generation:** SFX, LUT filters, and textures are AI-generated on demand via Replicate or Fal models. No large bundled asset library.
8. **V1 template library:** 5 essential templates — title/intro, content, quote, stat/metric, CTA/closing. Enough for any LinkedIn carousel.

## Open Questions

None — all questions resolved.

## Technical Notes

### Existing infrastructure to leverage:
- `VariableDefinition` / `$variable` refs — Token system with themed values → becomes the Vibe Kit backbone
- `PenNode.role` + `role-resolver.ts` — Semantic role system with registry-based defaults → templates
- `canvas-layout-engine.ts` — Auto-layout with padding, gap, justify, align → fluid grid foundation
- `animation/presets.ts` — Phase 1 animation presets (fade, slide, scale, bounce) → animation primitives
- `uikit/` — UIKit system for reusable components → template library storage
- `resolve-variables.ts` — Runtime `$ref` resolution → Vibe Kit application at render time
- `services/codegen/` — Code gen outputs `var(--name)` → export-ready output

### New infrastructure needed:
- **Vibe Kit schema** — Structured variable categories (typography, color, texture, animation, etc.) beyond flat `VariableDefinition`
- **Vibe Kit registry** — Storage, switching, import/export of complete kits
- **Fluid grid engine** — Reflow algorithm that recomputes layout for target dimensions (extends `canvas-layout-engine.ts`)
- **Format preset definitions** — Dimensions, safe areas, export specs per platform
- **Token extraction service** — Website scraping + token mapping to Vibe Kit schema
- **Template library** — Composed PenNode arrangements styled purely by Vibe Kit refs
- **AI asset generation pipeline** — Replicate/Fal integration for on-demand texture, LUT, and SFX generation
- **Onboarding flow** — First-run experience: extract from your site or pick from generated gallery
