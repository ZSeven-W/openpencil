# Web App

TanStack Start full-stack React app (Vite + Nitro). Routes in `src/routes/`, auto-generated tree in `src/routeTree.gen.ts` (do not edit).

- `/` — Landing page
- `/editor` — Main design editor

## Canvas Engine (`src/canvas/`)

14 files + `skia/` subdir with 14 files.

### CanvasKit/Skia Architecture

- **GPU-accelerated WASM rendering** — CanvasKit (Skia compiled to WASM) renders all canvas content via WebGL surface
- **SkiaEngine class** (`skia-engine.ts`) is the core: owns the render loop, viewport transforms, node flattening, and `SpatialIndex` for hit testing
- **Dirty-flag rendering** — `markDirty()` schedules a `requestAnimationFrame` redraw; no continuous rendering loop
- **Node flattening** — `syncFromDocument()` walks the PenDocument tree, resolves auto-layout positions via layout engine, and produces flat `RenderNode[]` with absolute coordinates
- **SpatialIndex** (`skia-hit-test.ts`) — R-tree backed spatial queries for `hitTest()` (click) and `searchRect()` (marquee selection)
- **Coordinate conversion** — `screenToScene()` / `sceneToScreen()` in `skia-viewport.ts` handle viewport ↔ scene transforms
- **Event handling** — mouse/keyboard events managed by `SkiaInteractionManager` (`skia-interaction.ts`); hit testing for resize/rotate/arc handles in `skia-hit-handlers.ts`; `skia-canvas.tsx` is the React component (lifecycle, sync, rendering)
- **Parent-child transforms** — nodes are flattened to absolute coordinates; transforms propagate to descendants during drag/scale/rotate

### `skia/` Files

- `skia-canvas.tsx` — React component: lifecycle, sync effects, wheel zoom, text editing overlay; delegates interaction to `SkiaInteractionManager`
- `skia-interaction.ts` — `SkiaInteractionManager` class: all mouse/keyboard interaction state and handlers (select, drag, resize, rotate, draw, marquee, pen tool, arc editing, hover cursor)
- `skia-hit-handlers.ts` — Hit test functions: `hitTestHandle` (resize), `hitTestRotation` (rotation zone), `hitTestArcHandle` (ellipse arc)
- `skia-engine.ts` — Core rendering engine: `SkiaEngine` class, `syncFromDocument()`, viewport, node flattening, zoom/pan, dirty-flag loop
- `skia-renderer.ts` — GPU draw calls: shapes, text, paths, images, selection handles, guides, agent indicators
- `skia-init.ts` — CanvasKit WASM loader with CDN fallback
- `skia-hit-test.ts` — `SpatialIndex` R-tree for spatial queries
- `skia-viewport.ts` — Viewport math
- `skia-paint-utils.ts` — Color parsing, gradient creation, text line wrapping
- `skia-path-utils.ts` — SVG path to CanvasKit Path conversion
- `skia-image-loader.ts` — Async image loading and caching
- `skia-overlays.ts` — Selection overlays, hover highlights, dimension labels
- `skia-pen-tool.ts` — Pen tool: anchor points, control handles, path building
- `skia-font-manager.ts` — Font management

### Shared Canvas Modules

- `canvas-sync-lock.ts` — Prevents circular sync loops
- `canvas-sync-utils.ts` — `forcePageResync()` utility
- `canvas-constants.ts` — Default colors, zoom limits, stroke widths
- `canvas-node-creator.ts` — `createNodeForTool`, `isDrawingTool`
- `canvas-layout-engine.ts` — Auto-layout (delegates to `@zseven-w/pen-core`)
- `canvas-text-measure.ts` — Text width/height estimation, CJK detection
- `font-utils.ts`, `node-helpers.ts` — Re-exports from pen-core
- `insertion-indicator.ts`, `selection-context.ts`, `agent-indicator.ts`, `use-layout-indicator.ts`, `skia-engine-ref.ts`

## Zustand Stores (`src/stores/`)

