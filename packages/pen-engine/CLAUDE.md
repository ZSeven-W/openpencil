# pen-engine

Headless design engine -- framework-free document, selection, history, viewport, and spatial indexing.

## Structure

### Core (`src/core/`)

- `design-engine.ts` — `DesignEngine` class: composes all managers, exposes high-level API (`loadDocument`, `addNode`, `updateNode`, `deleteNode`, `moveNode`, `select`, `undo`, `redo`, `setTool`, `generateCode`, `batch`, `on`/`off` events). Zero DOM/React/Zustand dependencies
- `document-manager.ts` — `DocumentManager`: immutable PenDocument tree mutations with history integration. Methods: `getDocument`, `loadDocument`, `addNode`, `updateNode`, `deleteNode`, `moveNode`, `findNode`, `findParent`, `duplicateNode`
- `history-manager.ts` — `HistoryManager`: framework-agnostic undo/redo stack with batch support and debouncing (300ms). Methods: `push`, `undo`, `redo`, `clear`, `beginBatch`, `endBatch`
- `selection-manager.ts` — `SelectionManager`: immutable selection state. Methods: `select`, `clearSelection`, `getSelection`, `getActiveId`, `setHoveredId`
- `page-manager.ts` — `PageManager`: multi-page lifecycle. Methods: `getActivePage`, `setActivePage`, `addPage`, `removePage`, `renamePage`, `duplicatePage`, `reorderPage`
- `variable-manager.ts` — `VariableManager`: design variable CRUD with theme support. Methods: `setVariable`, `removeVariable`, `renameVariable`, `setThemes`
- `viewport-controller.ts` — `ViewportController`: zoom/pan math, coordinate transforms. Methods: `setViewport`, `screenToScene`, `sceneToScreen`, `zoomTo`, `zoomToFit`
- `event-emitter.ts` — `TypedEventEmitter<Events>`: generic typed pub/sub with `on`/`off`/`emit`/`dispose`
- `node-creator.ts` — `createNodeForTool(tool, x, y, w, h)`: factory for default PenNodes per tool type; `isDrawingTool(tool)` check
- `svg-parser.ts` — `parseSvgToNodes(svgString)`: isomorphic SVG to PenNode[] converter (DOMParser in browser, regex fallback in Node.js)
- `spatial-index.ts` — `EngineSpatialIndex`: wraps pen-renderer's `SpatialIndex` for engine-level hit testing. Methods: `rebuild`, `hitTest`, `searchRect`, `hitTestNode`
- `constants.ts` — Engine constants: `DEFAULT_MAX_HISTORY`, `HISTORY_DEBOUNCE_MS`, `MIN_DRAW_SIZE`, `DRAG_THRESHOLD`, `HANDLE_HIT_RADIUS`, `HANDLE_CURSORS`

### Browser adapter (`src/browser/`)

- `canvas-bindings.ts` — `attachCanvas(engine, canvasEl, options)`: loads CanvasKit WASM, creates `CanvasBinding` for GPU rendering with auto-rerender on engine events
- `canvas-renderer.ts` — `CanvasRenderer`: internal class managing CanvasKit surface, render nodes, and redraw loop
- `text-edit-overlay.ts` — `TextEditOverlayOptions`: DOM textarea overlay for inline text editing
- `interaction/` — Mouse/keyboard event handlers:
  - `interaction-controller.ts` — `attachInteraction(engine, canvasEl)`: binds all pointer/keyboard events
  - `select-handler.ts` — Click/drag selection, multi-select, entered-frame navigation
  - `draw-handler.ts` — Shape drawing (rectangle, ellipse, frame, line, polygon)
  - `resize-handler.ts` — Selection handle resize with aspect ratio support
  - `pen-tool-handler.ts` — Bezier pen tool for path creation
  - `arc-handler.ts` — Arc/sweep angle editing for ellipses

## Key exports (from `src/index.ts`)

`DesignEngine`, `TypedEventEmitter`, `HistoryManager`, `DocumentManager`, `SelectionManager`, `PageManager`, `VariableManager`, `ViewportController`, `EngineSpatialIndex`, `createNodeForTool`, `isDrawingTool`, `parseSvgToNodes`

Browser adapter (from `src/browser.ts`): `attachCanvas`, `attachInteraction`, `CanvasBinding`, `AttachCanvasOptions`

## Testing

```bash
bun --bun vitest run packages/pen-engine/src/__tests__/
```
