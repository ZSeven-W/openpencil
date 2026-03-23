# @zseven-w/pen-renderer

Standalone CanvasKit/Skia renderer for [OpenPencil](https://github.com/nicepkg/openpencil) design files. Render `.op` documents to a GPU-accelerated canvas — works in browsers, Node.js, and headless environments.

## Install

```bash
npm install @zseven-w/pen-renderer canvaskit-wasm
```

`canvaskit-wasm` is a peer dependency — you provide the WASM binary.

## Usage

```ts
import { loadCanvasKit, PenRenderer } from '@zseven-w/pen-renderer'

// Initialize CanvasKit
await loadCanvasKit()

// Create renderer on a canvas element
const renderer = new PenRenderer(canvas, document, {
  width: 1920,
  height: 1080,
})

// Render
renderer.render()
```

## API

### High-level

- **`loadCanvasKit()`** — Initialize the CanvasKit WASM module
- **`PenRenderer`** — Full-featured renderer with viewport, selection, and interaction support

### Document Flattening

Pre-process documents for rendering:

```ts
import { flattenToRenderNodes, resolveRefs, premeasureTextHeights } from '@zseven-w/pen-renderer'
```

### Viewport Utilities

```ts
import { viewportMatrix, screenToScene, sceneToScreen, zoomToPoint } from '@zseven-w/pen-renderer'
```

### Low-level Renderers

For custom rendering pipelines:

- `SkiaNodeRenderer` — Renders individual nodes to a Skia canvas
- `SkiaTextRenderer` — Text layout and rendering
- `SkiaFontManager` — Font loading and management
- `SkiaImageLoader` — Async image loading with caching
- `SpatialIndex` — R-tree spatial index for hit testing

## License

MIT
