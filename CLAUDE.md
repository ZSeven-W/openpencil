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
- **Electron dev:** `bun run electron:dev` (starts Vite + Electron together)
- **Electron compile:** `bun run electron:compile` (esbuild electron/ to electron-dist/)
- **Electron build:** `bun run electron:build` (full web build + compile + electron-builder package)

## Architecture

OpenPencil is an open-source vector design tool (alternative to Pencil.dev) with a Design-as-Code philosophy. Built as a **TanStack Start** full-stack React application with Bun runtime. Server API powered by **Nitro**. Also ships as an **Electron** desktop app for macOS, Windows, and Linux.

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

- `document-store-pages.ts` — page CRUD actions: `addPage`, `removePage`, `renamePage`, `reorderPage`, `duplicatePage`
- `canvas-store.ts` — `activePageId` state, `setActivePageId` action
- `canvas-sync-utils.ts` — `forcePageResync()` triggers page-aware canvas re-sync
- `page-tabs.tsx` — tab bar UI for multi-page navigation with context menu

### Design Variables Architecture

```text
PenDocument (source of truth)
  ├── variables: Record<string, VariableDefinition>   ($color-1, $spacing-md, ...)
  ├── themes: Record<string, string[]>                ({Theme-1: ["Default","Dark"]})
  └── children: PenNode[]                             (nodes with $variable refs)
                │
     ┌──────────┴──────────┐
     ▼                      ▼
  Canvas Sync             Code Generation
  resolveNodeForCanvas()  $ref → var(--name)
  $ref → concrete value   CSS Variables block
```

- **`$variable` references are preserved** in the document store (e.g. `$color-1` in fill color)
- `normalize-pen-file.ts` does NOT resolve `$refs` — only fixes format issues
- `resolveNodeForCanvas()` resolves `$refs` on-the-fly before CanvasKit rendering
- Code generators output `var(--name)` for `$ref` values
- Multiple theme axes supported (e.g. Theme-1 with Light/Dark, Theme-2 with Compact/Comfortable)
- Each theme axis has variants; variables can have per-variant values (`ThemedValue[]`)

### MCP Layered Design Workflow

External LLMs (Claude Code, Codex, Gemini CLI, etc.) can generate designs via MCP using two approaches:

**Single-shot** (existing): `batch_design` or `insert_node` — generate entire design in one call. Simple but lower fidelity for complex designs.

**Layered** (new): Break generation into phases, each with focused context and per-section post-processing:

```text
get_design_prompt(section="planning")     → Load planning-specific guidelines
        │
        ▼
design_skeleton(rootFrame, sections)      → Create root + section frames
        │                                    Returns: section IDs, content width,
        │                                    per-section guidelines, suggested roles
        ▼
design_content(sectionId, children) ×N    → Populate each section independently
        │                                    Runs: role resolution, icon resolution,
        │                                    sanitization per section
        ▼
design_refine(rootId)                     → Full-tree validation + auto-fixes
                                             Returns: fix report, layout snapshot
```

**`get_design_prompt` segmented retrieval**: Instead of loading the full ~8K char prompt at once, external LLMs can request focused subsets:
- `schema` — PenNode types, fill/stroke format
- `layout` — Flexbox layout engine rules
- `roles` — Semantic role listing with defaults
- `text` — Typography, CJK, copywriting rules
- `style` — Visual style policy, palette
- `icons` — Available icon names + usage
- `examples` — Design examples
- `guidelines` — General design tips
- `planning` — Design type detection, section decomposition, style guide template, layered workflow guide

**`design_skeleton` section guidelines**: The tool generates context-specific content guidelines for each section based on its name/role (nav → navbar layout tips, hero → headline sizing, form → input width rules, etc.), reducing per-call cognitive load.

### Key Modules

