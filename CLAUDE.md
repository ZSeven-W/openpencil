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
│   ├── pen-core/      Document tree ops, layout engine, variables, boolean ops
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

#### Packages (`packages/`)

- **`pen-types/src/`** — Type definitions (9 files):
  - `pen.ts` — PenDocument/PenNode (frame, group, rectangle, ellipse, line, polygon, path, text, image, ref), ContainerProps, `PenPage`; `PenDocument.variables`, `PenDocument.themes`, `PenDocument.pages`
  - `canvas.ts` — ToolType (select, frame, rectangle, ellipse, line, polygon, path, text, hand), ViewportState, SelectionState, CanvasInteraction
  - `styles.ts` — PenFill (solid, linear_gradient, radial_gradient), PenStroke, PenEffect (shadow, blur), BlendMode, StyledTextSegment
  - `variables.ts` — `VariableDefinition` (type + value), `ThemedValue` (value per theme), `VariableValue`
  - `uikit.ts` — UIKit, KitComponent, ComponentCategory types
  - `agent-settings.ts` — AI provider config types (`AIProviderType`: anthropic/openai/opencode/copilot, `AIProviderConfig`, `MCPCliIntegration`, `GroupedModel`)
  - `electron.d.ts` — Electron IPC bridge types (file dialogs, save operations, updater)
  - `theme-preset.ts` — Theme preset types
  - `opencode-sdk.d.ts` — Type declarations for @opencode-ai/sdk
- **`pen-core/src/`** — Core document operations (11 files + `layout/` + `variables/` subdirs):
  - `tree-utils.ts` — Pure tree helpers: `findNodeInTree`, `findParentInTree`, `removeNodeFromTree`, `updateNodeInTree`, `flattenNodes`, `insertNodeInTree`, `isDescendantOf`, `getNodeBounds`, `findClearX`, `scaleChildrenInPlace`, `rotateChildrenInPlace`, `createEmptyDocument`, `DEFAULT_FRAME_ID`
  - `normalize.ts` — Pen file normalization (format fixes only, preserves `$variable` refs)
  - `boolean-ops.ts` — Union/subtract/intersect via Paper.js
  - `sync-lock.ts` — Prevents circular sync loops
  - `arc-path.ts` — SVG arc utilities
  - `font-utils.ts` — Font utilities
  - `node-helpers.ts` — Node helper functions
  - `constants.ts` — Core constants
  - `id.ts` — ID generation
  - `layout/engine.ts` — Auto-layout computation: `resolvePadding`, `getNodeWidth/Height`, `computeLayoutPositions`, `Padding` interface
  - `layout/text-measure.ts` — Text width/height estimation, CJK detection, `parseSizing`
  - `variables/resolve.ts` — Core resolution: `resolveVariableRef`, `resolveNodeForCanvas`, `getDefaultTheme`, `isVariableRef`
  - `variables/replace-refs.ts` — `replaceVariableRefsInTree`: recursively walk node tree to replace/resolve `$refs`
- **`pen-codegen/src/`** — Multi-platform code generators (9 files, output `var(--name)` for `$variable` refs):
  - `react-generator.ts` — React + Tailwind CSS
  - `html-generator.ts` — HTML + CSS
  - `css-variables-generator.ts` — CSS Variables from design tokens
  - `vue-generator.ts` — Vue 3 + CSS
  - `svelte-generator.ts` — Svelte + CSS
  - `flutter-generator.ts` — Flutter/Dart
  - `swiftui-generator.ts` — SwiftUI
  - `compose-generator.ts` — Android Jetpack Compose
  - `react-native-generator.ts` — React Native
- **`pen-figma/src/`** — Figma `.fig` file import pipeline (17 files):
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
- **`pen-renderer/src/`** — Standalone CanvasKit/Skia renderer (13 files):
  - `renderer.ts` — Core renderer class
  - `document-flattener.ts` — Document tree flattening with layout resolution
  - `node-renderer.ts` — Node-level draw calls
  - `text-renderer.ts` — Text rendering
  - `paint-utils.ts` — Color parsing, gradient creation
  - `path-utils.ts` — SVG path conversion
  - `image-loader.ts` — Async image loading and caching
  - `font-manager.ts` — Font management
  - `spatial-index.ts` — R-tree backed spatial queries
  - `viewport.ts` — Viewport math
  - `init.ts` — CanvasKit WASM loader
  - `types.ts` — Renderer-specific types
