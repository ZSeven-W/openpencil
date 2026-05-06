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

你负责为设计中的单个 chunk 生成代码。你会收到本地 PenNode data，以及对应 framework-specific skill。

## 输入

1. PenNode objects array（chunk 的 nodes，带完整 properties）
2. target framework name 及其 framework-specific rules
3. chunk 的 suggested component name
4. dependency chunks 的 contracts（如果有）

## 输出

你必须输出两部分，中间用仅包含 `---CONTRACT---` 的一行分隔：

1. 生成的代码（完整、可编译的 component）
2. JSON contract block

示例输出：

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

## Node-to-Code 映射规则

### layout nodes（type: "frame" 且有 layout property）

- `layout: "vertical"` → vertical stack（flexbox column、VStack、Column 等）
- `layout: "horizontal"` → horizontal stack（flexbox row、HStack、Row 等）
- `layout: "none"` 或缺省 → absolute/relative positioning
- `gap` → children 之间的 spacing
- `padding` → internal padding（可以是 uniform 或 per-side: top/right/bottom/left）
- `justifyContent` / `alignItems` → stack 内部 alignment
- `clipContent: true` → overflow hidden

### dimensions 处理

- 固定像素 `width`/`height` → 使用精确值
- `width: "fill_container"` → stretch to fill parent（width: 100%、flex: 1 等）
- `height: "fill_container"` → stretch to fill parent height
- Root component：使用 frame 的实际 dimensions 作为 max-width，并支持 responsive scaling

### text nodes（type: "text"）

- `characters` → text content
- 不要翻译、改写或统一语言；按输入 node 中已有 text content 原样映射，除非用户明确要求改文案
- `fontSize`, `fontWeight`, `fontFamily` → typography
- `lineHeight` → line spacing
- `textAlign` → text alignment
- `fill` → text color
- 适当使用 semantic HTML tags（heading 使用 h1-h6，body text 使用 p）

### shape nodes（type: "rectangle", "ellipse", "polygon", "line", "path"）

- 尽可能转换为 CSS shapes（ellipse 可用 border-radius 等）
- `fill` → background color/gradient
- `stroke` → border
- `cornerRadius` → border-radius (can be uniform or per-corner)
- `effects` → box-shadow (for drop shadows), filter (for blur)
- `opacity` → opacity
- `rotation` → transform: rotate()

### image nodes（type: "image"）

- `src` → image source URL
- `objectFit` → object-fit CSS property
- 使用 `<img>`，alt text 从 node name 派生

### variable references

- 以 `$` 开头的值是 variable references
- Web frameworks：使用 CSS custom properties 输出为 `var(--variable-name)`
- Mobile frameworks：输出 literal value，并添加 `/* var(--name) */` comment

### naming

- Component name：使用 chunk 的 `suggestedComponentName`
- CSS classes/variable names：从 node names 派生，使用 kebab-case
- Internal variables：使用 camelCase，名称要有描述性

### 使用 dependency contracts

- 如果 dependency chunk 导出了 component，按其 `componentName` import 并使用
- 遵守 dependency 的 `exportedProps`，传入 required props
- 将 dependency 的 `slots` 用作 children/content areas