- **`src/canvas/`** — Canvas engine (33 files + `skia/` subdir with 11 files):
  - **`skia/`** — CanvasKit/Skia WASM renderer (primary canvas engine):
    - `skia-canvas.tsx` — Main canvas React component (SkiaCanvas), mouse/keyboard event handling, drawing tools, select/drag/resize/rotate, marquee, pen tool, text editing overlay
    - `skia-engine.ts` — Core rendering engine: `SkiaEngine` class, `syncFromDocument()`, viewport transform, node flattening with layout resolution, `SpatialIndex` integration, zoom/pan, dirty-flag rendering loop
    - `skia-renderer.ts` — GPU-accelerated draw calls: rectangles, ellipses, text, paths, images, frames, groups, selection handles, guides, agent indicators
    - `skia-init.ts` — CanvasKit WASM loader with CDN fallback
    - `skia-hit-test.ts` — `SpatialIndex` for spatial queries: `hitTest()`, `searchRect()`, R-tree backed
    - `skia-viewport.ts` — Viewport math: `screenToScene`, `sceneToScreen`, `viewportMatrix`, `zoomToPoint`
    - `skia-paint-utils.ts` — Color parsing, gradient creation, text line wrapping for CanvasKit
    - `skia-path-utils.ts` — SVG path `d` string to CanvasKit Path conversion
    - `skia-image-loader.ts` — Async image loading and caching for CanvasKit
    - `skia-overlays.ts` — Selection overlays, hover highlights, dimension labels
    - `skia-pen-tool.ts` — Pen tool implementation for Skia: anchor points, control handles, path building
  - **Legacy Fabric.js files** (retained for `zoomToFitContent` utility, pending removal):
    - `fabric-canvas.tsx`, `use-fabric-canvas.ts`, `canvas-object-factory.ts`, `canvas-object-sync.ts`, `canvas-object-modified.ts`, `canvas-controls.ts`
  - **Shared modules** (used by both Skia and legacy code):
    - `canvas-sync-lock.ts` — Prevents circular sync loops
    - `canvas-sync-utils.ts` — `forcePageResync()` utility for page-aware canvas re-sync
    - `canvas-constants.ts` — Default colors, zoom limits, stroke widths
    - `canvas-node-creator.ts` — `createNodeForTool`, `isDrawingTool` helpers
    - `canvas-layout-engine.ts` — Auto-layout computation: `resolvePadding`, `getNodeWidth/Height`, `computeLayoutPositions`, `Padding` interface
    - `canvas-text-measure.ts` — Text width/height estimation, CJK detection, `parseSizing`
    - `canvas-zoom-cache.ts` — Bitmap caching during zoom/pan for performance
    - `use-canvas-sync.ts` — PenDocument ↔ canvas sync, node flattening, variable resolution via `resolveNodeForCanvas()`
    - `use-canvas-events.ts` — Drawing events, shape creation
    - `use-canvas-viewport.ts` — Wheel zoom, space+drag panning, tool cursor switching
    - `use-canvas-selection.ts` — Selection sync between canvas and store
    - `use-canvas-hover.ts` — Hover state management
    - `use-canvas-guides.ts` / `guide-utils.ts` — Smart alignment guides with snapping
    - `pen-tool.ts` — Bezier pen tool (Fabric-based, Skia version in `skia/skia-pen-tool.ts`)
    - `parent-child-transform.ts` — Propagates parent transforms to children proportionally
    - `use-dimension-label.ts` / `use-frame-labels.ts` / `use-entered-frame-overlay.ts` / `use-layout-indicator.ts` — Visual overlays
    - `insertion-indicator.ts` / `drag-into-layout.ts` / `drag-reparent.ts` / `layout-reorder.ts` — Drag-and-drop support
    - `selection-context.ts` — Multi-select context management
    - `agent-indicator.ts` / `use-agent-indicators.ts` — Agent visual indicators on canvas during concurrent AI generation
