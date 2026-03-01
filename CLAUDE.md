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

**Key technologies:** React 19, Fabric.js v7 (canvas engine), Zustand v5 (state management), TanStack Router (file-based routing), Tailwind CSS v4, shadcn/ui (UI primitives), Vite 7, Nitro (server), Electron 35 (desktop), TypeScript (strict mode).

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
   Fabric.js Canvas      canvas-sync-lock
   (imperative render)   (prevents circular sync)
```

- **document-store** is the single source of truth. Fabric.js only renders.
- User edits on canvas → Fabric events → update document-store (with sync lock)
- User edits in panels → update document-store → `use-canvas-sync` updates Fabric
- `canvas-sync-lock.ts` prevents circular updates when Fabric events write to the store

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
- `resolveNodeForCanvas()` resolves `$refs` on-the-fly before Fabric.js rendering
- Code generators output `var(--name)` for `$ref` values
- Multiple theme axes supported (e.g. Theme-1 with Light/Dark, Theme-2 with Compact/Comfortable)
- Each theme axis has variants; variables can have per-variant values (`ThemedValue[]`)

### Key Modules

- **`src/canvas/`** — Fabric.js integration (30 files):
  - `fabric-canvas.tsx` — Canvas component initialization
  - `use-fabric-canvas.ts` — Canvas initialization hook
  - `canvas-object-factory.ts` — Creates Fabric objects from PenNodes (rect, ellipse, line, polygon, path, text, image, frame, group)
  - `canvas-object-sync.ts` — Syncs individual object properties between Fabric and store
  - `canvas-sync-lock.ts` — Prevents circular sync loops
  - `canvas-sync-utils.ts` — `forcePageResync()` utility for page-aware canvas re-sync
  - `canvas-controls.ts` — Custom rotation controls and cursor styling
  - `canvas-constants.ts` — Default colors, zoom limits, stroke widths
  - `use-canvas-events.ts` — Drawing events, shape creation, smart guides activation, tool-based `skipTargetFind` management
  - `canvas-node-creator.ts` — `createNodeForTool`, `isDrawingTool`, `toScene` helpers extracted from use-canvas-events
  - `canvas-object-modified.ts` — `syncObjToStore`, `syncSelectionToStore`, `handleObjectModified` — Fabric `object:modified` handler logic
  - `use-canvas-sync.ts` — Bidirectional PenDocument ↔ Fabric.js sync, node flattening with parent offsets, variable resolution via `resolveNodeForCanvas()`
  - `canvas-layout-engine.ts` — Auto-layout computation: `resolvePadding`, `getNodeWidth/Height`, `computeLayoutPositions`, `Padding` interface
  - `canvas-text-measure.ts` — Text width/height estimation, CJK detection, `parseSizing`, `getTextOpticalCenterYOffset`
  - `use-canvas-viewport.ts` — Wheel zoom, space+drag panning, tool cursor switching, selection toggling per tool
  - `use-canvas-selection.ts` — Selection sync between Fabric objects and canvas-store
  - `use-canvas-hover.ts` — Hover state management for objects
  - `use-canvas-guides.ts` — Smart alignment guides with snapping
  - `guide-utils.ts` — Guide calculation and rendering
  - `pen-tool.ts` — Bezier pen tool: anchor points, control handles, path closure, preview rendering
  - `parent-child-transform.ts` — Propagates parent transforms (move/scale/rotate) to children proportionally
  - `use-dimension-label.ts` — Shows size/position labels during object manipulation
  - `use-frame-labels.ts` — Renders frame names and boundaries on canvas
  - `use-entered-frame-overlay.ts` — Visual overlay when entering a frame for editing
  - `use-layout-indicator.ts` — Layout indicator rendering during drag operations
  - `insertion-indicator.ts` — Insertion point indicator for layout drop targets
  - `drag-into-layout.ts` — Drag-and-drop into auto-layout frames with insertion detection
  - `drag-reparent.ts` — Reparenting nodes during drag operations
  - `layout-reorder.ts` — Reorder children within layout frames during drag
  - `selection-context.ts` — Selection context management for multi-select operations
- **`src/variables/`** — Design variables system (2 files):
  - `resolve-variables.ts` — Core resolution utilities: `resolveVariableRef`, `resolveNodeForCanvas`, `getDefaultTheme`, `isVariableRef`; resolves `$variable` references to concrete values for canvas rendering with circular reference guards
  - `replace-refs.ts` — `replaceVariableRefsInTree`: recursively walk node tree to replace/resolve `$refs` when renaming or deleting variables (covers opacity, gap, padding, fills, strokes, effects, text)
- **`src/stores/`** — Zustand stores (8 files):
  - `canvas-store.ts` — UI/tool/selection/viewport/clipboard/interaction state, `variablesPanelOpen` toggle, `activePageId`, `figmaImportDialogOpen`
  - `document-store.ts` — PenDocument tree CRUD: `addNode`, `updateNode`, `removeNode`, `moveNode`, `reorderNode`, `duplicateNode`, `groupNodes`, `ungroupNode`, `toggleVisibility`, `toggleLock`, `scaleDescendantsInStore`, `rotateDescendantsInStore`, `getNodeById`, `getParentOf`, `getFlatNodes`, `isDescendantOf`; Variable CRUD: `setVariable`, `removeVariable`, `renameVariable`, `setThemes` (all with history support)
  - `document-store-pages.ts` — Page actions extracted from document-store: `addPage`, `removePage`, `renamePage`, `reorderPage`, `duplicatePage`
  - `document-tree-utils.ts` — Pure tree helpers extracted from document-store: `findNodeInTree`, `findParentInTree`, `removeNodeFromTree`, `updateNodeInTree`, `flattenNodes`, `insertNodeInTree`, `isDescendantOf`, `getNodeBounds`, `findClearX`, `scaleChildrenInPlace`, `rotateChildrenInPlace`, `createEmptyDocument`, `DEFAULT_FRAME_ID`
  - `history-store.ts` — Undo/redo (max 300 states), batch mode for grouped operations
  - `ai-store.ts` — Chat messages, streaming state, generated code, model selection, `pendingAttachments` for image uploads
  - `agent-settings-store.ts` — AI provider config (Anthropic/OpenAI/OpenCode), MCP CLI integrations (Claude Code, Codex CLI, Gemini CLI, OpenCode CLI, Kiro CLI), localStorage persistence
  - `uikit-store.ts` — UIKit management: imported kits, component browser state (search, category filters), localStorage persistence
- **`src/types/`** — Type system (8 files):
  - `pen.ts` — PenDocument/PenNode (frame, group, rectangle, ellipse, line, polygon, path, text, image, ref), ContainerProps, `PenPage`; `PenDocument.variables`, `PenDocument.themes`, `PenDocument.pages`
  - `canvas.ts` — ToolType (select, frame, rectangle, ellipse, line, polygon, path, text, hand), ViewportState, SelectionState, CanvasInteraction
  - `styles.ts` — PenFill (solid, linear_gradient, radial_gradient), PenStroke, PenEffect (shadow, blur), BlendMode, StyledTextSegment
  - `variables.ts` — `VariableDefinition` (type + value), `ThemedValue` (value per theme), `VariableValue`
  - `uikit.ts` — UIKit, KitComponent, ComponentCategory types for reusable component organization and browsing
  - `agent-settings.ts` — AI provider config types (`AIProviderType`, `AIProviderConfig`, `MCPCliIntegration`, `GroupedModel`)
  - `electron.d.ts` — Electron IPC bridge types (file dialogs, save operations, updater: `UpdaterState`/`UpdaterStatus`, `getState`/`checkForUpdates`/`quitAndInstall`/`onStateChange`)
  - `opencode-sdk.d.ts` — Type declarations for @opencode-ai/sdk
- **`src/components/editor/`** — Editor UI (8 files): editor-layout, toolbar (with variables panel toggle), tool-button, shape-tool-dropdown (rectangle/ellipse/line/path + icon picker + image import), top-bar (with `AgentStatusButton`), status-bar, page-tabs (multi-page navigation with context menu), update-ready-banner (Electron auto-updater notification)
- **`src/components/panels/`** — Panels (26 files):
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
  - `code-panel.tsx` — Code generation output (React/Tailwind, HTML/CSS, CSS Variables)
  - `component-browser-panel.tsx` / `component-browser-grid.tsx` / `component-browser-card.tsx` — Resizable floating panel for browsing, importing, and inserting UIKit components with category tabs and search
  - `variables-panel.tsx` — Design variables management: theme axes as tabs, variant columns, resizable floating panel, add/rename/delete themes and variants
  - `variable-row.tsx` — Individual variable row: type icon, editable name, per-theme-variant value cells (color picker, number input, text input), context menu
- **`src/components/shared/`** — Reusable UI (10 files): ColorPicker, NumberInput, DropdownSelect, SectionHeader, ExportDialog, SaveDialog, AgentSettingsDialog, IconPickerDialog, VariablePicker, FigmaImportDialog
- **`src/components/icons/`** — Provider/brand logos: ClaudeLogo, OpenAILogo, OpenCodeLogo, FigmaLogo
- **`src/components/ui/`** — shadcn/ui primitives: Button, Select, Separator, Slider, Switch, Toggle, Tooltip
- **`src/services/ai/`** — AI services (20 files):
  - `ai-service.ts` — Main AI chat API wrapper, model negotiation, provider selection
  - `ai-prompts.ts` — System prompts for design generation, context building
  - `ai-types.ts` — `ChatMessage` (with `attachments?: ChatAttachment[]`), `ChatAttachment` (id, name, mediaType, data, size), `AIDesignRequest`, `OrchestratorPlan`, streaming response types
  - `ai-runtime-config.ts` — Configuration constants for AI timeouts, thinking modes, effort levels, prompt length limits
  - `design-generator.ts` — Top-level `generateDesign`/`generateDesignModification` with orchestrator fallback, re-exports from design-parser and design-canvas-ops
  - `design-parser.ts` — Pure JSON/JSONL parsing: `extractJsonFromResponse`, `extractStreamingNodes`, `parseJsonlToTree`, node validation and scoring
  - `design-canvas-ops.ts` — Canvas mutation operations: `insertStreamingNode`, `applyNodesToCanvas`, `upsertNodesToCanvas`, `animateNodesToCanvas`, generation state management, sanitization and heuristics
  - `design-node-sanitization.ts` — Node cloning and merging utilities: `deepCloneNode`, `setNodeChildren`, `mergeNodeForProgressiveUpsert` (extracted from design-canvas-ops)
  - `design-animation.ts` — Fade-in animation coordination for generated design nodes
  - `design-validation.ts` — Post-generation screenshot validation using vision API to detect and auto-fix visual issues
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
- **`src/services/figma/`** — Figma `.fig` file import pipeline (11 files):
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
- **`src/services/codegen/`** — React+Tailwind and HTML+CSS code generators (output `var(--name)` for `$variable` refs), CSS variables generator
- **`src/hooks/`** — Hooks (2 files):
  - `use-keyboard-shortcuts.ts` — Global keyboard event handling: tools, clipboard, undo/redo, save, select all, delete, arrow nudge, z-order
  - `use-electron-menu.ts` — Electron native menu IPC listener: dispatches menu actions (new, open, save, save-as, undo, redo, etc.) to Zustand stores
- **`src/lib/`** — Utility functions (`utils.ts` with `cn()` for class merging)
- **`src/uikit/`** — UI kit system (3 files + `kits/` subdir):
  - `built-in-registry.ts` — Default built-in UIKit with standard UI components
  - `kit-import-export.ts` — Import/export UIKits from .pen files with variable reference collection
  - `kit-utils.ts` — UIKit utilities: extract components from documents, find reusable nodes, deep clone
  - `kits/` — Default kit data: `default-kit.ts`, `default-kit-meta.ts`
- **`src/mcp/`** — MCP server integration (2 files + `tools/` and `utils/` subdirs):
  - `server.ts` — MCP server entry point, tool registration
  - `document-manager.ts` — MCP utility for reading, writing, and caching PenDocuments from disk
  - `tools/` — Individual MCP tool implementations: `batch-design.ts`, `batch-get.ts`, `find-empty-space.ts`, `open-document.ts`, `snapshot-layout.ts`, `variables.ts`
  - `utils/` — Shared utilities: `id.ts`, `node-operations.ts`
- **`src/utils/`** — File operations (save/open .pen), export (PNG/SVG), node clone, pen file normalization (format fixes only, preserves `$variable` refs), SVG parser (import SVG to editable PenNodes), syntax highlight
- **`server/api/ai/`** — Nitro server API (7 files): `chat.ts` (streaming SSE with thinking state, multimodal image attachments per provider), `generate.ts` (non-streaming generation), `connect-agent.ts` (Claude Code/Codex CLI/OpenCode connection), `models.ts` (model definitions), `validate.ts` (vision-based post-generation validation), `mcp-install.ts` (MCP server install/uninstall into CLI tool configs), `icon.ts` (icon name → SVG path resolution via local Iconify sets). Supports Anthropic API key or Claude Agent SDK (local OAuth) as dual providers
- **`server/utils/`** — Server utilities (4 files):
  - `resolve-claude-cli.ts` — Resolves standalone `claude` binary path (handles Nitro bundling issues with SDK's `import.meta.url`)
  - `resolve-claude-agent-env.ts` — Builds Claude Agent SDK environment: merges `~/.claude/settings.json` env, validates `ANTHROPIC_CUSTOM_HEADERS`, handles auth token compat
  - `opencode-client.ts` — Shared OpenCode client manager, reuses server on port 4096 with random port fallback
  - `codex-client.ts` — Codex CLI client wrapper with JSON streaming, thinking mode support, timeout handling, optional `imageFiles` for vision queries

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

### Electron Desktop App

- **`electron/main.ts`** — Main process: window creation, Nitro server fork, IPC for native file dialogs, native application menu, auto-updater, macOS traffic-light padding (auto-hidden in fullscreen)
- **`electron/preload.ts`** — Context bridge for renderer ↔ main IPC (file dialogs, menu actions, updater state)
- **`electron-builder.yml`** — Packaging config: macOS (dmg/zip), Windows (nsis/portable), Linux (AppImage/deb)
- **`scripts/electron-dev.ts`** — Dev workflow: starts Vite → waits for port 3000 → compiles electron/ with esbuild → launches Electron
- Build flow: `BUILD_TARGET=electron bun run build` → `bun run electron:compile` → `npx electron-builder`
- In production, Nitro server is forked as a child process on a random port; Electron loads `http://127.0.0.1:{port}/editor`
- Auto-updater checks GitHub Releases on startup and every hour; `update-ready-banner.tsx` shows download progress and "Restart & Install" prompt

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