- **`pen-sdk/src/`** — Umbrella SDK (1 file): `index.ts` re-exports all packages

#### Web App (`apps/web/`)

- **`src/canvas/`** — Canvas engine (14 files + `skia/` subdir with 12 files):
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
    - `skia-font-manager.ts` — Font management for CanvasKit
  - **Shared modules:**
    - `canvas-sync-lock.ts` — Prevents circular sync loops
    - `canvas-sync-utils.ts` — `forcePageResync()` utility for page-aware canvas re-sync
    - `canvas-constants.ts` — Default colors, zoom limits, stroke widths
    - `canvas-node-creator.ts` — `createNodeForTool`, `isDrawingTool` helpers
    - `canvas-layout-engine.ts` — Auto-layout computation (delegates to `@zseven-w/pen-core`)
    - `canvas-text-measure.ts` — Text width/height estimation, CJK detection
    - `font-utils.ts` — Font utilities
    - `node-helpers.ts` — Node helper functions
    - `insertion-indicator.ts` — Drag-and-drop insertion indicator
    - `selection-context.ts` — Multi-select context management
    - `agent-indicator.ts` — Agent visual indicators on canvas during concurrent AI generation
    - `use-layout-indicator.ts` — Layout indicator overlay
    - `skia-engine-ref.ts` — SkiaEngine singleton ref
- **`src/stores/`** — Zustand stores (9 files):
  - `canvas-store.ts` — UI/tool/selection/viewport/clipboard/interaction state, `variablesPanelOpen` toggle, `activePageId`, `figmaImportDialogOpen`
  - `document-store.ts` — PenDocument tree CRUD: `addNode`, `updateNode`, `removeNode`, `moveNode`, `reorderNode`, `duplicateNode`, `groupNodes`, `ungroupNode`, `toggleVisibility`, `toggleLock`, `scaleDescendantsInStore`, `rotateDescendantsInStore`, `getNodeById`, `getParentOf`, `getFlatNodes`, `isDescendantOf`; Variable CRUD: `setVariable`, `removeVariable`, `renameVariable`, `setThemes` (all with history support)
  - `document-store-pages.ts` — Page actions: `addPage`, `removePage`, `renamePage`, `reorderPage`, `duplicatePage`
  - `document-tree-utils.ts` — Pure tree helpers (delegates to `@zseven-w/pen-core`)
  - `history-store.ts` — Undo/redo (max 300 states), batch mode for grouped operations
  - `ai-store.ts` — Chat messages, streaming state, generated code, model selection, `pendingAttachments` for image uploads
  - `agent-settings-store.ts` — AI provider config (Anthropic/OpenAI/OpenCode/Copilot), MCP CLI integrations (Claude Code, Codex CLI, Gemini CLI, OpenCode CLI, Kiro CLI, Copilot CLI), localStorage persistence
  - `uikit-store.ts` — UIKit management: imported kits, component browser state, localStorage persistence
  - `theme-preset-store.ts` — Theme preset management and persistence
