# pen-mcp

MCP (Model Context Protocol) server for external LLM integration with OpenPencil.

## Structure

### Server

- `src/server.ts` — Standalone MCP server entry point (stdio + HTTP transports). Registers all tool definitions and dispatches to route modules
- `src/document-manager.ts` — File I/O and live canvas sync: `openDocument`, `saveDocument`, `resolveDocPath`, `getCachedDocument`, `setSyncUrl`, `getSyncUrl`, `getLiveSyncState`, `fetchLiveSelection`. Supports both `.op` files and `live://canvas` (Electron/web dev server sync)
- `src/hooks.ts` — `McpHooks` interface + `configureMcpHooks`/`getMcpHooks`: injectable hooks for role resolution, icon lookup, node sanitization. Web app injects implementations at startup
- `src/constants.ts` — `MCP_DEFAULT_PORT` (3100), `PORT_FILE_DIR_NAME`, `PORT_FILE_NAME`, `ICONIFY_API_URL`

### Tools (`src/tools/`)

Document tools:

- `open-document.ts` — `handleOpenDocument`: opens/creates `.op` file or connects to live canvas
- `batch-get.ts` — `handleBatchGet`: reads document tree with depth control
- `get-selection.ts` — `handleGetSelection`: returns current selection from live canvas
- `snapshot-layout.ts` — `handleSnapshotLayout`: captures layout snapshot for comparison
- `find-empty-space.ts` — `handleFindEmptySpace`: finds unoccupied canvas regions
- `pages.ts` — Page CRUD: `handleAddPage`, `handleRemovePage`, `handleRenamePage`, `handleReorderPage`, `handleDuplicatePage`

Node tools:

- `node-crud.ts` — `handleInsertNode`, `handleUpdateNode`, `handleDeleteNode`, `handleMoveNode`, `handleCopyNode`, `handleReplaceNode`, `postProcessNode`
- `read-nodes.ts` — `handleReadNodes`: read specific nodes by ID with depth control
- `import-svg.ts` — `handleImportSvg`: parse and insert SVG content

Design tools:

- `batch-design.ts` — `handleBatchDesign`: single-shot batch design DSL
- `design-skeleton.ts` — `handleDesignSkeleton`: phase 1 of layered design (structure)
- `design-content.ts` — `handleDesignContent`: phase 2 of layered design (content filling)
- `design-refine.ts` — `handleDesignRefine`: phase 3 of layered design (polish)
- `design-prompt.ts` — `buildDesignPrompt`, `listPromptSections`: segmented design knowledge prompt
- `design-md.ts` — `handleGetDesignMd`, `handleSetDesignMd`, `handleExportDesignMd`
- `layered-design-defs.ts` — Tool definitions for the layered design workflow

Variable/theme tools:

- `variables.ts` — `handleGetVariables`, `handleSetVariables`, `handleSetThemes`
- `theme-presets.ts` — `handleSaveThemePreset`, `handleLoadThemePreset`, `handleListThemePresets`

Codegen tools:

- `codegen-plan.ts` — `handleCodegenPlan`: AI-driven component chunking
- `codegen-submit.ts` — `handleCodegenSubmit`: per-chunk code generation
- `codegen-assemble.ts` — `handleCodegenAssemble`: final code assembly
- `codegen-clean.ts` — `handleCodegenClean`: cleanup stale codegen state

Debug tools:

- `debug-logs-tail.ts` — Tail debug logs
- `debug-screenshot.ts` — Capture canvas screenshot
- `debug-validation-report.ts` — Document validation report

### Routes (`src/routes/`)

Route modules group tool definitions and dispatch handlers by domain: `document-routes.ts`, `node-routes.ts`, `design-routes.ts`, `variable-routes.ts`, `codegen-routes.ts`, `style-guide-routes.ts`, `style-operations-routes.ts`, `debug-routes.ts`

### Utils (`src/utils/`)

- `sanitize.ts` — `sanitizeObject`: deep-cleans objects for safe serialization
- `id.ts` — `generateId`: nanoid wrapper
- `node-operations.ts` — `readNodeWithDepth`: depth-limited node tree reading
- `log-utils.ts` — `SENSITIVE_LOG_PATTERN`, `readDebugTail`, `readLogTail`: log file reading with sensitive content redaction
- `design-md-parser.ts` — Design.md parsing (re-exported from pen-core)
- `design-md-style-policy.ts` — `buildDesignMdStylePolicy`: generates style policy from design spec
- `validate-contract.ts` — `validateContract`: validates codegen chunk contracts
- `svg-node-parser.ts` — SVG to PenNode parser (Node.js environment)

## Testing

```bash
bun --bun vitest run packages/pen-mcp/src/__tests__/
```
