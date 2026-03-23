# @zseven-w/pen-figma

Figma `.fig` file parser and converter for [OpenPencil](https://github.com/nicepkg/openpencil). Import Figma designs directly into the OpenPencil document model.

## Install

```bash
npm install @zseven-w/pen-figma
```

## Features

- Parse binary `.fig` files (Kiwi schema + zstd/zip compression)
- Convert Figma node trees to `PenDocument`
- Multi-page support — import all pages or a single page
- Clipboard paste — detect and convert Figma clipboard HTML
- Image blob resolution

## Usage

### Parse a `.fig` file

```ts
import { parseFigFile, figmaAllPagesToPenDocument } from '@zseven-w/pen-figma'

const figFile = parseFigFile(buffer)
const document = figmaAllPagesToPenDocument(figFile)
```

### Single page import

```ts
import { parseFigFile, getFigmaPages, figmaToPenDocument } from '@zseven-w/pen-figma'

const figFile = parseFigFile(buffer)
const pages = getFigmaPages(figFile)
const document = figmaToPenDocument(figFile, pages[0])
```

### Clipboard paste

```ts
import { isFigmaClipboardHtml, extractFigmaClipboardData, figmaClipboardToNodes } from '@zseven-w/pen-figma'

if (isFigmaClipboardHtml(html)) {
  const data = extractFigmaClipboardData(html)
  const nodes = figmaClipboardToNodes(data)
}
```

## License

MIT
