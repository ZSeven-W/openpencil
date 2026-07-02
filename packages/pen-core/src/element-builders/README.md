# element-builders

Pure tree-build functions (one per N-tool `add_X_v0` MCP tool) that produce `PenNode` subtrees from typed parameters. Shared between `pen-mcp` handlers (external MCP clients like Claude Code / Codex / Gemini CLI) and the browser-side client shim in `apps/web/src/services/ai/element-tool-shims/`.

## Why a shared module

The N-tool system has **three executable paths** that all need to produce identical trees for the same args:

```text
 ┌────────────────────┐       ┌────────────────────┐       ┌────────────────────┐
 │  pen-mcp handler   │       │ apps/web shim      │       │ Nitro server bld   │
 │  (external clients │       │ (browser-side      │       │ (/api/mcp/exec-    │
 │   via stdio/HTTP)  │       │  client shim)      │       │  tool HTTP fallbk) │
 └─────────┬──────────┘       └─────────┬──────────┘       └─────────┬──────────┘
           │                            │                            │
           ▼                            ▼                            ▼
           ┌──────────────────────────────────────────────────────────┐
           │              @zseven-w/pen-core/element-builders         │
           │  (buildHeading, buildCardRow, buildTopNavBar, …, 50×)    │
           └──────────────────────────────────────────────────────────┘
```

If all three paths import the same `buildX` here, the tree is **drift-free by construction** — no registry can silently emit a different shape.

## Module layout

- `index.ts` — barrel; **every new builder must be re-exported here**
- `helpers.ts` — `assignIdsRecursively`, `buildScrollWrapper`, `ElementTree`
- `cjk-detect.ts` — `detectCjkScript`, `cjkFontFamily` (Noto Sans SC/JP/KR dispatch)
- `<name>.ts` — one file per tool (50 today, as of 2026-04-22), each exporting `build<Name>` + its params type

## What a builder is

A pure function that takes typed params and returns an `ElementTree`:

```ts
import type { ElementTree } from './helpers.js';

export interface MyThingParams {
  label: string;
  icon?: string;
}

export function buildMyThing(params: MyThingParams): ElementTree {
  return {
    type: 'frame',
    name: 'My Thing',
    role: 'my-thing',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    // …
    children: [
      // …
    ],
  };
}
```

Rules:

- **Browser-safe**: no `node:fs`, no I/O, no `import.meta.env` guards, no async. Builders are synchronous value producers.
- **No id stamping**: ids are stamped by `assignIdsRecursively` AFTER construction (callers own that step).
- **No parent wiring**: `parent_id` / `pageId` / `filePath` are **meta params** stripped before the builder is called.
- **Return shape** is intentionally `Record<string, unknown>` (loose) — the downstream insert pipeline validates.

## Conventions crystallized from 42 existing builders

- **Layout sizing**: frame containers use `width: 'fill_container'` + `height: 'fit_content'` as the default. Atoms that carry concrete dimensions (avatars, rings) set numeric sizes.
- **Icons are `icon_font` nodes**: `type: 'icon_font'` + `iconFontFamily: 'lucide'` + `iconFontName: '<slug>'`. Never use `path` for icons in builder output.
- **Text nodes**: never set explicit `height`. Let text grow; use `fontSize`/`lineHeight`/`letterSpacing` to control typography.
- **Roles**: every top-level node sets `role: '<kebab-name>'`. Sub-nodes may set scoped roles (`list-row-text`, `card`, `stat-cell`). The role string drives downstream post-processing (role-resolver, contrast pass, layout inference).
- **CJK dispatch**: only `buildHeading` dispatches fontFamily per script (`Noto Sans SC/JP/KR`). Body text always `fontFamily: 'Inter'`. Other builders don't dispatch CJK — children inherit the renderer default.
- **Text-button / form-input**: `width: 'fill_container'`, height 48, cornerRadius 8, padding `[12, 16]` or `[12, 24]` — matches the Pencil-demo contract.
- **Icon-only buttons**: 44×44 (Apple HIG / Material min-hit-target), flex-centered `icon_font`. Never `layout: 'none'` with manual x/y.
- **Ring / circle with content**: use `frame` with `cornerRadius: width/2`. Never two stacked ellipses — that anti-pattern trips `rewriteLlmAntiPatterns`.
- **Divider**: `rectangle` with `height: 1` and `fill_container`. Never a one-sided stroke on a frame (renderer only supports uniform or `[T,R,B,L]` stroke thicknesses).