- **`src/variables/`** — Design variables system (2 files):
  - `resolve-variables.ts` — Core resolution utilities: `resolveVariableRef`, `resolveNodeForCanvas`, `getDefaultTheme`, `isVariableRef`; resolves `$variable` references to concrete values for canvas rendering with circular reference guards
  - `replace-refs.ts` — `replaceVariableRefsInTree`: recursively walk node tree to replace/resolve `$refs` when renaming or deleting variables (covers opacity, gap, padding, fills, strokes, effects, text)
- **`src/stores/`** — Zustand stores (9 files):
  - `canvas-store.ts` — UI/tool/selection/viewport/clipboard/interaction state, `variablesPanelOpen` toggle, `activePageId`, `figmaImportDialogOpen`
  - `document-store.ts` — PenDocument tree CRUD: `addNode`, `updateNode`, `removeNode`, `moveNode`, `reorderNode`, `duplicateNode`, `groupNodes`, `ungroupNode`, `toggleVisibility`, `toggleLock`, `scaleDescendantsInStore`, `rotateDescendantsInStore`, `getNodeById`, `getParentOf`, `getFlatNodes`, `isDescendantOf`; Variable CRUD: `setVariable`, `removeVariable`, `renameVariable`, `setThemes` (all with history support)
  - `document-store-pages.ts` — Page actions extracted from document-store: `addPage`, `removePage`, `renamePage`, `reorderPage`, `duplicatePage`
  - `document-tree-utils.ts` — Pure tree helpers extracted from document-store: `findNodeInTree`, `findParentInTree`, `removeNodeFromTree`, `updateNodeInTree`, `flattenNodes`, `insertNodeInTree`, `isDescendantOf`, `getNodeBounds`, `findClearX`, `scaleChildrenInPlace`, `rotateChildrenInPlace`, `createEmptyDocument`, `DEFAULT_FRAME_ID`
  - `history-store.ts` — Undo/redo (max 300 states), batch mode for grouped operations
  - `ai-store.ts` — Chat messages, streaming state, generated code, model selection, `pendingAttachments` for image uploads
  - `agent-settings-store.ts` — AI provider config (Anthropic/OpenAI/OpenCode/Copilot), MCP CLI integrations (Claude Code, Codex CLI, Gemini CLI, OpenCode CLI, Kiro CLI, Copilot CLI), localStorage persistence
  - `uikit-store.ts` — UIKit management: imported kits, component browser state (search, category filters), localStorage persistence
  - `theme-preset-store.ts` — Theme preset management and persistence
- **`src/types/`** — Type system (9 files):
  - `pen.ts` — PenDocument/PenNode (frame, group, rectangle, ellipse, line, polygon, path, text, image, ref), ContainerProps, `PenPage`; `PenDocument.variables`, `PenDocument.themes`, `PenDocument.pages`
  - `canvas.ts` — ToolType (select, frame, rectangle, ellipse, line, polygon, path, text, hand), ViewportState, SelectionState, CanvasInteraction
  - `styles.ts` — PenFill (solid, linear_gradient, radial_gradient), PenStroke, PenEffect (shadow, blur), BlendMode, StyledTextSegment
  - `variables.ts` — `VariableDefinition` (type + value), `ThemedValue` (value per theme), `VariableValue`
  - `uikit.ts` — UIKit, KitComponent, ComponentCategory types for reusable component organization and browsing
  - `agent-settings.ts` — AI provider config types (`AIProviderType`: anthropic/openai/opencode/copilot, `AIProviderConfig`, `MCPCliIntegration`, `GroupedModel`)
  - `electron.d.ts` — Electron IPC bridge types (file dialogs, save operations, updater: `UpdaterState`/`UpdaterStatus`, `getState`/`checkForUpdates`/`quitAndInstall`/`onStateChange`)
  - `theme-preset.ts` — Type definitions for theme presets
  - `opencode-sdk.d.ts` — Type declarations for @opencode-ai/sdk
