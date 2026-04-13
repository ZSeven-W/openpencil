# @zseven-w/pen-react

React UI SDK for [OpenPencil](https://github.com/ZSeven-W/openpencil) — a complete set of hooks, components, and panels to build a design editor with React.

## Install

```bash
npm install @zseven-w/pen-react
# or
bun add @zseven-w/pen-react
```

**Peer dependencies:** `react@^19`, `react-dom@^19`, `@radix-ui/react-*` (popover, select, separator, slider, switch, toggle, tooltip)

## Overview

`pen-react` wraps [`@zseven-w/pen-engine`](../pen-engine) into idiomatic React: a context provider, 10 semantic hooks, and 39 ready-to-use components covering the full editor UI.

```
<DesignProvider>
  <CoreToolbar />
  <DesignCanvas />
  <LayerPanel />
  <PropertyPanel />
  <PageTabs />
  <StatusBar />
</DesignProvider>
```

## Quick Start

```tsx
import {
  DesignProvider,
  DesignCanvas,
  CoreToolbar,
  LayerPanel,
  PropertyPanel,
} from '@zseven-w/pen-react';

function Editor() {
  return (
    <DesignProvider initialDocument={myDoc}>
      <div className="flex h-screen">
        <LayerPanel />
        <div className="flex flex-col flex-1">
          <CoreToolbar />
          <DesignCanvas className="flex-1" />
        </div>
        <PropertyPanel />
      </div>
    </DesignProvider>
  );
}
```

## Hooks

All hooks subscribe to the engine and re-render on change:

```tsx
import {
  useDesignEngine,
  useDocument,
  useSelection,
  useViewport,
  useActiveTool,
  useHistory,
  useActiveNode,
  useActivePage,
  useHover,
  useVariables,
} from '@zseven-w/pen-react';

function Inspector() {
  const node = useActiveNode(); // PenNode | null
  const selection = useSelection(); // string[]
  const { canUndo, undo } = useHistory();
  const viewport = useViewport(); // { zoom, panX, panY }
  const tool = useActiveTool(); // ToolType
  const doc = useDocument(); // PenDocument
  const page = useActivePage(); // PenPage
  const hoverId = useHover(); // string | null
  const variables = useVariables(); // VariableDefinition[]
  const engine = useDesignEngine(); // DesignEngine (escape hatch)

  return <div>Selected: {selection.length} nodes</div>;
}
```

## Provider

### Uncontrolled mode

Engine owns the document. Good for standalone editors:

```tsx
<DesignProvider initialDocument={doc}>{children}</DesignProvider>
```

### Controlled mode

Parent owns the document. Good for integration into existing state:

```tsx
<DesignProvider document={doc} onDocumentChange={(newDoc) => setDoc(newDoc)}>
  {children}
</DesignProvider>
```

Echo-loop prevention is built in — `onDocumentChange` won't fire for changes that originated from the parent.

## Components

### Canvas

| Component      | Description                                                                               |
| -------------- | ----------------------------------------------------------------------------------------- |
| `DesignCanvas` | GPU-rendered canvas with CanvasKit/Skia. Handles zoom, pan, resize, and all interactions. |

```tsx
<DesignCanvas
  className="w-full h-full"
  onReady={(engine) => console.log('Canvas ready')}
  loadingFallback={<Spinner />}
/>
```

### Toolbar

| Component           | Description                                                       |
| ------------------- | ----------------------------------------------------------------- |
| `CoreToolbar`       | Main tool selection bar (select, frame, shapes, text, pen, image) |
| `ToolButton`        | Individual tool button with icon + active state                   |
| `ShapeToolDropdown` | Dropdown for shape tools (rectangle, ellipse, polygon, line)      |
| `BooleanToolbar`    | Union, subtract, intersect, exclude operations                    |

### Panels

| Component          | Description                                           |
| ------------------ | ----------------------------------------------------- |
| `LayerPanel`       | Hierarchical tree view with drag-and-drop reordering  |
| `LayerItem`        | Single layer row — collapse, visibility, lock, rename |
| `LayerContextMenu` | Right-click menu: copy, paste, delete, group, z-order |
| `PropertyPanel`    | Tabbed property inspector for the selected node       |
| `PageTabs`         | Multi-page tab bar with add/rename/reorder/delete     |
| `StatusBar`        | Bottom bar with zoom, coordinates, node count         |

### Property Sections

Drop these into your own property panel or use `PropertyPanel` which includes all of them:

| Section                | Edits                                           |
| ---------------------- | ----------------------------------------------- |
| `SizeSection`          | x, y, width, height, rotation, constraints      |
| `FillSection`          | Solid color, linear/radial gradient             |
| `StrokeSection`        | Color, thickness, dash pattern, cap, join       |
| `TextSection`          | Font family, size, weight, color, alignment     |
| `TextLayoutSection`    | Line height, letter spacing, paragraph spacing  |
| `CornerRadiusSection`  | Uniform or per-corner border radius             |
| `EffectsSection`       | Drop shadow, inner shadow, blur                 |
| `LayoutSection`        | Auto-layout direction, gap, justify, align      |
| `LayoutPaddingSection` | Uniform or per-side padding                     |
| `AppearanceSection`    | Opacity, blend mode                             |
| `IconSection`          | Icon name picker (Lucide icons)                 |
| `ImageSection`         | Image source, fit mode                          |
| `ExportSection`        | Code generation target (React, HTML, Vue, etc.) |

### Shared UI

| Component          | Description                                         |
| ------------------ | --------------------------------------------------- |
| `ColorPicker`      | Color input with swatch palette and hex input       |
| `NumberInput`      | Numeric field with drag-to-adjust and arrow keys    |
| `SectionHeader`    | Collapsible section header with title + actions     |
| `FontPicker`       | Font family selector with preview                   |
| `VariablePicker`   | Design variable reference picker (`$primary`, etc.) |
| `IconPickerDialog` | Modal icon browser with search and categories       |

## UI Store

Ephemeral UI state (panel open/close, drag state) managed by Zustand — separate from engine state:

```tsx
import { useUIStore } from '@zseven-w/pen-react';

const { layerPanelOpen, toggleLayerPanel } = useUIStore();
```

## Styling

Components use [Tailwind CSS](https://tailwindcss.com/) + [CVA](https://cva.style/docs) for variant styling, and [Radix UI](https://www.radix-ui.com/) primitives for accessibility. Override styles via `className` props or Tailwind's design token system.

## License

[MIT](./LICENSE)
