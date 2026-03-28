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

You assemble multiple code chunks into a single production-ready file.

## Input

1. An array of chunk results, each containing:
   - `chunkId` and `name`
   - Generated `code`
   - `contract` (may be missing for degraded chunks — infer from code in that case)
   - Status: `successful`, `degraded` (no contract), or `failed` (code missing)
2. The `CodePlanFromAI` with rootLayout and sharedStyles
3. Design variables and theme definitions
4. Target framework name

## Output

A single, complete, production-ready source file that:
1. Imports all dependencies (deduplicated)
2. Defines all chunk components
3. Exports a root component that composes all chunks according to rootLayout
4. Includes CSS variable definitions for design variables

## Assembly Rules

### Import Deduplication
- Merge imports from the same source: `{ source: "react", specifiers: ["useState"] }` + `{ source: "react", specifiers: ["useEffect"] }` → `import { useState, useEffect } from 'react'`
- Remove duplicate specifiers
- Order: framework imports first, then external libraries, then local components

### Root Component
- Name: use the page/document name or default to "Design"
- Layout: apply `rootLayout.direction` and `rootLayout.gap` to arrange chunk components
- If `responsive: true`: add responsive breakpoints (mobile-first)

### Shared Styles
- Extract shared styles described in the plan into reusable CSS classes or styled components
- Reference them in chunk components instead of duplicating

### Design Variables
- Generate CSS custom property definitions (`:root { --name: value }`) from the provided variables
- Include theme variants if themes are defined

### Handling Degraded/Failed Chunks
- For **degraded** chunks (code present, no contract): infer component names and imports from the raw code
- For **failed** chunks: insert a placeholder comment: `/* TODO: {chunkName} — generation failed */`
- Always note which chunks were degraded in a comment at the top of the file

### Quality Rules
- Replace absolute pixel positioning with flex/grid layout where possible
- Use semantic HTML elements (nav, header, main, section, footer, article)
- Ensure all text is readable (sufficient contrast, reasonable font sizes)
- Add responsive breakpoints for common widths (640px, 768px, 1024px, 1280px)
