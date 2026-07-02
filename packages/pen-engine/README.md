# @zseven-w/pen-engine

Headless design engine for [OpenPencil](https://github.com/ZSeven-W/openpencil) — framework-free document management, selection, history, viewport, and spatial queries. Build your own design tool UI on top of this engine.

## Install

```bash
npm install @zseven-w/pen-engine
# or
bun add @zseven-w/pen-engine
```

## Overview

`pen-engine` is the core runtime that powers OpenPencil's editor. It manages the entire document lifecycle without any framework dependency — React, Vue, Svelte, or vanilla JS all work. The optional `browser.ts` entry adds GPU-accelerated canvas rendering via CanvasKit/Skia.

```
DesignEngine
  |- DocumentManager    Node CRUD, tree operations
  |- SelectionManager   Multi-select, hover tracking
  |- HistoryManager     Undo/redo with debounce + batch
  |- PageManager        Multi-page support
  |- VariableManager    Design variables ($refs)
  |- ViewportController Zoom, pan, coordinate transforms
  |- SpatialIndex       R-tree for hit testing & spatial queries
  |- EventEmitter       Typed event system
```

## Quick Start

```typescript
import { DesignEngine } from '@zseven-w/pen-engine';

const engine = new DesignEngine();

// Load or create a document
engine.loadDocument(myDocument);

// Add a node
engine.addNode(null, {
  id: 'frame-1',
  type: 'frame',
  name: 'Header',
  width: 1200,
  height: 80,
  layout: 'horizontal',
});

// Select, undo, inspect
engine.select(['frame-1']);
engine.undo();
console.log(engine.getDocument());
```

## Features

### Document Operations

Create, query, and mutate the node tree:

```typescript
engine.addNode(parentId, node, index?);
engine.updateNode(id, { fill: [{ type: 'solid', color: '#FF0000' }] });
engine.removeNode(id);
engine.moveNode(id, newParentId, index);
engine.duplicateNode(id);
engine.groupNodes(['node-1', 'node-2']);
engine.ungroupNode(groupId);
engine.getNodeById(id);
```

### Selection & Hover

```typescript
engine.select(['node-1', 'node-2']);
engine.clearSelection();
engine.getSelection(); // string[]
engine.setHoveredId('node-3');
engine.getHoveredId(); // string | null
```

### History (Undo / Redo)

Structural history with debouncing and batch support:

```typescript
engine.undo();
engine.redo();
engine.canUndo; // boolean
engine.canRedo; // boolean

// Batch multiple mutations into a single history entry
engine.batch(() => {
  engine.updateNode('a', { x: 100 });
  engine.updateNode('b', { x: 200 });
});
```

### Viewport

Pan, zoom, and coordinate conversion:

```typescript
engine.setViewport(zoom, panX, panY);
engine.zoomToRect(x, y, w, h, containerW, containerH);
engine.getContentBounds(); // { x, y, w, h } | null
engine.screenToScene(screenX, screenY); // { x, y }
engine.sceneToScreen(sceneX, sceneY); // { x, y }
```

### Hit Testing (Spatial Index)

R-tree backed queries for click and marquee selection:

```typescript
engine.hitTest(x, y); // PenNode | null
engine.searchRect(x, y, w, h); // PenNode[]
```

### Multi-Page

```typescript
engine.addPage(); // returns pageId
engine.removePage(pageId);
engine.setActivePage(pageId);
engine.getActivePage();
```

### Design Variables

```typescript
engine.setVariable('primary', { type: 'color', value: '#2563EB' });
engine.removeVariable('primary');
engine.renameVariable('primary', 'brand');
engine.resolveVariable('$primary'); // '#2563EB'
```

### SVG Import

Isomorphic SVG parser (DOM in browser, regex fallback in Node.js):

```typescript
import { parseSvgToNodes } from '@zseven-w/pen-engine';

const nodes = parseSvgToNodes(svgString, 400);
engine.addNode(null, nodes[0]);
```

### Events

Typed event system for reactive UI binding:

```typescript
const unsub = engine.on('document:change', (doc) => {
  /* re-render */
});
engine.on('selection:change', (ids) => {
  /* update UI */
});
engine.on('viewport:change', (viewport) => {
  /* update zoom indicator */
});
unsub(); // unsubscribe
```

### Browser Canvas (Optional)

GPU-accelerated rendering via CanvasKit/Skia — import from `@zseven-w/pen-engine/browser`:

```typescript
import { attachCanvas, attachInteraction } from '@zseven-w/pen-engine/browser';

const binding = await attachCanvas(engine, canvasElement);
const detach = attachInteraction(engine, canvasElement);

// Later
binding.dispose();
detach();
```

## API Reference

| Method                          | Description            |
| ------------------------------- | ---------------------- |
| `loadDocument(doc)`             | Load a PenDocument     |
| `getDocument()`                 | Get current document   |
| `createDocument()`              | Create empty document  |
| `addNode(parent, node, index?)` | Insert node            |
| `updateNode(id, updates)`       | Partial update         |
| `removeNode(id)`                | Delete node + children |
| `moveNode(id, parent, index)`   | Reparent node          |
| `duplicateNode(id)`             | Deep clone             |
| `groupNodes(ids)`               | Group into frame       |
| `ungroupNode(id)`               | Dissolve group         |
| `select(ids)`                   | Set selection          |
| `undo()` / `redo()`             | History navigation     |
| `batch(fn)`                     | Batch mutations        |
| `setViewport(z, x, y)`          | Set viewport           |
| `hitTest(x, y)`                 | Point query            |
| `searchRect(x, y, w, h)`        | Area query             |
| `importSVG(svg, parent?)`       | Parse and insert SVG   |
| `dispose()`                     | Clean up resources     |

## License

[MIT](./LICENSE)
