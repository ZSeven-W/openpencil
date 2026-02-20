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

OpenPencil is an open-source vector design tool (alternative to Pencil.dev) with a Design-as-Code philosophy. Built as a **TanStack Start** full-stack React application with Bun runtime. Server API powered by **Nitro**.

**Key technologies:** React 19, Fabric.js v7 (canvas engine), Zustand v5 (state management), TanStack Router (file-based routing), Tailwind CSS v4, shadcn/ui (UI primitives), Vite 7, Nitro (server), TypeScript (strict mode).

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

- **`src/canvas/`** — Fabric.js integration (16 files):
  - `fabric-canvas.tsx` — Canvas component initialization
  - `canvas-object-factory.ts` — Creates Fabric objects from PenNodes (rect, ellipse, line, polygon, path, text, image, frame, group)
  - `canvas-object-sync.ts` — Syncs individual object properties between Fabric and store
  - `canvas-sync-lock.ts` — Prevents circular sync loops
  - `canvas-controls.ts` — Custom rotation controls and cursor styling
  - `canvas-constants.ts` — Default colors, zoom limits, stroke widths
  - `use-canvas-events.ts` — Drawing events, shape creation, smart guides activation, tool-based `skipTargetFind` management
  - `use-canvas-sync.ts` — Bidirectional PenDocument ↔ Fabric.js sync, node flattening with parent offsets
  - `use-canvas-viewport.ts` — Wheel zoom, space+drag panning, tool cursor switching, selection toggling per tool
  - `use-canvas-selection.ts` — Selection sync between Fabric objects and canvas-store
  - `use-canvas-guides.ts` — Smart alignment guides with snapping
  - `guide-utils.ts` — Guide calculation and rendering
  - `pen-tool.ts` — Bezier pen tool: anchor points, control handles, path closure, preview rendering
  - `parent-child-transform.ts` — Propagates parent transforms (move/scale/rotate) to children proportionally
  - `use-dimension-label.ts` — Shows size/position labels during object manipulation
  - `use-frame-labels.ts` — Renders frame names and boundaries on canvas
- **`src/stores/`** — Zustand stores (5 files):
  - `canvas-store.ts` — UI/tool/selection/viewport/clipboard/interaction state
  - `document-store.ts` — PenDocument tree CRUD: `addNode`, `updateNode`, `removeNode`, `moveNode`, `reorderNode`, `duplicateNode`, `groupNodes`, `ungroupNode`, `toggleVisibility`, `toggleLock`, `scaleDescendantsInStore`, `rotateDescendantsInStore`, `getNodeById`, `getParentOf`, `getFlatNodes`, `isDescendantOf`
  - `history-store.ts` — Undo/redo (max 300 states), batch mode for grouped operations
  - `ai-store.ts` — Chat messages, streaming state, generated code, model selection
  - `agent-settings-store.ts` — AI provider config (Anthropic/OpenAI), MCP CLI integrations, localStorage persistence
- **`src/types/`** — Type system:
  - `pen.ts` — PenDocument/PenNode (frame, group, rectangle, ellipse, line, polygon, path, text, image, ref), ContainerProps
  - `canvas.ts` — ToolType (select, frame, rectangle, ellipse, line, polygon, path, text, hand), ViewportState, SelectionState
  - `styles.ts` — PenFill (solid, linear_gradient, radial_gradient), PenStroke, PenEffect (shadow, blur)
  - `variables.ts` — VariableDefinition for design tokens
  - `agent-settings.ts` — AI provider config types
- **`src/components/editor/`** — Editor UI (6 files): editor-layout, toolbar, tool-button, shape-tool-dropdown (rectangle/ellipse/line/path + icon picker + image import), top-bar, status-bar
- **`src/components/panels/`** — Panels (15 files):
  - `layer-panel.tsx` / `layer-item.tsx` / `layer-context-menu.tsx` — Tree view with drag-and-drop reordering and drop-into-children (above/below/inside), visibility/lock toggles, context menu, rename
  - `property-panel.tsx` — Unified property panel
  - `fill-section.tsx` — Solid + gradient fill
  - `stroke-section.tsx` — Stroke color/width/dash
  - `corner-radius-section.tsx` — Unified or 4-point corner radius
  - `size-section.tsx` — Position, size, rotation
  - `text-section.tsx` — Font, size, weight, spacing, alignment
  - `effects-section.tsx` — Shadow and blur
  - `layout-section.tsx` — Auto-layout (none/vertical/horizontal), gap, padding, justify, align
  - `appearance-section.tsx` — Opacity, visibility, lock, flip
  - `ai-chat-panel.tsx` / `chat-message.tsx` — AI chat with markdown, design block collapse, apply design
  - `code-panel.tsx` — Code generation output (React/Tailwind and HTML/CSS)
