# pen-sdk

Umbrella SDK that re-exports all OpenPencil packages from a single entry point.

## Structure

- `src/index.ts` — Single barrel file re-exporting from:
  - `@zseven-w/pen-types` — All document model types and codegen types
  - `@zseven-w/pen-core` — Tree operations, layout engine, variables, normalization, boolean ops
  - `@zseven-w/pen-engine` — `DesignEngine` and all managers
  - `@zseven-w/pen-react` — All hooks, components, and stores (`export *`)
  - `@zseven-w/pen-renderer` — `PenRenderer`, CanvasKit loader, low-level rendering utilities
  - `@zseven-w/pen-figma` — Figma file parser and converter

## Usage

```ts
import {
  type PenDocument,
  createEmptyDocument,
  DesignEngine,
  DesignProvider,
  useDocument,
  PenRenderer,
  parseFigFile,
} from '@zseven-w/pen-sdk';
```

Consumers can import from `@zseven-w/pen-sdk` instead of individual packages. All types, runtime exports, and React hooks are available.
