---
name: codegen-chunk
description: Universal rules for generating code from a PenNode chunk — layout semantics, naming, property mapping
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 10
budget: 3000
category: base
---

# Code Chunk Generation

You generate code for a single chunk of a design. You receive local PenNode data + a framework-specific skill.

## Input

1. An array of PenNode objects (the chunk's nodes with full properties)
2. The target framework name and its framework-specific rules
3. The chunk's suggested component name
4. Contracts from dependency chunks (if any)

## Output

You MUST output TWO things separated by a line containing only `---CONTRACT---`:

1. The generated code (complete, compilable component)
2. A JSON contract block

Example output:
```
import React from 'react'

export function NavBar() {
  return (
    <nav className="flex items-center justify-between px-6 py-4">
      <div className="text-xl font-bold">Logo</div>
      <div className="flex gap-4">
        <a href="#">Home</a>
        <a href="#">About</a>
      </div>
    </nav>
  )
}
---CONTRACT---
{
  "chunkId": "chunk-1",
  "componentName": "NavBar",
  "exportedProps": [],
  "slots": [],
  "cssClasses": [],
  "cssVariables": [],
  "imports": [{ "source": "react", "specifiers": ["default"] }]
}
```

## Node-to-Code Mapping Rules

### Layout Nodes (type: "frame" with layout property)
- `layout: "vertical"` → vertical stack (flexbox column, VStack, Column, etc.)
- `layout: "horizontal"` → horizontal stack (flexbox row, HStack, Row, etc.)
- `layout: "none"` or absent → absolute/relative positioning
- `gap` → spacing between children
- `padding` → internal padding (can be uniform or per-side: top/right/bottom/left)
- `justifyContent` / `alignItems` → alignment within the stack
- `clipContent: true` → overflow hidden

### Dimension Handling
- Fixed `width`/`height` in pixels → use exact values
- `width: "fill_container"` → stretch to fill parent (width: 100%, flex: 1, etc.)
- `height: "fill_container"` → stretch to fill parent height
- Root component: use the frame's actual dimensions as max-width with responsive scaling

### Text Nodes (type: "text")
- `characters` → text content
- `fontSize`, `fontWeight`, `fontFamily` → typography
- `lineHeight` → line spacing
- `textAlign` → text alignment
- `fill` → text color
- Use semantic HTML tags when appropriate (h1-h6 for headings, p for body text)

### Shape Nodes (type: "rectangle", "ellipse", "polygon", "line", "path")
- Convert to CSS shapes where possible (border-radius for ellipse, etc.)
- `fill` → background color/gradient
- `stroke` → border
- `cornerRadius` → border-radius (can be uniform or per-corner)
- `effects` → box-shadow (for drop shadows), filter (for blur)
- `opacity` → opacity
- `rotation` → transform: rotate()

### Image Nodes (type: "image")
- `src` → image source URL
- `objectFit` → object-fit CSS property
- Use `<img>` with proper alt text derived from node name

### Variable References
- Values starting with `$` are variable references
- Web frameworks: output as `var(--variable-name)` using CSS custom properties
- Mobile frameworks: output as literal value with `/* var(--name) */` comment

### Naming
- Component name: use the chunk's `suggestedComponentName`
- CSS classes/variable names: derive from node names, kebab-case
- Internal variables: camelCase, descriptive

### Using Dependency Contracts
- If a dependency chunk exported a component, import and use it by its `componentName`
- Respect the dependency's `exportedProps` — pass required props
- Use dependency's `slots` as children/content areas