- **`src/components/editor/`** — Editor UI (9 files): editor-layout, toolbar (with variables panel toggle), boolean-toolbar (contextual floating toolbar for union/subtract/intersect, shown when 2+ compatible shapes selected), tool-button, shape-tool-dropdown (rectangle/ellipse/line/path + icon picker + image import), top-bar (with `AgentStatusButton`), status-bar, page-tabs (multi-page navigation with context menu), update-ready-banner (Electron auto-updater notification)
- **`src/components/panels/`** — Panels (29 files):
  - `layer-panel.tsx` / `layer-item.tsx` / `layer-context-menu.tsx` — Tree view with drag-and-drop reordering and drop-into-children (above/below/inside), visibility/lock toggles, context menu, rename
  - `property-panel.tsx` — Unified property panel
  - `fill-section.tsx` — Solid + gradient fill, variable picker integration for color binding
  - `stroke-section.tsx` — Stroke color/width/dash, variable picker for stroke color binding
  - `corner-radius-section.tsx` — Unified or 4-point corner radius
  - `size-section.tsx` — Position, size, rotation
  - `text-section.tsx` — Font, size, weight, spacing, alignment
  - `text-layout-section.tsx` — Text node layout controls (auto/fixed-width/fixed-height modes, fill width/height)
  - `icon-section.tsx` — Icon property panel section: current icon name, library dropdown, icon picker
  - `effects-section.tsx` — Shadow and blur
  - `export-section.tsx` — Per-layer export to PNG/SVG with scale options (1x/2x/3x)
  - `layout-section.tsx` — Auto-layout (none/vertical/horizontal), gap, padding, justify, align; variable picker for gap/padding binding
  - `layout-padding-section.tsx` — Extracted padding controls: single/axis/T-R-B-L modes with popover mode switcher
  - `appearance-section.tsx` — Opacity, visibility, lock, flip; variable picker for opacity binding
  - `ai-chat-panel.tsx` / `chat-message.tsx` — AI chat with markdown, design block collapse, apply design, image attachment upload (paperclip button, preview strip, 5MB/4-image limit)
  - `ai-chat-handlers.ts` — `useChatHandlers` hook, `isDesignRequest`, `buildContextString` helpers extracted from ai-chat-panel
  - `ai-chat-checklist.tsx` — `FixedChecklist` component for AI generation progress display
  - `code-panel.tsx` — Code generation output (React/Tailwind, HTML/CSS, CSS Variables, Vue, Svelte, Flutter, SwiftUI, Compose, React Native)
  - `image-section.tsx` — Image property panel section
  - `node-preview-svg.tsx` — Node SVG preview component
  - `right-panel.tsx` — Right panel container
  - `component-browser-panel.tsx` / `component-browser-grid.tsx` / `component-browser-card.tsx` — Resizable floating panel for browsing, importing, and inserting UIKit components with category tabs and search
  - `variables-panel.tsx` — Design variables management: theme axes as tabs, variant columns, resizable floating panel, add/rename/delete themes and variants
  - `variable-row.tsx` — Individual variable row: type icon, editable name, per-theme-variant value cells (color picker, number input, text input), context menu
