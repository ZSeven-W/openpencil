# pen-renderer

Standalone CanvasKit/Skia renderer for OpenPencil (.op) design files. No React, no Zustand -- pure TypeScript + CanvasKit WASM.

## Structure

- `src/renderer.ts` — `PenRenderer`: high-level read-only renderer. Methods: `init(canvas)`, `setDocument(doc)`, `render()`, `zoomToFit()`, `zoomTo(zoom, x, y)`, `pan(dx, dy)`, `hitTest(x, y)`, `dispose()`. Manages viewport, render nodes, spatial index, and frame labels
- `src/document-flattener.ts` — Tree flattening with layout resolution: `flattenToRenderNodes` (recursive walk producing `RenderNode[]` with absolute positions), `resolveRefs` (ref node resolution), `remapIds`, `premeasureTextHeights` (Canvas 2D text measurement for accurate heights), `collectReusableIds`, `collectInstanceIds`
- `src/node-renderer.ts` — `SkiaNodeRenderer`: core draw calls for all node types (rectangles, ellipses, paths, text, images, icons, lines, polygons). Handles fills (solid, gradient, image), strokes, effects (shadow, blur), corner radius, clip content, opacity, blend mode
- `src/text-renderer.ts` — `SkiaTextRenderer`: text rendering sub-system with both vector (Paragraph API) and bitmap (Canvas 2D rasterization) paths. FIFO caches with byte-based eviction limits (256 MB text, 64 MB paragraph)
- `src/paint-utils.ts` — Color/paint utilities: `parseColor` (hex to CanvasKit Color4f), `cornerRadiusValue`, `cornerRadii`, `resolveFillColor`, `resolveStrokeColor`, `resolveStrokeWidth`, `wrapLine`, `cssFontFamily`
- `src/path-utils.ts` — SVG path utilities: `sanitizeSvgPath` (normalize for CanvasKit parser), `hasInvalidNumbers`, `tryManualPathParse`
- `src/image-loader.ts` — `SkiaImageLoader`: async image loading with browser Image element, Canvas 2D rasterization, CanvasKit Image conversion, and caching. Supports custom source resolvers
- `src/font-manager.ts` — `SkiaFontManager`: font management with bundled fonts (Inter, Poppins, Roboto, etc.) and Google Fonts CSS fetching. `BUNDLED_FONT_FAMILIES` constant
- `src/spatial-index.ts` — `SpatialIndex`: R-tree backed (rbush) spatial queries for hit testing. Methods: `rebuild`, `hitTest`, `searchRect`, `get`. Returns results topmost-first by render order
- `src/viewport.ts` — Viewport math: `viewportMatrix` (3x3 transform for CanvasKit), `screenToScene`, `sceneToScreen`, `zoomToPoint`, `getViewportBounds`, `isRectInViewport`
- `src/init.ts` — `loadCanvasKit(options)`, `getCanvasKit()`: singleton CanvasKit WASM loader
- `src/render-node-thumbnail.ts` — `renderNodeThumbnail(node, context)`: offscreen thumbnail helper for individual nodes (used by git conflict UI). Returns data URL or null
- `src/types.ts` — `RenderNode` (node + absolute bounds + clip rect), `PenRendererOptions`, `IconLookupFn`

## Key exports

Primary: `loadCanvasKit`, `getCanvasKit`, `PenRenderer`

Low-level (for editor re-use): `SkiaNodeRenderer`, `SkiaTextRenderer`, `SkiaFontManager`, `SkiaImageLoader`, `SpatialIndex`, `flattenToRenderNodes`, `resolveRefs`, `premeasureTextHeights`, `viewportMatrix`, `screenToScene`, `sceneToScreen`, `zoomToPoint`, `parseColor`, `sanitizeSvgPath`, `renderNodeThumbnail`

## Testing

```bash
bun --bun vitest run packages/pen-renderer/src/__tests__/
```