- **`src/components/editor/`** — Editor UI (9 files): editor-layout, toolbar (with variables panel toggle), boolean-toolbar (contextual floating toolbar for union/subtract/intersect), tool-button, shape-tool-dropdown (rectangle/ellipse/line/path + icon picker + image import), top-bar (with `AgentStatusButton`), status-bar, page-tabs (multi-page navigation with context menu), update-ready-banner (Electron auto-updater notification)
- **`src/components/panels/`** — Panels (32 files):
  - `layer-panel.tsx` / `layer-item.tsx` / `layer-context-menu.tsx` — Tree view with drag-and-drop reordering and drop-into-children, visibility/lock toggles, context menu, rename
  - `property-panel.tsx` — Unified property panel
  - `fill-section.tsx` — Solid + gradient fill, variable picker integration for color binding
  - `stroke-section.tsx` — Stroke color/width/dash, variable picker for stroke color binding
  - `corner-radius-section.tsx` — Unified or 4-point corner radius
  - `size-section.tsx` — Position, size, rotation
  - `text-section.tsx` — Font, size, weight, spacing, alignment
  - `text-layout-section.tsx` — Text node layout controls (auto/fixed-width/fixed-height modes)
  - `icon-section.tsx` — Icon property panel section
  - `effects-section.tsx` — Shadow and blur
  - `export-section.tsx` — Per-layer export to PNG/SVG with scale options
  - `layout-section.tsx` — Auto-layout (none/vertical/horizontal), gap, padding, justify, align
  - `layout-padding-section.tsx` — Extracted padding controls: single/axis/T-R-B-L modes
  - `appearance-section.tsx` — Opacity, visibility, lock, flip
  - `image-section.tsx` / `image-fill-popover.tsx` / `image-generate-popover.tsx` / `image-search-popover.tsx` — Image property panel with AI generation and search
  - `ai-chat-panel.tsx` / `chat-message.tsx` / `ai-chat-handlers.ts` / `ai-chat-checklist.tsx` — AI chat with markdown, design block collapse, apply design, image attachment
  - `code-panel.tsx` — Code generation output (React/Tailwind, HTML/CSS, CSS Variables, Vue, Svelte, Flutter, SwiftUI, Compose, React Native)
  - `node-preview-svg.tsx` — Node SVG preview component
  - `right-panel.tsx` — Right panel container
  - `component-browser-panel.tsx` / `component-browser-grid.tsx` / `component-browser-card.tsx` — UIKit component browser
  - `variables-panel.tsx` / `variable-row.tsx` — Design variables management
- **`src/components/shared/`** — Reusable UI (12 files): ColorPicker, NumberInput, SectionHeader, ExportDialog, SaveDialog, AgentSettingsDialog, AgentSettingsImagesPage, IconPickerDialog, VariablePicker, FigmaImportDialog, FontPicker, LanguageSelector
- **`src/components/icons/`** — Provider/brand logos: ClaudeLogo, OpenAILogo, OpenCodeLogo, CopilotLogo, FigmaLogo
- **`src/components/ui/`** — shadcn/ui primitives: Button, Select, Separator, Slider, Switch, Toggle, Tooltip
- **`src/services/ai/`** — AI services (35 files + `role-definitions/` + `design-principles/` subdirs):
  - `ai-service.ts` — Main AI chat API wrapper, model negotiation, provider selection
  - `ai-prompts.ts` — System prompts for design generation, context building
  - `ai-types.ts` — `ChatMessage` (with `attachments?: ChatAttachment[]`), `ChatAttachment`, `AIDesignRequest`, `OrchestratorPlan`, streaming response types
  - `ai-runtime-config.ts` — Configuration constants for AI timeouts, thinking modes, effort levels, prompt length limits
  - `model-profiles.ts` — Model capability profiles: adapts thinking mode, effort, timeouts, prompt complexity per model tier (full/standard/basic)
  - `agent-identity.ts` — Agent identity assignment (color, name) for concurrent multi-agent generation
  - `design-generator.ts` — Top-level `generateDesign`/`generateDesignModification` with orchestrator fallback
  - `design-parser.ts` — Pure JSON/JSONL parsing: `extractJsonFromResponse`, `extractStreamingNodes`, `parseJsonlToTree`
  - `design-canvas-ops.ts` — Canvas mutation operations: `insertStreamingNode`, `applyNodesToCanvas`, `upsertNodesToCanvas`, `animateNodesToCanvas`
  - `design-node-sanitization.ts` — Node cloning and merging utilities
  - `design-animation.ts` — Fade-in animation coordination for generated design nodes
  - `design-validation.ts` — Post-generation screenshot validation using vision API
  - `design-validation-fixes.ts` — Auto-fix strategies for validation-detected issues
  - `design-pre-validation.ts` — Pre-validation heuristics before vision API call
  - `design-screenshot.ts` — Canvas screenshot capture for validation pipeline
  - `design-type-presets.ts` — Design type detection and section presets
  - `design-code-generator.ts` — AI-powered code generation from design nodes
  - `design-code-prompts.ts` — Prompts for design-to-code generation
  - `design-system-generator.ts` — Design system token extraction
  - `design-system-prompts.ts` — Prompts for design system generation
  - `html-renderer.ts` — PenNode tree to HTML rendering for validation screenshots
  - `visual-ref-orchestrator.ts` — Visual reference-based orchestration for image-guided generation
  - `generation-utils.ts` — Pure utilities for text measurement, size/padding parsing, color extraction
  - `icon-resolver.ts` — Auto-resolves AI-generated icon path nodes by name to verified Lucide SVG paths
  - `image-search-pipeline.ts` — AI-powered image search pipeline
  - `role-resolver.ts` — Registry-based system for applying role-specific defaults
  - `role-definitions/` — Modular role definition files: index, content, display, interactive, layout, navigation, media, typography, table
  - `design-principles/` — Design principle reference files
  - `orchestrator.ts` — Orchestrator entry point: `executeOrchestration`, `callOrchestrator`, plan parsing
  - `orchestrator-sub-agent.ts` — Sub-agent execution: `executeSubAgentsSequentially`, prompt building, retry/fallback logic
  - `orchestrator-progress.ts` — `emitProgress`, `buildFinalStepTags` for streaming progress updates
  - `orchestrator-prompts.ts` — Ultra-lightweight orchestrator prompt for spatial decomposition
  - `orchestrator-prompt-optimizer.ts` — Prompt preparation, compression, timeout calculation, fallback plan generation
  - `context-optimizer.ts` — Chat history trimming, sliding window to prevent unbounded context growth
