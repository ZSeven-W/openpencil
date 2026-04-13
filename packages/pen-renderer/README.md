# @zseven-w/pen-renderer

Standalone CanvasKit/Skia renderer for [OpenPencil](https://github.com/ZSeven-W/openpencil) design files. Render `.op` documents to a GPU-accelerated canvas — works in browsers, Node.js, and headless environments.

## Install

```bash
npm install @zseven-w/pen-renderer canvaskit-wasm
# or
bun add @zseven-w/pen-renderer canvaskit-wasm
```

`canvaskit-wasm` is a peer dependency — you provide the WASM binary.

## Overview

`pen-renderer` is a pure TypeScript + CanvasKit rendering pipeline with no React or framework dependency. It takes a `PenDocument` and renders it to a WebGL surface with GPU acceleration. The pipeline:

```
PenDocument → flattenToRenderNodes() → absolute positions → SkiaNodeRenderer → GPU canvas
                                           ↓
                                    SpatialIndex (R-tree) → hitTest / searchRect
```

## Quick Start

```typescript
import { loadCanvasKit, PenRenderer } from '@zseven-w/pen-renderer';

// 1. Initialize CanvasKit WASM (once, globally)
await loadCanvasKit();

// 2. Create renderer on a canvas element
const renderer = new PenRenderer(canvas, document, {
  width: 1920,
  height: 1080,
});

// 3. Render
renderer.render();

// 4. Interact
renderer.zoomToFit();
renderer.zoomTo(1.5, centerX, centerY);
renderer.pan(deltaX, deltaY);
const node = renderer.hitTest(mouseX, mouseY);

// 5. Cleanup
renderer.dispose();
```

## Features

### High-Level Renderer

`PenRenderer` provides a complete rendering solution with viewport, selection, and interaction:

```typescript
const renderer = new PenRenderer(canvas, document, options);

renderer.setDocument(newDoc); // Update document
renderer.render(); // Trigger re-render
renderer.zoomToFit(); // Fit content to viewport
renderer.zoomTo(zoom, cx, cy); // Zoom to point
renderer.pan(dx, dy); // Pan viewport
renderer.hitTest(x, y); // Hit test at screen coords
renderer.dispose(); // Free resources
```

### Document Flattening

Pre-process the document tree into flat render nodes with absolute positions:

```typescript
import {
  flattenToRenderNodes,
  resolveRefs,
  premeasureTextHeights,
  remapIds,
} from '@zseven-w/pen-renderer';

// Flatten tree → absolute positions
const renderNodes = flattenToRenderNodes(children, viewport);

// Resolve $ref nodes to their source
const resolved = resolveRefs(renderNodes, document);

// Pre-measure text heights using Canvas 2D (for accurate layout)
premeasureTextHeights(renderNodes, canvasContext);
```

### Viewport Math

Camera transforms for pan, zoom, and coordinate conversion:

```typescript
import {
  viewportMatrix,
  screenToScene,
  sceneToScreen,
  zoomToPoint,
  getViewportBounds,
  isRectInViewport,
} from '@zseven-w/pen-renderer';

const matrix = viewportMatrix(zoom, panX, panY); // 3x3 CanvasKit matrix
const scene = screenToScene(mouseX, mouseY, viewport);
const screen = sceneToScreen(nodeX, nodeY, viewport);
const newVp = zoomToPoint(viewport, 2.0, centerX, centerY);
```

### Spatial Index

R-tree backed spatial queries for click hit testing and marquee selection:

```typescript
import { SpatialIndex } from '@zseven-w/pen-renderer';

const index = new SpatialIndex();
index.rebuild(renderNodes);

const clicked = index.hitTest(x, y); // topmost node at point
const selected = index.searchRect(x, y, w, h); // all nodes in rect
const node = index.get(nodeId); // lookup by ID
```

### Low-Level Renderers

For custom rendering pipelines:

```typescript
import {
  SkiaNodeRenderer,
  SkiaTextRenderer,
  SkiaFontManager,
  SkiaImageLoader,
} from '@zseven-w/pen-renderer';
```

| Class              | Handles                                                                                                                                                                                 |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SkiaNodeRenderer` | All node types — rectangles, ellipses, paths, images, icons, lines, polygons. Fills (solid, gradient, image), strokes, effects (shadow, blur), corner radius, clip, opacity, blend mode |
| `SkiaTextRenderer` | Text layout and rendering via Paragraph API with bitmap fallback. FIFO caches (256 MB text, 64 MB paragraph)                                                                            |
| `SkiaFontManager`  | Font loading — bundled fonts (Inter, Poppins, Roboto, etc.) + Google Fonts CSS fetching                                                                                                 |
| `SkiaImageLoader`  | Async image loading with caching and custom source resolvers                                                                                                                            |

### Thumbnail Generation

Render individual nodes to offscreen thumbnails (used for git conflict UI, exports):

```typescript
import { renderNodeThumbnail } from '@zseven-w/pen-renderer';

const dataUrl = renderNodeThumbnail(node, { width: 200, height: 200 });
```

### Paint Utilities

```typescript
import {
  parseColor,
  resolveFillColor,
  resolveStrokeColor,
  wrapLine,
  cssFontFamily,
  sanitizeSvgPath,
} from '@zseven-w/pen-renderer';

const color = parseColor('#2563EB'); // CanvasKit Color4f
```

## API Reference

| Category      | Exports                                                                                |
| ------------- | -------------------------------------------------------------------------------------- |
| **Init**      | `loadCanvasKit(options?)`, `getCanvasKit()`                                            |
| **Renderer**  | `PenRenderer`                                                                          |
| **Flatten**   | `flattenToRenderNodes`, `resolveRefs`, `premeasureTextHeights`, `remapIds`             |
| **Viewport**  | `viewportMatrix`, `screenToScene`, `sceneToScreen`, `zoomToPoint`, `getViewportBounds` |
| **Spatial**   | `SpatialIndex` — `rebuild`, `hitTest`, `searchRect`, `get`                             |
| **Node**      | `SkiaNodeRenderer`                                                                     |
| **Text**      | `SkiaTextRenderer`                                                                     |
| **Font**      | `SkiaFontManager`, `BUNDLED_FONT_FAMILIES`                                             |
| **Image**     | `SkiaImageLoader`                                                                      |
| **Paint**     | `parseColor`, `sanitizeSvgPath`, `cssFontFamily`                                       |
| **Thumbnail** | `renderNodeThumbnail`                                                                  |

## License

[MIT](./LICENSE)
