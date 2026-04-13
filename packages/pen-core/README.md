# @zseven-w/pen-core

Core document operations for [OpenPencil](https://github.com/ZSeven-W/openpencil) — tree manipulation, layout engine, design variables, boolean path operations, 3-way merge, and more.

## Install

```bash
npm install @zseven-w/pen-core
# or
bun add @zseven-w/pen-core
```

## Overview

`pen-core` is the foundation layer of the OpenPencil stack. It provides pure functions for every document operation — tree CRUD, auto-layout computation, variable resolution, SVG path booleans, and document normalization. All operations are immutable (structural sharing) and framework-free.

## Features

### Document Tree Operations

Create, query, and mutate the document tree:

```typescript
import {
  createEmptyDocument,
  findNodeInTree,
  findParentInTree,
  insertNodeInTree,
  removeNodeFromTree,
  updateNodeInTree,
  flattenNodes,
  isDescendantOf,
  getNodeBounds,
} from '@zseven-w/pen-core';

const doc = createEmptyDocument();
const node = findNodeInTree(doc.children, 'node-id');
const parent = findParentInTree(doc.children, 'node-id');
const flat = flattenNodes(doc.children); // all nodes in flat array
```

### Node Cloning

Deep clone nodes with optional ID regeneration:

```typescript
import { deepCloneNode, cloneNodeWithNewIds, cloneNodesWithNewIds } from '@zseven-w/pen-core';

const clone = deepCloneNode(node); // preserve IDs
const fresh = cloneNodeWithNewIds(node); // new IDs for all descendants
const batch = cloneNodesWithNewIds([a, b, c]); // batch clone
```

### Multi-Page Support

```typescript
import {
  getActivePage,
  getActivePageChildren,
  setActivePageChildren,
  migrateToPages,
  ensureDocumentNodeIds,
} from '@zseven-w/pen-core';

// Migrate a single-page document to multi-page format
const multiPageDoc = migrateToPages(doc);
```

### Layout Engine

Flexbox-like auto-layout computation supporting `fill_container`, `fit_content`, gap, padding, and alignment:

```typescript
import {
  computeLayoutPositions,
  inferLayout,
  fitContentWidth,
  fitContentHeight,
  resolvePadding,
  getNodeWidth,
  getNodeHeight,
  isNodeVisible,
} from '@zseven-w/pen-core';

// Compute absolute positions for all children in a layout container
const positions = computeLayoutPositions(frame, frame.children);

// Infer layout direction from child arrangement
const layout = inferLayout(children); // 'horizontal' | 'vertical' | 'none'
```

### Text Measurement

Estimate text dimensions for layout without a browser DOM:

```typescript
import {
  estimateTextWidth,
  estimateTextWidthPrecise,
  estimateTextHeight,
  resolveTextContent,
  hasCjkText,
  defaultLineHeight,
} from '@zseven-w/pen-core';

const width = estimateTextWidth('Hello World', 16, 400); // font size, weight
const height = estimateTextHeight(textNode, containerWidth);
const isCjk = hasCjkText('こんにちは'); // true
```

### Design Variables

Resolve `$variable` references against document variables and theme axes:

```typescript
import {
  isVariableRef,
  resolveVariableRef,
  resolveNodeForCanvas,
  replaceVariableRefsInTree,
  getDefaultTheme,
} from '@zseven-w/pen-core';

isVariableRef('$primary'); // true

// Resolve all $refs in a node tree for rendering
const resolved = resolveNodeForCanvas(node, doc.variables, doc.themes);

// Rename $old-name → $new-name across entire tree
replaceVariableRefsInTree(children, 'old-name', 'new-name');
```

### Boolean Path Operations

Union, subtract, intersect, and exclude paths via Paper.js:

```typescript
import { executeBooleanOp, canBooleanOp, BooleanOpType } from '@zseven-w/pen-core';

if (canBooleanOp(selectedNodes)) {
  const result = executeBooleanOp(selectedNodes, BooleanOpType.Union);
}
```

### Document Normalization

Sanitize and fix documents from external sources (Figma imports, AI generation):

```typescript
import { normalizePenDocument } from '@zseven-w/pen-core';

const cleaned = normalizePenDocument(rawDoc);
// Fixes: fill type "color" → "solid", gradient stop position → offset,
// sizing strings, padding arrays. Preserves $variable refs.
```

### Layout Normalization

Repair AI-generated layout issues:

```typescript
import {
  normalizeTreeLayout,
  unwrapFakePhoneMockups,
  stripRedundantSectionFills,
  normalizeStrokeFillSchema,
} from '@zseven-w/pen-core';

// Infer missing layout modes, strip child x/y in layout containers
normalizeTreeLayout(rootNode);
```

### 3-Way Document Merge

Diff and merge document trees for collaborative editing and git integration:

```typescript
import { diffDocuments, mergeDocuments } from '@zseven-w/pen-core';

// One-direction diff: base → current
const patches = diffDocuments(base, current);
// patches: NodePatch[] — add, remove, modify, move

// 3-way merge: base + ours + theirs
const result = mergeDocuments(base, ours, theirs);
// result: { document, conflicts }
```

### Design.md Parser

Parse and generate design specification documents:

```typescript
import {
  parseDesignMd,
  generateDesignMd,
  designMdColorsToVariables,
  extractDesignMdFromDocument,
} from '@zseven-w/pen-core';
```

### Path Anchors

Convert between anchor point representation and SVG path data:

```typescript
import {
  anchorsToPathData,
  pathDataToAnchors,
  getPathBoundsFromAnchors,
  inferPathAnchorPointType,
} from '@zseven-w/pen-core';
```

## API Reference

| Category      | Key Functions                                                                                  |
| ------------- | ---------------------------------------------------------------------------------------------- |
| **Tree CRUD** | `findNodeInTree`, `insertNodeInTree`, `removeNodeFromTree`, `updateNodeInTree`, `flattenNodes` |
| **Cloning**   | `deepCloneNode`, `cloneNodeWithNewIds`, `cloneNodesWithNewIds`                                 |
| **Pages**     | `getActivePage`, `migrateToPages`, `ensureDocumentNodeIds`                                     |
| **Layout**    | `computeLayoutPositions`, `inferLayout`, `fitContentWidth`, `fitContentHeight`                 |
| **Text**      | `estimateTextWidth`, `estimateTextHeight`, `hasCjkText`                                        |
| **Variables** | `resolveVariableRef`, `resolveNodeForCanvas`, `replaceVariableRefsInTree`                      |
| **Boolean**   | `executeBooleanOp`, `canBooleanOp`                                                             |
| **Normalize** | `normalizePenDocument`, `normalizeTreeLayout`                                                  |
| **Merge**     | `diffDocuments`, `mergeDocuments`                                                              |
| **IDs**       | `generateId`                                                                                   |

## License

[MIT](./LICENSE)