- **`src/hooks/`** — Hooks (6 files):
  - `use-keyboard-shortcuts.ts` — Global keyboard event handling: tools, clipboard, undo/redo, save, select all, delete, arrow nudge, z-order, boolean operations (Cmd+Alt+U/S/I)
  - `use-electron-menu.ts` — Electron native menu IPC listener: dispatches menu actions to Zustand stores; handles `onOpenFile` for `.op` file association
  - `use-figma-paste.ts` — Handle Figma clipboard paste into canvas
  - `use-file-drop.ts` — Handle file drag-and-drop onto canvas
  - `use-mcp-sync.ts` — MCP live canvas synchronization hook
  - `use-system-fonts.ts` — System font detection
- **`src/lib/`** — Utility functions (`utils.ts` with `cn()` for class merging)
- **`src/uikit/`** — UI kit system (3 files + `kits/` subdir):
  - `built-in-registry.ts` — Default built-in UIKit with standard UI components
  - `kit-import-export.ts` — Import/export UIKits from .pen files
  - `kit-utils.ts` — UIKit utilities: extract components, find reusable nodes, deep clone
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
- **`src/utils/`** — File operations (12 files): save/open .pen (`file-operations.ts`), export PNG/SVG (`export.ts`), node clone (`node-clone.ts`), pen file normalization (`normalize-pen-file.ts`), SVG parser (`svg-parser.ts`), syntax highlight (`syntax-highlight.ts`), boolean operations (`boolean-ops.ts`), `app-storage.ts`, `arc-path.ts`, `theme-preset-io.ts`, `id.ts`
- **`src/constants/`** — Application constants: `app.ts` (MCP default port, app-level config)
- **`src/i18n/`** — Internationalization setup and locale files

#### Server API (`apps/web/server/`)

- **`api/ai/`** — Nitro server API (11 files): `chat.ts` (streaming SSE with thinking state, multimodal image attachments per provider), `generate.ts` (non-streaming generation), `connect-agent.ts` (Claude Code/Codex CLI/OpenCode/Copilot connection), `models.ts` (model definitions), `validate.ts` (vision-based post-generation validation), `mcp-install.ts` (MCP server install/uninstall into CLI tool configs; auto-detects `node` availability — if missing, falls back to HTTP URL config), `install-agent.ts` (agent installation endpoint), `icon.ts` (icon name → SVG path resolution via local Iconify sets), `image-generate.ts` (AI image generation), `image-search.ts` (AI image search), `image-service-test.ts` (image service testing). Supports Anthropic API key or Claude Agent SDK (local OAuth) as dual providers
- **`utils/`** — Server utilities (8 files):
  - `resolve-claude-cli.ts` — Resolves standalone `claude` binary path
  - `resolve-claude-agent-env.ts` — Builds Claude Agent SDK environment
  - `opencode-client.ts` — Shared OpenCode client manager
  - `codex-client.ts` — Codex CLI client wrapper
  - `copilot-client.ts` — Resolves standalone `copilot` binary path
  - `mcp-server-manager.ts` — MCP HTTP server lifecycle management
  - `mcp-sync-state.ts` — In-memory MCP state: current document/selection, SSE broadcast
  - `server-logger.ts` — Server-side logging utility

#### Desktop App (`apps/desktop/`)

