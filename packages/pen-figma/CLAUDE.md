# pen-figma

Figma `.fig` binary file parser and converter to PenDocument format.

## Structure

- `src/fig-parser.ts` — `parseFigFile(buffer)`: binary `.fig` file parser using kiwi-schema, supports zstd/zip compression, embedded images. Returns `FigmaDecodedFile`
- `src/figma-node-mapper.ts` — `figmaToPenDocument`, `figmaAllPagesToPenDocument`, `getFigmaPages`, `figmaNodeChangesToPenNodes`: top-level conversion from decoded Figma data to PenDocument/PenNode arrays. Resolves style references (fill, stroke, text, effect)
- `src/figma-tree-builder.ts` — `buildTree`, `buildTreeForClipboard`, `collectComponents`, `collectSymbolTree`, `guidToString`, `isUserPage`: builds parent-child tree from flat Figma node list
- `src/figma-clipboard.ts` — `isFigmaClipboardHtml`, `extractFigmaClipboardData`, `figmaClipboardToNodes`: handles paste from Figma clipboard (base64-encoded `.fig` data in HTML)
- `src/figma-types.ts` — Internal Figma types: `FigmaDecodedFile`, `FigmaNodeChange`, `FigmaGUID`, `FigmaColor`, `FigmaMatrix`, `FigmaPaintType`, `FigmaImportLayoutMode`
- `src/figma-image-resolver.ts` — `resolveImageBlobs`: resolves image blob references from decoded `.fig` data to data URLs
- `src/figma-color-utils.ts` — Color space conversion utilities (Figma 0-1 floats to hex)
- `src/converters/` — Node type converters (dispatcher + per-type modules):
  - `index.ts` — `convertNode` dispatcher, `convertChildren` recursive walker
  - `common.ts` — Shared helpers: `commonProps`, `extractPosition`, `extractRotation`, `mapCornerRadius`, `resolveWidth/Height`, `scaleTreeChildren`, `collectImageBlobs`, `setIconLookup`, `lookupIconByName`
  - `frame-converter.ts` — `convertFrame`, `convertGroup`, `convertComponent`, `convertInstance`
  - `shape-converter.ts` — `convertRectangle`, `convertEllipse`, `convertLine`
  - `text-converter.ts` — `convertText`
  - `path-converter.ts` — `convertVector` (vector, star, polygon, boolean operation)
  - `image-converter.ts` — Image node conversion
- `src/figma-fill-mapper.ts` — Maps Figma paint arrays to `PenFill[]`
- `src/figma-stroke-mapper.ts` — Maps Figma stroke properties to `PenStroke`
- `src/figma-effect-mapper.ts` — Maps Figma effects to `PenEffect[]`
- `src/figma-layout-mapper.ts` — Maps Figma auto-layout to PenNode layout props (direction, gap, padding, alignment)
- `src/figma-text-mapper.ts` — Converts Figma text styles and segments
- `src/figma-vector-decoder.ts` — Decodes Figma vector geometry to SVG path data

## Key exports

`parseFigFile`, `figmaToPenDocument`, `figmaAllPagesToPenDocument`, `getFigmaPages`, `figmaNodeChangesToPenNodes`, `isFigmaClipboardHtml`, `extractFigmaClipboardData`, `figmaClipboardToNodes`, `resolveImageBlobs`, `setIconLookup`

## Testing

```bash
bun --bun vitest run packages/pen-figma/src/converters/__tests__/
```