- **`src/components/shared/`** — Reusable UI (10 files): ColorPicker, NumberInput, SectionHeader, ExportDialog, SaveDialog, AgentSettingsDialog, IconPickerDialog, VariablePicker, FigmaImportDialog, LanguageSelector
- **`src/components/icons/`** — Provider/brand logos: ClaudeLogo, OpenAILogo, OpenCodeLogo, CopilotLogo, FigmaLogo
- **`src/components/ui/`** — shadcn/ui primitives: Button, Select, Separator, Slider, Switch, Toggle, Tooltip
- **`src/services/ai/`** — AI services (31 files + `role-definitions/` subdir with 9 files):
  - `ai-service.ts` — Main AI chat API wrapper, model negotiation, provider selection
  - `ai-prompts.ts` — System prompts for design generation, context building
  - `ai-types.ts` — `ChatMessage` (with `attachments?: ChatAttachment[]`), `ChatAttachment` (id, name, mediaType, data, size), `AIDesignRequest`, `OrchestratorPlan`, streaming response types
  - `ai-runtime-config.ts` — Configuration constants for AI timeouts, thinking modes, effort levels, prompt length limits
  - `model-profiles.ts` — Model capability profiles: adapts thinking mode, effort, timeouts, prompt complexity per model tier (full/standard/basic)
  - `agent-identity.ts` — Agent identity assignment (color, name) for concurrent multi-agent generation
  - `design-generator.ts` — Top-level `generateDesign`/`generateDesignModification` with orchestrator fallback, re-exports from design-parser and design-canvas-ops
  - `design-parser.ts` — Pure JSON/JSONL parsing: `extractJsonFromResponse`, `extractStreamingNodes`, `parseJsonlToTree`, node validation and scoring
  - `design-canvas-ops.ts` — Canvas mutation operations: `insertStreamingNode`, `applyNodesToCanvas`, `upsertNodesToCanvas`, `animateNodesToCanvas`, generation state management, sanitization and heuristics
  - `design-node-sanitization.ts` — Node cloning and merging utilities: `deepCloneNode`, `setNodeChildren`, `mergeNodeForProgressiveUpsert` (extracted from design-canvas-ops)
  - `design-animation.ts` — Fade-in animation coordination for generated design nodes
  - `design-validation.ts` — Post-generation screenshot validation using vision API to detect and auto-fix visual issues
  - `design-validation-fixes.ts` — Auto-fix strategies for validation-detected issues
  - `design-pre-validation.ts` — Pre-validation heuristics before vision API call
  - `design-screenshot.ts` — Canvas screenshot capture for validation pipeline
  - `design-type-presets.ts` — Design type detection and section presets (landing page, dashboard, mobile, etc.)
  - `design-code-generator.ts` — AI-powered code generation from design nodes
  - `design-code-prompts.ts` — Prompts for design-to-code generation
  - `design-system-generator.ts` — Design system token extraction from generated designs
  - `design-system-prompts.ts` — Prompts for design system generation
  - `html-renderer.ts` — PenNode tree to HTML rendering for validation screenshots
  - `visual-ref-orchestrator.ts` — Visual reference-based orchestration for image-guided generation
  - `generation-utils.ts` — Pure utilities for text measurement, size/padding parsing, phone placeholder generation, color extraction
  - `icon-resolver.ts` — Auto-resolves AI-generated icon path nodes by name to verified Lucide SVG paths
  - `role-resolver.ts` — Registry-based system for applying role-specific defaults (button padding, card gaps) and tree post-pass fixes
  - `role-definitions/` — Modular role definition files: index, content, display, interactive, layout, navigation, media, typography, table
  - `orchestrator.ts` — Orchestrator entry point: `executeOrchestration`, `callOrchestrator`, plan parsing
  - `orchestrator-sub-agent.ts` — Sub-agent execution: `executeSubAgentsSequentially`, `executeSubAgent`, prompt building, retry/fallback logic
  - `orchestrator-progress.ts` — `emitProgress`, `buildFinalStepTags` for streaming progress updates
  - `orchestrator-prompts.ts` — Ultra-lightweight orchestrator prompt for spatial decomposition
  - `orchestrator-prompt-optimizer.ts` — Prompt preparation, compression, timeout calculation, fallback plan generation
  - `context-optimizer.ts` — Chat history trimming, sliding window to prevent unbounded context growth
