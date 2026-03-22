# Packages

Shared libraries consumed by `apps/web` and `apps/desktop`. All re-exported via `@zseven-w/pen-sdk`.

## pen-types (`pen-types/src/`)

Type definitions (9 files):
- `pen.ts` — PenDocument/PenNode (frame, group, rectangle, ellipse, line, polygon, path, text, image, ref), ContainerProps, `PenPage`; `PenDocument.variables`, `PenDocument.themes`, `PenDocument.pages`
- `canvas.ts` — ToolType, ViewportState, SelectionState, CanvasInteraction
- `styles.ts` — PenFill (solid, linear_gradient, radial_gradient), PenStroke, PenEffect, BlendMode, StyledTextSegment
- `variables.ts` — `VariableDefinition`, `ThemedValue`, `VariableValue`
- `uikit.ts` — UIKit, KitComponent, ComponentCategory types
- `agent-settings.ts` — AI provider config types
- `electron.d.ts` — Electron IPC bridge types
- `theme-preset.ts` — Theme preset types
- `opencode-sdk.d.ts` — Type declarations for @opencode-ai/sdk

## pen-core (`pen-core/src/`)

Core document operations (11 files + `layout/` + `variables/` subdirs):
- `tree-utils.ts` — Pure tree helpers: `findNodeInTree`, `findParentInTree`, `removeNodeFromTree`, `updateNodeInTree`, `flattenNodes`, `insertNodeInTree`, `isDescendantOf`, `getNodeBounds`, `findClearX`, `scaleChildrenInPlace`, `rotateChildrenInPlace`, `createEmptyDocument`, `DEFAULT_FRAME_ID`; Clone utilities: `deepCloneNode`, `cloneNodeWithNewIds`, `cloneNodesWithNewIds` (canonical source for all node cloning)
- `normalize.ts` — Pen file normalization (format fixes only, preserves `$variable` refs)
- `boolean-ops.ts` — Union/subtract/intersect via Paper.js
- `sync-lock.ts` — Prevents circular sync loops
- `arc-path.ts` — SVG arc utilities
- `font-utils.ts` — Font utilities
- `node-helpers.ts` — Node helper functions
- `constants.ts` — Core constants
- `id.ts` — ID generation (`nanoid`)
- `layout/engine.ts` — Auto-layout computation: `resolvePadding`, `getNodeWidth/Height`, `computeLayoutPositions`
- `layout/text-measure.ts` — Text width/height estimation, CJK detection, `parseSizing`
- `variables/resolve.ts` — Core resolution: `resolveVariableRef`, `resolveNodeForCanvas`, `getDefaultTheme`, `isVariableRef`
- `variables/replace-refs.ts` — `replaceVariableRefsInTree`: recursively walk node tree to replace/resolve `$refs`

## pen-codegen (`pen-codegen/src/`)

Multi-platform code generators (9 files, output `var(--name)` for `$variable` refs):
- `react-generator.ts` — React + Tailwind CSS
- `html-generator.ts` — HTML + CSS
- `css-variables-generator.ts` — CSS Variables from design tokens
- `vue-generator.ts` — Vue 3 + CSS
- `svelte-generator.ts` — Svelte + CSS
- `flutter-generator.ts` — Flutter/Dart
- `swiftui-generator.ts` — SwiftUI
- `compose-generator.ts` — Android Jetpack Compose
- `react-native-generator.ts` — React Native

## pen-figma (`pen-figma/src/`)

Figma `.fig` file import pipeline (17 files):
- `fig-parser.ts` — Binary `.fig` file parser
- `figma-node-mapper.ts` — Maps Figma nodes to PenNodes (uses injectable icon lookup via `setIconLookup()`)
- `figma-node-converters.ts` — Figma node conversion utilities
- `figma-fill-mapper.ts`, `figma-stroke-mapper.ts`, `figma-effect-mapper.ts` — Style converters
- `figma-layout-mapper.ts` — Maps Figma auto-layout to PenNode layout props
- `figma-text-mapper.ts` — Converts Figma text styles
- `figma-vector-decoder.ts` — Decodes Figma vector geometry
- `figma-color-utils.ts` — Color space conversion utilities
- `figma-image-resolver.ts` — Resolves image blob references
- `figma-clipboard.ts` — Figma clipboard paste handling
- `figma-tree-builder.ts` — Figma document tree building
- `figma-types.ts` — Figma internal type definitions

## pen-renderer (`pen-renderer/src/`)

Standalone CanvasKit/Skia renderer (13 files):
- `renderer.ts` — Core renderer class
- `document-flattener.ts` — Document tree flattening with layout resolution
- `node-renderer.ts` — Node-level draw calls
- `text-renderer.ts` — Text rendering
- `paint-utils.ts` — Color parsing, gradient creation
- `path-utils.ts` — SVG path conversion
- `image-loader.ts` — Async image loading and caching
- `font-manager.ts` — Font management
- `spatial-index.ts` — R-tree backed spatial queries
- `viewport.ts` — Viewport math
- `init.ts` — CanvasKit WASM loader
- `types.ts` — Renderer-specific types

## pen-sdk (`pen-sdk/src/`)

Umbrella SDK (1 file): `index.ts` re-exports all packages.
