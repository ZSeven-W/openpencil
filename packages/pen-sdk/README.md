# @zseven-w/pen-sdk

The umbrella SDK for [OpenPencil](https://github.com/ZSeven-W/openpencil). One import gives you everything — types, document operations, headless engine, React components, code generation, Figma import, and GPU rendering.

## Install

```bash
npm install @zseven-w/pen-sdk
# or
bun add @zseven-w/pen-sdk
```

## Overview

`pen-sdk` re-exports all OpenPencil packages through a single entry point. Use it when you want the full stack without managing individual dependencies. For smaller bundles, install only the packages you need.

## What's Included

| Package                                     | Provides                                                                            |
| ------------------------------------------- | ----------------------------------------------------------------------------------- |
| [`@zseven-w/pen-types`](../pen-types)       | TypeScript types for the document model (`PenDocument`, `PenNode`, `PenFill`, etc.) |
| [`@zseven-w/pen-core`](../pen-core)         | Tree operations, layout engine, variables, boolean ops, normalization, 3-way merge  |
| [`@zseven-w/pen-engine`](../pen-engine)     | Headless design engine — document, selection, history, viewport, spatial index      |
| [`@zseven-w/pen-react`](../pen-react)       | React UI SDK — `DesignProvider`, `DesignCanvas`, 10 hooks, 39 components            |
| [`@zseven-w/pen-renderer`](../pen-renderer) | CanvasKit/Skia GPU renderer with viewport, hit testing, font/image management       |
| [`@zseven-w/pen-figma`](../pen-figma)       | Figma `.fig` binary parser and converter                                            |

## Usage

### Build a full editor

```tsx
import {
  DesignProvider,
  DesignCanvas,
  CoreToolbar,
  LayerPanel,
  PropertyPanel,
  useDocument,
  useSelection,
  useHistory,
} from '@zseven-w/pen-sdk';

function Editor() {
  return (
    <DesignProvider initialDocument={myDoc}>
      <CoreToolbar />
      <DesignCanvas />
      <LayerPanel />
      <PropertyPanel />
    </DesignProvider>
  );
}
```

### Document operations

```typescript
import {
  type PenDocument,
  type PenNode,
  createEmptyDocument,
  findNodeInTree,
  insertNodeInTree,
  flattenNodes,
  normalizePenDocument,
  resolveNodeForCanvas,
} from '@zseven-w/pen-sdk';

const doc = createEmptyDocument();
const node = findNodeInTree(doc.children, 'header');
```

### Headless engine (no React)

```typescript
import { DesignEngine } from '@zseven-w/pen-sdk';

const engine = new DesignEngine();
engine.loadDocument(doc);
engine.addNode(null, { type: 'frame', name: 'Page', width: 1200, height: 800 });
engine.select(['node-1']);
engine.undo();
```

### Code generation

```typescript
import {
  generateReactFromDocument,
  generateHTMLFromDocument,
  generateFlutterFromDocument,
  generateVueFromDocument,
  generateSvelteFromDocument,
  generateSwiftUIFromDocument,
} from '@zseven-w/pen-sdk';

const reactCode = generateReactFromDocument(doc);
const htmlCode = generateHTMLFromDocument(doc);
```

### Figma import

```typescript
import { parseFigFile, figmaAllPagesToPenDocument, isFigmaClipboardHtml } from '@zseven-w/pen-sdk';

const figFile = parseFigFile(buffer);
const document = figmaAllPagesToPenDocument(figFile);
```

### GPU rendering (headless)

```typescript
import { loadCanvasKit, PenRenderer } from '@zseven-w/pen-sdk';

await loadCanvasKit();
const renderer = new PenRenderer(canvas, document);
renderer.render();
```

## Individual Packages

For smaller bundles, install only what you need:

```bash
# Types only (zero runtime)
npm install @zseven-w/pen-types

# Document operations (no rendering)
npm install @zseven-w/pen-core

# Headless engine (no React)
npm install @zseven-w/pen-engine

# React components
npm install @zseven-w/pen-react

# GPU renderer
npm install @zseven-w/pen-renderer canvaskit-wasm

# Figma import
npm install @zseven-w/pen-figma
```

## License

[MIT](./LICENSE)