- **`src/services/figma/`** — Figma `.fig` file import pipeline (14 files):
  - `fig-parser.ts` — Binary `.fig` file parser
  - `figma-types.ts` — Figma internal type definitions
  - `figma-node-mapper.ts` — Maps Figma nodes to PenNodes
  - `figma-fill-mapper.ts` — Converts Figma fills to PenFill
  - `figma-stroke-mapper.ts` — Converts Figma strokes to PenStroke
  - `figma-effect-mapper.ts` — Converts Figma effects to PenEffect
  - `figma-layout-mapper.ts` — Maps Figma auto-layout to PenNode layout props
  - `figma-text-mapper.ts` — Converts Figma text styles
  - `figma-vector-decoder.ts` — Decodes Figma vector geometry
  - `figma-color-utils.ts` — Color space conversion utilities
  - `figma-image-resolver.ts` — Resolves image blob references
  - `figma-clipboard.ts` — Figma clipboard paste handling
  - `figma-node-converters.ts` — Figma node conversion utilities
  - `figma-tree-builder.ts` — Figma document tree building
- **`src/services/codegen/`** — Multi-platform code generators (9 files, output `var(--name)` for `$variable` refs):
  - `react-generator.ts` — React + Tailwind CSS
  - `html-generator.ts` — HTML + CSS
  - `css-variables-generator.ts` — CSS Variables from design tokens
  - `vue-generator.ts` — Vue 3 + CSS
  - `svelte-generator.ts` — Svelte + CSS
  - `flutter-generator.ts` — Flutter/Dart
  - `swiftui-generator.ts` — SwiftUI
  - `compose-generator.ts` — Android Jetpack Compose
  - `react-native-generator.ts` — React Native
- **`src/hooks/`** — Hooks (5 files):
  - `use-keyboard-shortcuts.ts` — Global keyboard event handling: tools, clipboard, undo/redo, save, select all, delete, arrow nudge, z-order, boolean operations (Cmd+Alt+U/S/I)
  - `use-electron-menu.ts` — Electron native menu IPC listener: dispatches menu actions (new, open, save, save-as, undo, redo, etc.) to Zustand stores; also handles `onOpenFile` for `.op` file association
  - `use-figma-paste.ts` — Handle Figma clipboard paste into canvas
  - `use-file-drop.ts` — Handle file drag-and-drop onto canvas
  - `use-mcp-sync.ts` — MCP live canvas synchronization hook
- **`src/lib/`** — Utility functions (`utils.ts` with `cn()` for class merging)
- **`src/uikit/`** — UI kit system (3 files + `kits/` subdir):
  - `built-in-registry.ts` — Default built-in UIKit with standard UI components
  - `kit-import-export.ts` — Import/export UIKits from .pen files with variable reference collection
  - `kit-utils.ts` — UIKit utilities: extract components from documents, find reusable nodes, deep clone
  - `kits/` — Default kit data: `default-kit.ts`, `default-kit-meta.ts`
- **`src/mcp/`** — MCP server integration (2 files + `tools/` and `utils/` subdirs):
  - `server.ts` — MCP server entry point, tool registration (stdio + HTTP modes)
  - `document-manager.ts` — MCP utility for reading, writing, and caching PenDocuments from disk; live canvas sync via Nitro API
  - `tools/` — Individual MCP tool implementations:
    - Core: `open-document.ts`, `batch-get.ts`, `get-selection.ts`, `batch-design.ts` (DSL operations), `node-crud.ts` (insert/update/delete/move/copy/replace)
    - Layout: `snapshot-layout.ts`, `find-empty-space.ts`, `import-svg.ts`
    - Variables: `variables.ts`, `theme-presets.ts`
    - Pages: `pages.ts` (add/remove/rename/reorder/duplicate)
    - Layered design: `design-prompt.ts` (segmented retrieval), `design-skeleton.ts`, `design-content.ts`, `design-refine.ts`, `layered-design-defs.ts`
  - `utils/` — Shared utilities: `id.ts`, `node-operations.ts` (page-aware `getDocChildren`/`setDocChildren`), `sanitize.ts`, `svg-node-parser.ts`
