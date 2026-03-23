# @zseven-w/pen-core

Core document operations for [OpenPencil](https://github.com/nicepkg/openpencil) — tree manipulation, layout engine, design variables, boolean path operations, and more.

## Install

```bash
npm install @zseven-w/pen-core
```

## Features

### Document Tree Operations

Create, query, and mutate the document tree:

```ts
import {
  createEmptyDocument,
  findNodeInTree,
  insertNodeInTree,
  removeNodeFromTree,
  updateNodeInTree,
  deepCloneNode,
  flattenNodes,
} from '@zseven-w/pen-core'

const doc = createEmptyDocument()
const node = findNodeInTree(doc.children, 'node-id')
```

### Multi-Page Support

```ts
import { getActivePage, getActivePageChildren, migrateToPages } from '@zseven-w/pen-core'
```

### Layout Engine

Automatic layout computation with auto-sizing, padding, and gap support:

```ts
import { inferLayout, computeLayoutPositions, fitContentWidth, fitContentHeight } from '@zseven-w/pen-core'
```

### Design Variables

Resolve `$variable` references against theme axes:

```ts
import { resolveVariableRef, resolveNodeForCanvas, replaceVariableRefsInTree } from '@zseven-w/pen-core'
```

### Boolean Path Operations

Union, subtract, intersect, and exclude paths via Paper.js:

```ts
import { executeBooleanOp, BooleanOpType } from '@zseven-w/pen-core'
```

### Text Measurement

Estimate text dimensions for layout without a browser:

```ts
import { estimateTextWidth, estimateTextHeight } from '@zseven-w/pen-core'
```

### Document Normalization

Sanitize and fix documents imported from external sources:

```ts
import { normalizePenDocument } from '@zseven-w/pen-core'
```

## License

MIT