- **`main.ts`** — Main process: window creation, Nitro server fork, IPC for native file dialogs, `.op` file association handling (`open-file` event on macOS, CLI args + single-instance lock on Windows/Linux)
- **`preload.ts`** — Context bridge for renderer ↔ main IPC (file dialogs, menu actions, updater state, `onOpenFile`/`readFile` for file association)
- **`app-menu.ts`** — Native application menu configuration (File, Edit, View, Help)
- **`auto-updater.ts`** — Auto-updater implementation: checks GitHub Releases on startup and periodically
- **`constants.ts`** — Electron-specific constants
- **`logger.ts`** — Main process logging
- **`dev.ts`** — Dev workflow: starts Vite → waits for port 3000 → compiles MCP → compiles Electron → launches Electron
- **`electron-builder.yml`** — Packaging config: macOS (dmg/zip), Windows (nsis/portable), Linux (AppImage/deb), `.op` file association (`fileAssociations`)
- **`build/`** — Platform icons (.icns, .ico, .png)
- Build flow: `BUILD_TARGET=electron bun run build` → `bun run electron:compile` → `bun run mcp:compile` → `npx electron-builder --config apps/desktop/electron-builder.yml`
- In production, Nitro server is forked as a child process on a random port; Electron loads `http://127.0.0.1:{port}/editor`
- Auto-updater checks GitHub Releases on startup and every hour; `update-ready-banner.tsx` shows download progress and "Restart & Install" prompt
- **File association:** `.op` files are registered as OpenPencil documents via `fileAssociations` in `electron-builder.yml`. On macOS the `open-file` app event handles double-click/drag; on Windows/Linux `requestSingleInstanceLock` + `second-instance` event forwards CLI args to the existing window.

### CanvasKit/Skia Architecture

- **GPU-accelerated WASM rendering** — CanvasKit (Skia compiled to WASM) renders all canvas content via WebGL surface
- **SkiaEngine class** (`skia-engine.ts`) is the core: owns the render loop, viewport transforms, node flattening, and `SpatialIndex` for hit testing
- **Dirty-flag rendering** — `markDirty()` schedules a `requestAnimationFrame` redraw; no continuous rendering loop
- **Node flattening** — `syncFromDocument()` walks the PenDocument tree, resolves auto-layout positions via layout engine, and produces flat `RenderNode[]` with absolute coordinates
- **SpatialIndex** (`skia-hit-test.ts`) — R-tree backed spatial queries for `hitTest()` (click) and `searchRect()` (marquee selection)
- **Coordinate conversion** — `screenToScene()` / `sceneToScreen()` in `skia-viewport.ts` handle viewport ↔ scene transforms
- **Event handling** — all mouse/keyboard events are handled directly in `skia-canvas.tsx`
- **Parent-child transforms** — nodes are flattened to absolute coordinates; transforms propagate to descendants during drag/scale/rotate

### Routing

File-based routing via TanStack Router. Routes in `apps/web/src/routes/`, auto-generated tree in `apps/web/src/routeTree.gen.ts` (do not edit).

- `/` — Landing page
- `/editor` — Main design editor

### Path Aliases

`@/*` maps to `./src/*` (configured in `apps/web/tsconfig.json` and `apps/web/vite.config.ts`).

### Styling

Tailwind CSS v4 imported via `apps/web/src/styles.css`. UI primitives from shadcn/ui (`apps/web/src/components/ui/`). Icons from `lucide-react`. shadcn/ui config in `apps/web/components.json`.

### CI / CD

- **`.github/workflows/ci.yml`** — Push/PR: type check (`tsc --noEmit`), tests (`vitest`), web build
- **`.github/workflows/build-electron.yml`** — Tag push (`v*`) or manual: builds Electron for macOS, Windows, Linux in parallel, creates draft GitHub Release with all artifacts
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

By module: `editor`, `canvas`, `panels`, `history`, `ai`, `codegen`, `store`, `types`, `variables`, `figma`, `mcp`, `electron`, `renderer`, `sdk`.

### Rules

- Subject in English, lowercase start, no period, imperative mood (e.g. `add`, `fix`, `remove`).
- Body is optional; explain **why** not what.
- One commit per change. Do not mix unrelated changes in a single commit.

## License

MIT License. See [LICENSE](./LICENSE) for details.