- **`src/utils/`** — File operations (save/open .pen), export (PNG/SVG), node clone, pen file normalization (format fixes only, preserves `$variable` refs), SVG parser (import SVG to editable PenNodes), syntax highlight, boolean operations (union/subtract/intersect via Paper.js), `app-storage.ts` (application storage/persistence), `arc-path.ts` (SVG arc utilities), `theme-preset-io.ts` (theme preset file I/O)
- **`src/constants/`** — Application constants: `app.ts` (MCP default port, app-level config)
- **`src/i18n/`** — Internationalization setup and locale files
- **`server/api/ai/`** — Nitro server API (8 files): `chat.ts` (streaming SSE with thinking state, multimodal image attachments per provider), `generate.ts` (non-streaming generation), `connect-agent.ts` (Claude Code/Codex CLI/OpenCode/Copilot connection), `models.ts` (model definitions), `validate.ts` (vision-based post-generation validation), `mcp-install.ts` (MCP server install/uninstall into CLI tool configs; auto-detects `node` availability — if missing, falls back to HTTP URL config `{ type: "http", url }` and auto-starts the MCP HTTP server), `install-agent.ts` (agent installation endpoint), `icon.ts` (icon name → SVG path resolution via local Iconify sets). Supports Anthropic API key or Claude Agent SDK (local OAuth) as dual providers
- **`server/utils/`** — Server utilities (7 files):
  - `resolve-claude-cli.ts` — Resolves standalone `claude` binary path (handles Nitro bundling issues with SDK's `import.meta.url`)
  - `resolve-claude-agent-env.ts` — Builds Claude Agent SDK environment: merges `~/.claude/settings.json` env, validates `ANTHROPIC_CUSTOM_HEADERS`, handles auth token compat
  - `opencode-client.ts` — Shared OpenCode client manager, reuses server on port 4096 with random port fallback
  - `codex-client.ts` — Codex CLI client wrapper with JSON streaming, thinking mode support, timeout handling, optional `imageFiles` for vision queries
  - `copilot-client.ts` — Resolves standalone `copilot` binary path to avoid Bun's `node:sqlite` incompatibility with bundled CLI
  - `mcp-server-manager.ts` — MCP HTTP server lifecycle management: spawn/track/stop detached process, PID file persistence
  - `mcp-sync-state.ts` — In-memory MCP state: current document/selection, SSE broadcast to connected clients

### CanvasKit/Skia Architecture

- **GPU-accelerated WASM rendering** — CanvasKit (Skia compiled to WASM) renders all canvas content via WebGL surface
- **SkiaEngine class** (`skia-engine.ts`) is the core: owns the render loop, viewport transforms, node flattening, and `SpatialIndex` for hit testing
- **Dirty-flag rendering** — `markDirty()` schedules a `requestAnimationFrame` redraw; no continuous rendering loop
- **Node flattening** — `syncFromDocument()` walks the PenDocument tree, resolves auto-layout positions via `canvas-layout-engine.ts`, and produces flat `RenderNode[]` with absolute coordinates
- **SpatialIndex** (`skia-hit-test.ts`) — R-tree backed spatial queries for `hitTest()` (click) and `searchRect()` (marquee selection)
- **Coordinate conversion** — `screenToScene()` / `sceneToScreen()` in `skia-viewport.ts` handle viewport ↔ scene transforms
- **Bitmap caching** — `canvas-zoom-cache.ts` caches rendered frames during zoom/pan for smoother interaction
- **Event handling** — all mouse/keyboard events are handled directly in `skia-canvas.tsx` (no Fabric abstractions)
- **Parent-child transforms** — nodes are flattened to absolute coordinates; `parent-child-transform.ts` propagates transforms to descendants during drag/scale/rotate
- **Legacy Fabric.js** — `fabric` package remains as a dependency for `zoomToFitContent` utility; canvas rendering no longer uses Fabric

### Routing

File-based routing via TanStack Router. Routes in `src/routes/`, auto-generated tree in `src/routeTree.gen.ts` (do not edit).

- `/` — Landing page
- `/editor` — Main design editor

### Path Aliases

`@/*` maps to `./src/*` (configured in `tsconfig.json` and `vite.config.ts`).

### Styling

Tailwind CSS v4 imported via `src/styles.css`. UI primitives from shadcn/ui (`src/components/ui/`). Icons from `lucide-react`. shadcn/ui config in `components.json`.

### Electron Desktop App

- **`electron/main.ts`** — Main process: window creation, Nitro server fork, IPC for native file dialogs, `.op` file association handling (`open-file` event on macOS, CLI args + single-instance lock on Windows/Linux)
- **`electron/preload.ts`** — Context bridge for renderer ↔ main IPC (file dialogs, menu actions, updater state, `onOpenFile`/`readFile` for file association)
- **`electron/app-menu.ts`** — Native application menu configuration (File, Edit, View, Help)
- **`electron/auto-updater.ts`** — Auto-updater implementation: checks GitHub Releases on startup and periodically
- **`electron/constants.ts`** — Electron-specific constants
- **`electron/logger.ts`** — Main process logging
- **`electron-builder.yml`** — Packaging config: macOS (dmg/zip), Windows (nsis/portable), Linux (AppImage/deb), `.op` file association (`fileAssociations`)
- **`scripts/electron-dev.ts`** — Dev workflow: starts Vite → waits for port 3000 → compiles electron/ with esbuild → launches Electron
- Build flow: `BUILD_TARGET=electron bun run build` → `bun run electron:compile` → `npx electron-builder`
- In production, Nitro server is forked as a child process on a random port; Electron loads `http://127.0.0.1:{port}/editor`
- Auto-updater checks GitHub Releases on startup and every hour; `update-ready-banner.tsx` shows download progress and "Restart & Install" prompt
- **File association:** `.op` files are registered as OpenPencil documents via `fileAssociations` in `electron-builder.yml`. On macOS the `open-file` app event handles double-click/drag; on Windows/Linux `requestSingleInstanceLock` + `second-instance` event forwards CLI args to the existing window. Pending file paths are queued until the renderer is ready, then sent via `file:open` IPC channel. The renderer (`use-electron-menu.ts`) listens via `onOpenFile`, reads the file through `file:read` IPC, and calls `loadDocument`.

### CI / CD

- **`.github/workflows/ci.yml`** — Push/PR: type check (`tsc --noEmit`), tests (`vitest`), web build
- **`.github/workflows/build-electron.yml`** — Tag push (`v*`) or manual: builds Electron for macOS, Windows, Linux in parallel, creates draft GitHub Release with all artifacts

## Code Style

- Single files must not exceed 800 lines. Split into smaller modules when they grow beyond this limit.
- One component per file, each with a single responsibility.
- `.ts` and `.tsx` files use kebab-case naming, e.g. `canvas-store.ts`, `use-keyboard-shortcuts.ts`.
- UI components must use shadcn/ui design tokens (`bg-card`, `text-foreground`, `border-border`, etc.). No hardcoded Tailwind colors like `gray-*`, `blue-*`.
- Toolbar button active state uses `isActive` conditional className (`bg-primary text-primary-foreground`), not Radix Toggle's `data-[state=on]:` selector (has twMerge conflicts).

## Git Commit Convention

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <subject>

<body>
```

### Type

- `feat` — New feature
- `fix` — Bug fix
- `refactor` — Refactoring (no behavior change)
- `perf` — Performance optimization
- `style` — Code formatting (no logic change)
- `docs` — Documentation
- `test` — Tests
- `chore` — Build / tooling / dependency changes

### Scope

By module: `editor`, `canvas`, `panels`, `history`, `ai`, `codegen`, `store`, `types`, `variables`, `figma`, `mcp`, `electron`.

### Rules

- Subject in English, lowercase start, no period, imperative mood (e.g. `add`, `fix`, `remove`).
- Body is optional; explain **why** not what.
- One commit per change. Do not mix unrelated changes in a single commit.

## License

MIT License. See [LICENSE](./LICENSE) for details.
