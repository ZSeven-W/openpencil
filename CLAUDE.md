# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

- **Dev server:** `bun --bun run dev` (runs on port 3000)
- **Build:** `bun --bun run build`
- **Preview production build:** `bun --bun run preview`
- **Run all tests:** `bun --bun run test` (Vitest)
- **Run a single test:** `bun --bun vitest run path/to/test.ts`
- **Type check:** `npx tsc --noEmit`
- **Install dependencies:** `bun install`

## Architecture

OpenPencil is an open-source vector design tool (alternative to Pencil.dev) with a Design-as-Code philosophy. Built as a **TanStack Start** full-stack React application with Bun runtime.

**Key technologies:** React 19, Fabric.js v7 (canvas engine), Zustand v5 (state management), TanStack Router (file-based routing), Tailwind CSS v4, Vite 7, TypeScript (strict mode).

### Data Flow

```
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
   Fabric.js Canvas      canvas-sync-lock
   (imperative render)   (prevents circular sync)
```

- **document-store** is the single source of truth. Fabric.js only renders.
- User edits on canvas → Fabric events → update document-store (with sync lock)
- User edits in panels → update document-store → `use-canvas-sync` updates Fabric
- `canvas-sync-lock.ts` prevents circular updates when Fabric events write to the store

### Key Modules

- **`src/canvas/`** — Fabric.js integration: canvas init, drawing events, viewport (pan/zoom), selection sync, bidirectional document↔canvas sync, object factory
- **`src/stores/`** — Zustand stores: `canvas-store` (UI state), `document-store` (PenDocument tree CRUD)
- **`src/types/`** — Type system: `pen.ts` (PenDocument/PenNode), `canvas.ts` (ToolType), `styles.ts` (Fill/Stroke/Effect)
- **`src/components/editor/`** — Editor layout, toolbar, tool buttons
- **`src/components/panels/`** — Layer panel, property panel with section components
- **`src/components/shared/`** — Reusable UI: ColorPicker, NumberInput, SliderInput

### Fabric.js v7 Gotchas

- **Default origin is `center`/`center`** — always set `originX: 'left'`, `originY: 'top'` on objects so `left`/`top` means top-left corner
- **Pointer capture** — Fabric captures pointers on `upperCanvasEl`; attach pointer listeners there, not on `document`
- **Coordinate conversion** — use `canvas.getScenePoint(e)` with `canvas.calcOffset()` for accurate pointer-to-scene mapping
- **Default strokeWidth is 1** — explicitly set `strokeWidth: 0` when no stroke is desired

### Routing

File-based routing via TanStack Router. Routes in `src/routes/`, auto-generated tree in `src/routeTree.gen.ts` (do not edit).

- `/` — Landing page
- `/editor` — Main design editor

### Path Aliases

`@/*` maps to `./src/*` (configured in `tsconfig.json` and `vite.config.ts`).

### Styling

Tailwind CSS v4 imported via `src/styles.css`. Icons from `lucide-react`.

## Code Style

- 单个文件不要超过 800 行。超出时应拆分为更小的模块。
- 每个文件只导出一个组件，每个组件只承担单一职责。
- `.ts` 和 `.tsx` 文件命名使用 kebab-case（烤肉串风格），例如 `canvas-store.ts`、`use-keyboard-shortcuts.ts`。
- UI 组件统一使用 shadcn/ui 设计令牌（`bg-card`、`text-foreground`、`border-border` 等），禁止使用硬编码的 `gray-*`、`blue-*` 等 Tailwind 颜色。

## Git Commit 规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <subject>

<body>
```

### Type

- `feat` — 新功能
- `fix` — Bug 修复
- `refactor` — 重构（不改变行为）
- `perf` — 性能优化
- `style` — 代码格式（不影响逻辑）
- `docs` — 文档
- `test` — 测试
- `chore` — 构建/工具/依赖变更

### Scope

按模块划分：`editor`、`canvas`、`panels`、`history`、`ai`、`codegen`、`store`、`types`。

### 规则

- subject 用英文，小写开头，不加句号，祈使语气（如 `add`、`fix`、`remove`）。
- body 可选，解释 **why** 而非 what，可用中英文。
- 一个 commit 只做一件事。不要把不相关的改动混在一起。
