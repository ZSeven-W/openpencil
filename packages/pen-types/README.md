# @zseven-w/pen-types

Type definitions for the [OpenPencil](https://github.com/nicepkg/openpencil) document model.

## Install

```bash
npm install @zseven-w/pen-types
```

## What's Included

This package provides all TypeScript types and interfaces for the OpenPencil design file format (`.op`):

- **Document model** — `PenDocument`, `PenPage`, `PenNode` and all node types (`FrameNode`, `RectangleNode`, `EllipseNode`, `TextNode`, `ImageNode`, `PathNode`, etc.)
- **Styles** — `PenFill` (solid, gradient, image), `PenStroke`, `PenEffect` (blur, shadow), `BlendMode`, `StyledTextSegment`
- **Variables & Themes** — `VariableDefinition`, `VariableValue`, `ThemedValue`
- **Canvas state** — `ToolType`, `ViewportState`, `SelectionState`, `CanvasInteraction`
- **UIKit** — `UIKit`, `KitComponent`, `ComponentCategory`
- **Theme presets** — `ThemePreset`, `ThemePresetFile`
- **Design spec** — `DesignMdSpec`, `DesignMdColor`, `DesignMdTypography`

## Usage

```ts
import type { PenDocument, PenNode, FrameNode } from '@zseven-w/pen-types'
```

## License

MIT
