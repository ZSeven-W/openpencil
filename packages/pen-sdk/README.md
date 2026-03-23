# @zseven-w/pen-sdk

The umbrella SDK for [OpenPencil](https://github.com/nicepkg/openpencil). One import gives you everything — types, document operations, code generation, Figma import, and rendering.

## Install

```bash
npm install @zseven-w/pen-sdk
```

## What's Included

This package re-exports all OpenPencil packages:

| Package | Provides |
|---|---|
| `@zseven-w/pen-types` | TypeScript types for the document model |
| `@zseven-w/pen-core` | Tree operations, layout engine, variables, boolean ops |
| `@zseven-w/pen-codegen` | Code generators (React, HTML, Vue, Svelte, Flutter, SwiftUI, Compose, RN) |
| `@zseven-w/pen-figma` | Figma `.fig` parser and converter |
| `@zseven-w/pen-renderer` | CanvasKit/Skia GPU renderer |

## Usage

```ts
import {
  // Types
  type PenDocument,
  type PenNode,

  // Document operations
  createEmptyDocument,
  findNodeInTree,
  insertNodeInTree,
  normalizePenDocument,

  // Code generation
  generateReactFromDocument,
  generateHTMLFromDocument,
  generateFlutterFromDocument,

  // Figma import
  parseFigFile,
  figmaAllPagesToPenDocument,

  // Rendering
  loadCanvasKit,
  PenRenderer,
} from '@zseven-w/pen-sdk'
```

Or install individual packages for smaller bundles — see each package's README for details.

## License

MIT
