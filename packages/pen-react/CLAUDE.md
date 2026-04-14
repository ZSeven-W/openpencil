# pen-react

React UI SDK for OpenPencil -- provider, hooks, panels, toolbar, and property editor components.

## Structure

### Core

- `src/context.ts` — `DesignEngineContext`: React context holding the `DesignEngine` instance
- `src/provider.tsx` — `DesignProvider`: provides engine to React tree. Supports uncontrolled mode (`initialDocument`) and controlled mode (`document` + `onDocumentChange`) with echo-loop prevention

### Hooks (`src/hooks/`)

- `use-design-engine.ts` — `useDesignEngine()`: gets engine from context, throws if outside provider
- `use-document.ts` — `useDocument()`: returns current `PenDocument`, re-renders on mutation
- `use-selection.ts` — `useSelection()`: returns `string[]` of selected node IDs
- `use-viewport.ts` — `useViewport()`: returns `ViewportState` (zoom, panX, panY)
- `use-active-tool.ts` — `useActiveTool()`: returns current `ToolType`
- `use-history.ts` — `useHistory()`: returns `{ canUndo, canRedo }`
- `use-active-node.ts` — `useActiveNode()`: returns the single active `PenNode` or null
- `use-active-page.ts` — `useActivePage()`: returns active page ID
- `use-hover.ts` — `useHover()`: returns hovered node ID
- `use-variables.ts` — `useVariables()`: returns document variables map

### Utilities (`src/utils/`)

- `use-engine-subscribe.ts` — `useEngineSubscribe(engine, event, getSnapshot)`: generic hook using `useSyncExternalStore` to subscribe to engine events with stable snapshot refs

### Stores (`src/stores/`)

- `ui-store.ts` — `useUIStore` (Zustand): pure UI state -- panel visibility, layer drag state, collapsed nodes. NOT engine state

### Components (`src/components/`)

- `design-canvas.tsx` — `DesignCanvas`: canvas element with CanvasKit rendering via `attachCanvas` + `attachInteraction` from pen-engine browser adapter
- `core-toolbar.tsx` — `CoreToolbar`: main tool selection bar
- `tool-button.tsx` — `ToolButton`: individual tool button with active state
- `shape-tool-dropdown.tsx` — `ShapeToolDropdown`: dropdown for shape tools (rectangle, ellipse, line, polygon)
- `layer-panel.tsx` — `LayerPanel`: document tree panel with drag-and-drop reordering
- `layer-item.tsx` — `LayerItem`: individual layer row with visibility/lock toggles
- `layer-context-menu.tsx` — `LayerContextMenu`: right-click context menu for layers
- `property-panel.tsx` — `PropertyPanel`: right-side property inspector (delegates to section components)
- `color-picker.tsx` — `ColorPicker`: color input with hex/opacity, gradient support
- `number-input.tsx` — `NumberInput`: numeric input with drag-to-adjust
- `section-header.tsx` — `SectionHeader`: collapsible section header
- `font-picker.tsx` — `FontPicker`: font family selector with search
- `variable-picker.tsx` — `VariablePicker`: design variable selector
- `icon-picker-dialog.tsx` — `IconPickerDialog`: icon search and selection
- `boolean-toolbar.tsx` — `BooleanToolbar`: union/subtract/intersect boolean operations
- `page-tabs.tsx` — `PageTabs`: multi-page tab bar
- `status-bar.tsx` — `StatusBar`: bottom status bar (zoom, selection info)

### Property sections (`src/components/sections/`)

- `size-section.tsx` — Width, height, x, y, rotation
- `fill-section.tsx` — Fill array editor (solid, gradient, image)
- `stroke-section.tsx` — Stroke properties (thickness, alignment, dash)
- `text-section.tsx` — Text content, font, size, weight, style
- `text-layout-section.tsx` — Text alignment, growth mode
- `corner-radius-section.tsx` — Corner radius (uniform or per-corner)
- `effects-section.tsx` — Blur and shadow effects
- `layout-section.tsx` — Auto-layout direction, gap, alignment
- `layout-padding-section.tsx` — Padding (uniform or per-side)
- `appearance-section.tsx` — Opacity, blend mode, visibility
- `icon-section.tsx` — Icon font name and family
- `image-section.tsx` — Image source, fit mode, adjustments
- `export-section.tsx` — Export format and code preview

## Testing

```bash
bun --bun vitest run packages/pen-react/src/__tests__/
```
