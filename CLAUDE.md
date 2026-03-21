# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
Detailed module docs are in `packages/CLAUDE.md`, `apps/web/CLAUDE.md`, and `apps/desktop/CLAUDE.md` — loaded automatically when working in those directories.

## Commands

- **Dev server:** `bun --bun run dev` (runs on port 3000)
- **Build:** `bun --bun run build`
- **Preview production build:** `bun --bun run preview`
- **Run all tests:** `bun --bun run test` (Vitest)
- **Run a single test:** `bun --bun vitest run path/to/test.ts`
- **Type check:** `npx tsc --noEmit`
- **Install dependencies:** `bun install`
- **Bump version:** `bun run bump <version>` (syncs all package.json files)
- **Electron dev:** `bun run electron:dev` (starts Vite + Electron together)
- **Electron compile:** `bun run electron:compile` (esbuild electron/ to out/desktop/)
- **Electron build:** `bun run electron:build` (full web build + compile + electron-builder package)

## Architecture

OpenPencil is an open-source vector design tool (alternative to Pencil.dev) with a Design-as-Code philosophy. Organized as a **Bun monorepo** with workspaces:

```text
openpencil/
├── apps/
│   ├── web/           TanStack Start full-stack React app (Vite + Nitro)
│   └── desktop/       Electron desktop app (macOS, Windows, Linux)
├── packages/
│   ├── pen-types/     Type definitions for PenDocument model
│   ├── pen-core/      Document tree ops, layout engine, variables, boolean ops, clone utilities
│   ├── pen-codegen/   Multi-platform code generators
│   ├── pen-figma/     Figma .fig file parser and converter
│   ├── pen-renderer/  Standalone CanvasKit/Skia renderer
│   └── pen-sdk/       Umbrella SDK (re-exports all packages)
└── .githooks/         Pre-commit version sync from branch name
```

**Key technologies:** React 19, CanvasKit/Skia WASM (canvas engine), Paper.js (boolean path operations), Zustand v5 (state management), TanStack Router (file-based routing), Tailwind CSS v4, shadcn/ui (UI primitives), Vite 7, Nitro (server), Electron 35 (desktop), TypeScript (strict mode).

### Data Flow

```text
React Components (Toolbar, LayerPanel, PropertyPanel)
        │ Zustand hooks
        ▼
┌─────────────────┐    ┌───────────────────┐
│  canvas-store   │    │  document-store   │ ← single source of truth
│  (UI state:     │    │  (PenDocument)    │
│   tool/selection │    │  CRUD / tree ops  │
│   /viewport)    │    │                   │
└────────┬────────┘    └────────┬──────────┘
         │                      │
         ▼                      ▼
   CanvasKit/Skia        canvas-sync-lock
   (GPU-accelerated      (prevents circular sync)
    WASM renderer)
```

- **document-store** is the single source of truth. CanvasKit only renders.
- User edits on canvas → SkiaEngine events → update document-store
- User edits in panels → update document-store → SkiaEngine `syncFromDocument()` re-renders
- `canvas-sync-lock.ts` prevents circular updates when canvas events write to the store

### Multi-Page Architecture

```text
PenDocument
  ├── pages?: PenPage[]   (id, name, children)
  └── children: PenNode[] (default/single-page fallback)
```

### Design Variables Architecture

- **`$variable` references are preserved** in the document store (e.g. `$color-1` in fill color)
- `resolveNodeForCanvas()` resolves `$refs` on-the-fly before CanvasKit rendering
- Code generators output `var(--name)` for `$ref` values
- Multiple theme axes supported (e.g. Theme-1 with Light/Dark, Theme-2 with Compact/Comfortable)

### MCP Layered Design Workflow

External LLMs (Claude Code, Codex, Gemini CLI, etc.) can generate designs via MCP:

- **Single-shot**: `batch_design` or `insert_node` — one call
- **Layered**: `design_skeleton` → `design_content` × N → `design_refine` — phased generation with focused context
- **Segmented prompts**: `get_design_prompt(section=...)` loads focused subsets (schema, layout, roles, icons, etc.)

### Path Aliases

`@/*` maps to `./src/*` (configured in `apps/web/tsconfig.json` and `apps/web/vite.config.ts`).

### Styling

Tailwind CSS v4 imported via `apps/web/src/styles.css`. UI primitives from shadcn/ui. Icons from `lucide-react`.

### CI / CD

- **`.github/workflows/ci.yml`** — Push/PR: type check, tests, web build
- **`.github/workflows/build-electron.yml`** — Tag push (`v*`) or manual: builds Electron for all platforms, creates draft GitHub Release
- **`.github/workflows/docker.yml`** — Docker image build and push

### Version Sync

- **Pre-commit hook** (`.githooks/pre-commit`): extracts version from branch name (e.g. `v0.5.0` → `0.5.0`) and syncs to all `package.json` files
- **Manual bump:** `bun run bump <version>` to set a specific version across all workspaces
- Requires `git config core.hooksPath .githooks` (one-time setup per clone)

## Code Style

- Single files must not exceed 800 lines. Split into smaller modules when they grow beyond this limit.
- One component per file, each with a single responsibility.
- `.ts` and `.tsx` files use kebab-case naming, e.g. `canvas-store.ts`, `use-keyboard-shortcuts.ts`.
- UI components must use shadcn/ui design tokens (`bg-card`, `text-foreground`, `border-border`, etc.). No hardcoded Tailwind colors like `gray-*`, `blue-*`.
- Toolbar button active state uses `isActive` conditional className (`bg-primary text-primary-foreground`), not Radix Toggle's `data-[state=on]:` selector (has twMerge conflicts).

## Git Commit Convention

Use [Conventional Commits](https://www.conventionalcommits.org/) format: `<type>(<scope>): <subject>`

**Types:** `feat`, `fix`, `refactor`, `perf`, `style`, `docs`, `test`, `chore`

**Scopes:** `editor`, `canvas`, `panels`, `history`, `ai`, `codegen`, `store`, `types`, `variables`, `figma`, `mcp`, `electron`, `renderer`, `sdk`

**Rules:** Subject in English, lowercase start, no period, imperative mood. Body is optional; explain **why** not what. One commit per change.

## License

MIT License. See [LICENSE](./LICENSE) for details.