## Adding a new builder

1. **Create** `packages/pen-core/src/element-builders/<name>.ts` exporting `build<Name>(params)` + `<Name>Params`
2. **Re-export** from `packages/pen-core/src/element-builders/index.ts`
3. **Re-export** from `packages/pen-core/src/index.ts` (main barrel) — only matters if apps/web or pen-mcp import it via `@zseven-w/pen-core` directly
4. **Wire pen-mcp handler**: `packages/pen-mcp/src/tools/add-<name>-v0.ts` imports `build<Name>` and delegates
5. **Register the pen-mcp handler** in `packages/pen-mcp/src/routes/element-tool-defs.ts` (add import + switch branch + tool schema in `-base.ts` / `-ext.ts`)
6. **Wire apps/web shim**: add to `ELEMENT_SHIMS` in `apps/web/src/services/ai/element-tool-shims/index.ts`
7. **Wire server builder**: add to `SERVER_BUILDERS` in `apps/web/server/api/mcp/exec-tool.post.ts`
8. **Add to `elements.md`**: `packages/pen-ai-skills/skills/phases/generation/elements.md` — add the tool to the PREFER list + examples section
9. **Tests** (all of these should pass automatically if the builder follows conventions):
   - Layout smoke in `packages/pen-core/src/__tests__/element-builders-layout.test.ts`
   - Idempotency in `element-builders-post-process-idempotent.test.ts`
   - Role coverage in `apps/web/src/services/ai/__tests__/role-resolver-builder-coverage.test.ts`
   - Parity in `shim-server-parity.test.ts` + `element-tool-registry-parity.test.ts`
   - If the builder emits an `icon_font`, check `builder-icon-coverage.test.ts` covers it

Parity tests fail-fast on any skipped step — the test names tell you which registry or handler is missing.

## Drift guards in the test suite

Tests to run when touching this module (`bun run test` at repo root runs all):

| Test                                                | What it catches                                                       |
| --------------------------------------------------- | --------------------------------------------------------------------- |
| `element-builders-layout.test.ts`                   | `computeLayoutPositions` throws on your new builder                   |
| `element-builders-composition.test.ts`              | Builder doesn't compose with others in a screen frame                 |
| `element-builders-post-process-idempotent.test.ts`  | A post-pass mutates output twice                                      |
| `element-builders-edge-cases.test.ts`               | Extreme inputs (empty, huge, boundary) crash                          |
| `element-builders-cjk-dispatch.test.ts`             | CJK font dispatch regresses                                           |
| `element-builders-normalize-preservation.test.ts`   | `normalizePenDocument` drops a semantic field                         |
| `element-builders-performance.test.ts`              | New builder adds >5ms avg or >50ms cold                               |
| `role-resolver-builder-coverage.test.ts` (apps/web) | Role string doesn't match role-definitions set or typo                |
| `detectors-builder-clean.test.ts` (apps/web)        | Builder trips a pre-validation detector                               |
| `anti-patterns-builder-clean.test.ts` (apps/web)    | Builder produces an anti-pattern (stacked ellipses, open path + fill) |
| `shim-server-parity.test.ts` (apps/web)             | Shim and direct-buildX output diverge                                 |
| `builder-icon-coverage.test.ts` (apps/web)          | Builder emits an icon name that doesn't resolve                       |
| `element-tool-registry-parity.test.ts` (pen-mcp)    | Registry + handler file + dispatcher switch drift                     |

## Related reading

- `packages/pen-core/CLAUDE.md` — broader pen-core module map
- `packages/pen-ai-skills/skills/phases/generation/elements.md` — prompt-level spec of each tool, loaded by the AI
- `apps/web/src/services/ai/element-tool-shims/index.ts` — client shim registry
- `apps/web/src/services/ai/element-tools-dispatcher.ts` — routes parsed `<op_tool>` → shim → insert
- `apps/web/server/api/mcp/exec-tool.post.ts` — Nitro HTTP fallback (same builder catalog, for cases the client shim doesn't support)
- Spec: `openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md`
