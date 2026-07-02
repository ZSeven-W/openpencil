# @zseven-w/pen-figma

Figma `.fig` file parser and converter for [OpenPencil](https://github.com/ZSeven-W/openpencil). Import Figma designs directly into the OpenPencil document model — binary file parsing, multi-page support, clipboard paste, and full style conversion.

## Install

```bash
npm install @zseven-w/pen-figma
# or
bun add @zseven-w/pen-figma
```

## Overview

This package handles the complete pipeline from Figma's proprietary binary format to OpenPencil's `PenDocument`:

```
.fig binary → Kiwi schema decode → FigmaNodeChange[] → tree building → PenNode[] → PenDocument
```

It supports:

- Binary `.fig` files (Kiwi schema + zstd/zip compression)
- Figma clipboard HTML (copy from Figma → paste in OpenPencil)
- All node types: frames, groups, components, instances, shapes, text, vectors, images
- Full style conversion: fills, strokes, effects, gradients, auto-layout, typography

## Usage

### Parse a `.fig` file

```typescript
import { parseFigFile, figmaAllPagesToPenDocument } from '@zseven-w/pen-figma';

const buffer = await fs.readFile('design.fig');
const figFile = parseFigFile(buffer);
const document = figmaAllPagesToPenDocument(figFile);

console.log(`Imported ${document.pages?.length} pages`);
```

### Single page import

```typescript
import { parseFigFile, getFigmaPages, figmaToPenDocument } from '@zseven-w/pen-figma';

const figFile = parseFigFile(buffer);
const pages = getFigmaPages(figFile);

// Import just the first page
const document = figmaToPenDocument(figFile, pages[0]);
```

### Clipboard paste

Detect and convert Figma clipboard data (when users copy from Figma and paste into OpenPencil):

```typescript
import {
  isFigmaClipboardHtml,
  extractFigmaClipboardData,
  figmaClipboardToNodes,
} from '@zseven-w/pen-figma';

document.addEventListener('paste', (e) => {
  const html = e.clipboardData?.getData('text/html');
  if (html && isFigmaClipboardHtml(html)) {
    const data = extractFigmaClipboardData(html);
    const nodes = figmaClipboardToNodes(data);
    // Insert nodes into document...
  }
});
```

### Convert individual nodes

For lower-level access (e.g., incremental sync):

```typescript
import { figmaNodeChangesToPenNodes } from '@zseven-w/pen-figma';

const penNodes = figmaNodeChangesToPenNodes(figmaNodeChanges, options);
```

### Icon resolution

Register an icon lookup function for converting Figma component instances to icon nodes:

```typescript
import { setIconLookup } from '@zseven-w/pen-figma';

setIconLookup((name) => {
  // Return SVG path data for the icon name, or null
  return iconRegistry[name]?.path ?? null;
});
```

### Image resolution

Resolve embedded image blob references to data URLs:

```typescript
import { resolveImageBlobs } from '@zseven-w/pen-figma';

const images = resolveImageBlobs(figFile);
// Map<blobHash, dataURL>
```

## Conversion Coverage

### Node Types

| Figma Type | PenNode Type | Notes                                       |
| ---------- | ------------ | ------------------------------------------- |
| FRAME      | `frame`      | With auto-layout, constraints, clip content |
| GROUP      | `group`      | Preserves child transforms                  |
| COMPONENT  | `frame`      | Marked as reusable                          |
| INSTANCE   | `frame`      | Resolved with overrides                     |
| RECTANGLE  | `rectangle`  | Corner radius (uniform + per-corner)        |
| ELLIPSE    | `ellipse`    | Arc support                                 |
| LINE       | `line`       | Stroke properties                           |
| TEXT       | `text`       | Rich text with styled segments              |
| VECTOR     | `path`       | Complex paths, boolean ops, stars, polygons |
| IMAGE      | `image`      | Embedded or referenced                      |

### Styles

| Figma Style      | Conversion                                                            |
| ---------------- | --------------------------------------------------------------------- |
| Solid fills      | `PenFill` with solid type                                             |
| Linear gradients | `PenFill` with gradient stops + angle                                 |
| Radial gradients | `PenFill` with center + radius                                        |
| Image fills      | `PenFill` with image source                                           |
| Strokes          | `PenStroke` with cap, join, dash                                      |
| Drop shadows     | `PenEffect` shadow                                                    |
| Inner shadows    | `PenEffect` inner shadow                                              |
| Blur             | `PenEffect` blur                                                      |
| Auto-layout      | `layout`, `gap`, `padding`, `justifyContent`, `alignItems`            |
| Text styles      | `fontSize`, `fontWeight`, `fontFamily`, `lineHeight`, `letterSpacing` |
| Rich text        | `StyledTextSegment[]` with per-run styles                             |

## API Reference

| Function                              | Description                                   |
| ------------------------------------- | --------------------------------------------- |
| `parseFigFile(buffer)`                | Parse binary `.fig` file → `FigmaDecodedFile` |
| `figmaAllPagesToPenDocument(file)`    | Convert all pages → `PenDocument`             |
| `figmaToPenDocument(file, page)`      | Convert single page → `PenDocument`           |
| `getFigmaPages(file)`                 | List available pages                          |
| `figmaNodeChangesToPenNodes(changes)` | Convert raw node changes → `PenNode[]`        |
| `isFigmaClipboardHtml(html)`          | Detect Figma clipboard data                   |
| `extractFigmaClipboardData(html)`     | Extract base64 `.fig` from clipboard HTML     |
| `figmaClipboardToNodes(data)`         | Convert clipboard data → `PenNode[]`          |
| `resolveImageBlobs(file)`             | Resolve image references → data URLs          |
| `setIconLookup(fn)`                   | Register icon name → SVG path resolver        |

## License

[MIT](./LICENSE)