- `canvas-store.ts` — UI/tool/selection/viewport/clipboard/interaction state, `activePageId`
- `document-store.ts` — PenDocument tree CRUD, variable CRUD, component management (all with history)
- `document-store-pages.ts` — Page actions: add, remove, rename, reorder, duplicate
- `document-tree-utils.ts` — Re-exports tree helpers and clone utilities from `@zseven-w/pen-core`
- `history-store.ts` — Undo/redo (max 300 states), batch mode
- `ai-store.ts` — Chat messages, streaming state, model selection
- `agent-settings-store.ts` — AI provider config, MCP CLI integrations, localStorage persistence
- `uikit-store.ts` — UIKit management
- `theme-preset-store.ts` — Theme preset management

## Components (`src/components/`)

- **`editor/`** — Editor UI: editor-layout, toolbar, boolean-toolbar, tool-button, shape-tool-dropdown, top-bar, status-bar, page-tabs, update-ready-banner
- **`panels/`** — 32 files: layer panel, property panel, fill/stroke/corner/size/text/effects/export/layout/appearance sections, AI chat panel, code panel, component browser, variables panel
- **`shared/`** — Reusable UI: ColorPicker, NumberInput, SectionHeader, ExportDialog, SaveDialog, AgentSettingsDialog, IconPickerDialog, VariablePicker, FigmaImportDialog, FontPicker, LanguageSelector
- **`icons/`** — Provider/brand logos
- **`ui/`** — shadcn/ui primitives

## AI Services (`src/services/ai/`)

35 files + `role-definitions/` + `design-principles/` subdirs:
- `ai-service.ts` — Main AI chat API wrapper, model negotiation, provider selection
- `ai-prompts.ts` — System prompts for design generation
- `ai-types.ts` — ChatMessage, ChatAttachment, AIDesignRequest, OrchestratorPlan
- `model-profiles.ts` — Adapts thinking mode, effort, timeouts per model tier
- `design-generator.ts` — Top-level `generateDesign`/`generateDesignModification`
- `design-parser.ts` — JSON/JSONL parsing
- `design-canvas-ops.ts` — Canvas mutation operations
- `design-node-sanitization.ts` — Node merging (re-exports `deepCloneNode` from pen-core)
- `design-validation.ts` / `design-pre-validation.ts` / `design-validation-fixes.ts` — Post-generation validation
- `icon-resolver.ts` — Auto-resolves icon names to Lucide SVG paths
- `orchestrator.ts` / `orchestrator-sub-agent.ts` / `orchestrator-prompts.ts` — Spatial decomposition orchestrator
- `context-optimizer.ts` — Chat history trimming

## Hooks (`src/hooks/`)

- `use-keyboard-shortcuts.ts` — Global keyboard: tools, clipboard, undo/redo, save, z-order, boolean ops
- `use-electron-menu.ts` — Electron native menu IPC listener
- `use-figma-paste.ts` — Figma clipboard paste
- `use-file-drop.ts` — File drag-and-drop
- `use-mcp-sync.ts` — MCP live canvas sync
- `use-system-fonts.ts` — System font detection

## MCP Server (`src/mcp/`)

- `server.ts` — MCP server entry point, tool registration (stdio + HTTP modes)
- `document-manager.ts` — Document read/write/cache; live canvas sync via Nitro API
- `tools/` — Core (open-document, batch-get, get-selection, batch-design, node-crud), Layout (snapshot-layout, find-empty-space, import-svg), Variables, Pages, Layered design (design-prompt, design-skeleton, design-content, design-refine)
- `utils/` — `id.ts`, `node-operations.ts` (re-exports `cloneNodeWithNewIds` from pen-core), `sanitize.ts`, `svg-node-parser.ts`

## UIKit (`src/uikit/`)

- `built-in-registry.ts` — Default built-in UIKit
- `kit-import-export.ts` — Import/export UIKits from .pen files
- `kit-utils.ts` — Extract components, find reusable nodes (re-exports `deepCloneNode` from pen-core)

## Utilities (`src/utils/`)

File operations: save/open .pen, export PNG/SVG, node clone (re-exports `cloneNodesWithNewIds` from pen-core), pen file normalization, SVG parser, syntax highlight, boolean operations, `app-storage.ts`, `arc-path.ts`, `theme-preset-io.ts`, `id.ts`

## Server API (`server/`)

- **`api/ai/`** — Nitro API (11 files): streaming chat, generation, agent connection, validation, MCP install, icon resolution, image generation/search. Supports Anthropic API key or Claude Agent SDK (local OAuth)
- **`utils/`** — Server utilities: Claude CLI resolver, OpenCode/Codex/Copilot clients, MCP server manager, sync state, server logger
