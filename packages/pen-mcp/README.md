# @zseven-w/pen-mcp

[MCP](https://modelcontextprotocol.io/) server for [OpenPencil](https://github.com/ZSeven-W/openpencil) — enables Claude, GPT, Gemini, and other LLMs to read, create, and modify designs through a standard tool protocol.

## Install

```bash
npm install @zseven-w/pen-mcp
# or
bun add @zseven-w/pen-mcp
```

## Overview

`pen-mcp` exposes OpenPencil's full editing API as MCP tools. External AI agents can open documents, inspect the canvas, insert/update/delete nodes, and generate complete designs — all through structured tool calls.

Three workflows are supported:

| Workflow        | Tools                                                      | Best for                             |
| --------------- | ---------------------------------------------------------- | ------------------------------------ |
| **Single-shot** | `insert_node`, `batch_design`                              | Quick edits, single components       |
| **Layered**     | `design_skeleton` → `design_content` × N → `design_refine` | Full-page designs with high fidelity |
| **CRUD**        | `batch_get` → `update_node` / `delete_node`                | Reading & modifying existing content |

## Quick Start

```bash
# Run as stdio MCP server (for Claude Desktop, Cursor, etc.)
npx @zseven-w/pen-mcp

# Or connect to a running OpenPencil instance
op mcp:dev
```

### Claude Desktop Configuration

```json
{
  "mcpServers": {
    "openpencil": {
      "command": "npx",
      "args": ["@zseven-w/pen-mcp"]
    }
  }
}
```

## Tools

### Document & Read Tools

| Tool                | Description                                                                                         |
| ------------------- | --------------------------------------------------------------------------------------------------- |
| `open_document`     | Open an `.op` file or connect to the live Electron canvas. Always call first.                       |
| `batch_get`         | Search and read nodes by type, name regex, or specific IDs. Controls read depth for nested content. |
| `get_selection`     | Get the currently selected nodes on the live canvas.                                                |
| `snapshot_layout`   | Get a compact bounding-box layout tree — useful for spatial understanding.                          |
| `find_empty_space`  | Find available canvas space in a given direction for placing new content.                           |
| `get_design_prompt` | Retrieve segmented design knowledge (schema, layout, roles, text, style, icons, examples).          |

### Node CRUD Tools

| Tool           | Description                                                                         |
| -------------- | ----------------------------------------------------------------------------------- |
| `insert_node`  | Insert a new node with full PenNode data. Supports `postProcess` for auto-defaults. |
| `update_node`  | Shallow-merge properties into an existing node.                                     |
| `delete_node`  | Delete a node and all its children.                                                 |
| `move_node`    | Reparent a node to a new container.                                                 |
| `copy_node`    | Deep-clone a node with new IDs under a target parent.                               |
| `replace_node` | Replace a node entirely with new data at the same position.                         |
| `import_svg`   | Import a local SVG file as editable PenNodes.                                       |

### Batch Design DSL

`batch_design` accepts a compact DSL — one operation per line:

```
root=I(null, { "type": "frame", "name": "Page", "width": 1200, ... })
header=I(root, { "type": "frame", "name": "Header", ... })
U(header, { "fill": [{ "type": "solid", "color": "#1A1A2E" }] })
logo=C("existing-logo", header, { "x": 24 })
M("floating-btn", header)
D("old-section")
```

| Op    | Syntax                                     | Description       |
| ----- | ------------------------------------------ | ----------------- |
| **I** | `binding=I(parent, { data })`              | Insert node       |
| **U** | `U(path, { updates })`                     | Update properties |
| **C** | `binding=C(source, parent, { overrides })` | Copy node         |
| **R** | `binding=R(path, { newData })`             | Replace node      |
| **M** | `M(nodeId, parent, index?)`                | Move node         |
| **D** | `D(nodeId)`                                | Delete node       |

### Layered Generation Workflow

For high-fidelity multi-section designs:

```
1. design_skeleton  → Create root frame + section placeholders
2. design_content   → Fill each section with content nodes (call per section)
3. design_refine    → Run full-tree validation and auto-fixes
```

### Page Management

| Tool             | Description                                |
| ---------------- | ------------------------------------------ |
| `add_page`       | Add a new page to the document             |
| `remove_page`    | Remove a page (cannot remove the last one) |
| `rename_page`    | Rename a page                              |
| `reorder_page`   | Move a page to a new index                 |
| `duplicate_page` | Deep-clone a page with new IDs             |

### Post-Processing

All creation tools support `postProcess=true` for automatic:

- Semantic role defaults (button padding, input height, card radius, etc.)
- Icon name → SVG path resolution (Lucide icon set)
- Card row equalization in horizontal layouts
- Text height estimation
- Frame height expansion when content overflows
- `clipContent` auto-addition for frames with `cornerRadius` + images

## Design Prompt Sections

`get_design_prompt(section)` returns focused subsets of design knowledge:

| Section      | Content                                                                   |
| ------------ | ------------------------------------------------------------------------- |
| `schema`     | PenNode type definitions and property reference                           |
| `layout`     | Flexbox layout engine rules (gap, padding, justify, align)                |
| `roles`      | Semantic roles and their auto-defaults (button, input, card, navbar, ...) |
| `text`       | Typography rules, CJK support, copywriting guidelines                     |
| `style`      | Visual style policy (colors, fonts, aesthetic)                            |
| `icons`      | Feather/Lucide icon naming conventions                                    |
| `examples`   | Complete design examples with DSL                                         |
| `guidelines` | Design tips (cards, inputs, phone mockups, hero sections)                 |
| `planning`   | Layered workflow guide with section decomposition rules                   |

## Live Canvas Sync

When connected to a running OpenPencil desktop app, changes made via MCP tools appear on the canvas in real-time. The sync is bidirectional — user edits on the canvas are reflected in subsequent `batch_get` / `snapshot_layout` calls.

## Programmatic Usage

```typescript
import { configureMcpHooks, MCP_DEFAULT_PORT } from '@zseven-w/pen-mcp';

// Configure custom hooks (optional)
configureMcpHooks({
  onDocumentOpen: (path) => console.log(`Opened: ${path}`),
  onNodeInsert: (node) => console.log(`Inserted: ${node.id}`),
});
```

## License

[MIT](./LICENSE)