- **`src/components/shared/`** — Reusable UI (8 files): ColorPicker, NumberInput, DropdownSelect, SectionHeader, ExportDialog, SaveDialog, AgentSettingsDialog, IconPickerDialog
- **`src/components/ui/`** — shadcn/ui primitives: Button, Select, Separator, Slider, Toggle, Tooltip
- **`src/services/ai/`** — AI chat service, design prompts, design-to-node generation, AI types
- **`src/services/codegen/`** — React+Tailwind and HTML+CSS code generators
- **`src/hooks/`** — `use-keyboard-shortcuts` (global keyboard event handling: tools, clipboard, undo/redo, save, select all, delete, arrow nudge, z-order)
- **`src/lib/`** — Utility functions (`utils.ts` with `cn()` for class merging)
- **`src/utils/`** — File operations (save/open .pen), export (PNG/SVG), node clone, pen file normalization, SVG parser (import SVG to editable PenNodes), syntax highlight
- **`server/api/ai/`** — Nitro server API: `chat.ts` (streaming SSE with thinking state), `generate.ts` (non-streaming generation), `connect-agent.ts` (Claude Code/Codex CLI connection), `models.ts` (model definitions). Supports Anthropic API key or Claude Agent SDK (local OAuth) as dual providers

### Fabric.js v7 Gotchas

- **Default origin is `center`/`center`** — always set `originX: 'left'`, `originY: 'top'` on objects so `left`/`top` means top-left corner
- **Pointer capture** — Fabric captures pointers on `upperCanvasEl`; attach pointer listeners there, not on `document`
- **Coordinate conversion** — use `canvas.getScenePoint(e)` with `canvas.calcOffset()` for accurate pointer-to-scene mapping
- **Default strokeWidth is 1** — explicitly set `strokeWidth: 0` when no stroke is desired
- **Tool isolation** — when a drawing tool is active, set both `canvas.selection = false` and `canvas.skipTargetFind = true` to prevent Fabric from selecting existing objects during draw. Restore both when switching back to select tool.
- **Parent-child transforms** — nodes are flattened to absolute coordinates for Fabric; `nodeRenderInfo` stores parent offsets for converting back to relative coordinates. `parent-child-transform.ts` handles propagating transforms to descendants during drag/scale/rotate.

### Canvas Tool State Management

When switching tools, **two subscribers** manage canvas state:
- `use-canvas-events.ts` — sets `selection`/`skipTargetFind` based on drawing vs select tool
- `use-canvas-viewport.ts` — also manages `selection`/`skipTargetFind` for tool switches and space-key panning

Both must stay consistent: only `select` tool (without space pressed) should have `selection = true` and `skipTargetFind = false`.

### Routing

File-based routing via TanStack Router. Routes in `src/routes/`, auto-generated tree in `src/routeTree.gen.ts` (do not edit).

- `/` — Landing page
- `/editor` — Main design editor

### Path Aliases

`@/*` maps to `./src/*` (configured in `tsconfig.json` and `vite.config.ts`).

### Styling

Tailwind CSS v4 imported via `src/styles.css`. UI primitives from shadcn/ui (`src/components/ui/`). Icons from `lucide-react`. shadcn/ui config in `components.json`.

## Code Style

- 单个文件不要超过 800 行。超出时应拆分为更小的模块。
- 每个文件只导出一个组件，每个组件只承担单一职责。
- `.ts` 和 `.tsx` 文件命名使用 kebab-case（烤肉串风格），例如 `canvas-store.ts`、`use-keyboard-shortcuts.ts`。
- UI 组件统一使用 shadcn/ui 设计令牌（`bg-card`、`text-foreground`、`border-border` 等），禁止使用硬编码的 `gray-*`、`blue-*` 等 Tailwind 颜色。
- 工具栏按钮激活状态直接用 `isActive` 条件 className（`bg-primary text-primary-foreground`），不使用 Radix Toggle 的 `data-[state=on]:` 选择器（存在 twMerge 冲突）。

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

## License

MIT License. See [LICENSE](./LICENSE) for details.
