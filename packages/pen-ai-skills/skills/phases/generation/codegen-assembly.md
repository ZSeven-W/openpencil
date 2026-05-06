---
name: codegen-assembly
description: Merge generated code chunks into a single production-grade file with deduplication and responsive design
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 10
budget: 2000
category: base
---

# Code Assembly

你负责把多个 code chunks 组装成一个 production-ready source file。

## 输入

1. chunk results array，每个 result 包含：
   - `chunkId` and `name`
   - 生成的 `code`
   - `contract` (may be missing for degraded chunks — infer from code in that case)
   - Status: `successful`, `degraded` (no contract), or `failed` (code missing)
2. 带 rootLayout 和 sharedStyles 的 `CodePlanFromAI`
3. Design variables 和 theme definitions
4. Target framework name

## 输出

输出一个单一、完整、production-ready 的 source file，必须：

1. 导入所有 dependencies（去重）
2. 定义所有 chunk components
3. 导出一个 root component，并根据 rootLayout 组合所有 chunks
4. 包含 design variables 对应的 CSS variable definitions

## 组装规则

### import 去重

- 合并来自同一 source 的 imports：`{ source: "react", specifiers: ["useState"] }` + `{ source: "react", specifiers: ["useEffect"] }` → `import { useState, useEffect } from 'react'`
- 移除重复 specifiers
- 顺序：framework imports 优先，然后 external libraries，最后 local components

### root component

- 名称：使用 page/document name；没有时默认使用 `"Design"`
- Layout：应用 `rootLayout.direction` 和 `rootLayout.gap` 来排列 chunk components
- 如果 `responsive: true`：添加 responsive breakpoints（mobile-first）

### shared styles

- 将 plan 中描述的 shared styles 提取为 reusable CSS classes 或 styled components
- 在 chunk components 中引用它们，避免重复

### design variables

- 根据提供的 variables 生成 CSS custom property definitions（`:root { --name: value }`）
- 如果定义了 theme variants，也要包含对应变体

### degraded/failed chunks 处理

- 对 **degraded** chunks（有 code、无 contract）：从 raw code 推断 component names 和 imports
- 对 **failed** chunks：插入 placeholder comment：`/* TODO: {chunkName} — generation failed */`
- 始终在文件顶部 comment 中说明哪些 chunks 是 degraded

### 质量规则

- 尽可能用 flex/grid layout 替换 absolute pixel positioning
- 使用 semantic HTML elements（nav, header, main, section, footer, article）
- 确保所有文本可读（足够 contrast，合理 font sizes）
- 为常见宽度添加 responsive breakpoints（640px, 768px, 1024px, 1280px）
