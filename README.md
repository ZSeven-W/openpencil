# OpenPencil

Open-source vector design tool with a Design-as-Code philosophy. An alternative to [Pencil.dev](https://pencil.dev).

## Features

### Canvas

- Infinite canvas with pan (Space+drag / middle mouse / Hand tool) and zoom (scroll wheel)
- Smart alignment guides with edge, center, and distance snapping
- Dimension labels during object manipulation
- Frame labels and boundary visualization

### Drawing Tools

- **Shapes**: Rectangle, Ellipse, Line, Polygon
- **Frame**: Container with auto-layout support (vertical/horizontal)
- **Text**: Click-to-place text with full typography controls
- **Pen tool**: Bezier curve drawing with anchor points, control handles, and path closure
- **Icon picker**: Search and import icons via Iconify API
- **Image import**: PNG, JPEG, SVG, WebP, GIF (SVG parsed into editable nodes)

### Property Editing

- Position, size, rotation
- Fill: solid color and gradients (linear, radial)
- Stroke: color, width, dash patterns
- Corner radius (unified or per-corner)
- Opacity, visibility, lock, flip (horizontal/vertical)
- Effects: shadow and blur
- Auto-layout: direction, gap, padding, justify-content, align-items

### Layer Panel

- Hierarchical tree view with expand/collapse
- Drag-and-drop reordering with drop-into-children support (above/below/inside zones)
- Visibility and lock toggles per layer
- Rename via double-click
- Context menu: delete, duplicate, group, lock, hide
- Selection synced with canvas

### Parent-Child System

- Frame/Group containers with nested children
- Parent transforms (move, scale, rotate) propagate to children proportionally
- Circular reference prevention when reparenting

### History

- Undo/Redo with batched drag operations (Cmd+Z / Cmd+Shift+Z)
- Up to 300 history states

### Clipboard & Grouping

- Copy, cut, paste, duplicate (Cmd+C/X/V/D)
- Group / ungroup selected elements (Cmd+G / Cmd+Shift+G)

### File Operations

- Save/open `.pen` files (JSON-based, Git-friendly)
- Auto-save with File System Access API
- Export: PNG and SVG with scale options (Cmd+Shift+E)

### Code Generation

- React + Tailwind CSS code from designs
- HTML + CSS code from designs
- View in code panel (Cmd+Shift+C)

### AI Assistant

- Built-in AI chat panel (Cmd+J)
- AI-powered design generation from text prompts
- Design block preview with "Apply Design" action
- Streaming responses with thinking state
- Dual provider: Anthropic API or local Claude Code (OAuth)
- Multi-provider settings: Claude Code, Codex CLI

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| V | Select tool |
| R | Rectangle |
| O | Ellipse |
| L | Line |
| T | Text |
| F | Frame |
| H | Hand (pan) |
| Cmd+A | Select all |
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+C/X/V/D | Copy/Cut/Paste/Duplicate |
| Cmd+G | Group |
| Cmd+Shift+G | Ungroup |
| Cmd+S | Save |
| Cmd+Shift+E | Export |
| Cmd+Shift+C | Code panel |
| Cmd+J | AI chat |
| Delete/Backspace | Delete selected |
| Arrow keys | Nudge (1px, +Shift = 10px) |
| [ / ] | Reorder layers |
| Escape | Deselect / Cancel |

## Tech Stack

- **Framework:** [TanStack Start](https://tanstack.com/start) (React 19, SSR, file-based routing)
- **Canvas:** [Fabric.js](http://fabricjs.com/) v7
- **State:** [Zustand](https://zustand-demo.pmnd.rs/) v5
- **UI:** [shadcn/ui](https://ui.shadcn.com/) (Radix + Tailwind primitives)
- **Styling:** [Tailwind CSS](https://tailwindcss.com/) v4
- **Icons:** [Lucide React](https://lucide.dev/)
- **Server:** [Nitro](https://nitro.build/) (API routes)
- **AI:** [Anthropic SDK](https://docs.anthropic.com/) + [Claude Agent SDK](https://github.com/anthropics/claude-agent-sdk)
- **Runtime:** [Bun](https://bun.sh/)
- **Build:** [Vite](https://vite.dev/) 7

## Getting Started

```bash
bun install
bun --bun run dev
```

Open http://localhost:3000 and click "New Design" to enter the editor.

### AI Configuration

The AI assistant works in two modes:

- **Anthropic API**: Set `ANTHROPIC_API_KEY` in `.env`
- **Local Claude Code**: No config needed — uses Claude Agent SDK with OAuth login as fallback

## Scripts

| Command | Description |
|---|---|
| `bun --bun run dev` | Start dev server on port 3000 |
| `bun --bun run build` | Production build |
| `bun --bun run preview` | Preview production build |
| `bun --bun run test` | Run tests (Vitest) |
| `npx tsc --noEmit` | Type check |

## Project Structure

```
src/
  canvas/              # Fabric.js canvas engine (16 files)
    fabric-canvas.tsx      Canvas component initialization
    canvas-object-factory  Creates Fabric objects from PenNodes
    canvas-object-sync     Syncs object properties Fabric ↔ store
    canvas-sync-lock       Prevents circular sync loops
    canvas-controls        Custom rotation controls and cursors
    canvas-constants       Default colors, zoom limits
    use-canvas-events      Drawing events, tool management
    use-canvas-sync        Bidirectional PenDocument ↔ Fabric sync
    use-canvas-viewport    Zoom, pan, tool cursor switching
    use-canvas-selection   Selection sync Fabric ↔ store
    use-canvas-guides      Smart alignment guides
    guide-utils            Guide calculation and rendering
    pen-tool               Bezier pen tool with anchors/handles
    parent-child-transform Parent transform propagation to children
    use-dimension-label    Size/position labels during manipulation
    use-frame-labels       Frame name/boundary rendering
  components/
    editor/            # Editor layout, toolbar, tool buttons, status bar
    panels/            # Layer panel, property panel (15 files), AI chat, code panel
    shared/            # ColorPicker, NumberInput, ExportDialog, IconPickerDialog, etc.
    ui/                # shadcn/ui primitives (Button, Select, Slider, etc.)
  hooks/               # Keyboard shortcuts
  lib/                 # Utility functions (cn class merging)
  services/
    ai/                # AI chat service, prompts, design generation
    codegen/           # React+Tailwind and HTML+CSS code generators
  stores/              # Zustand stores (canvas, document, history, AI, agent-settings)
  types/               # PenDocument/PenNode types, style types, variables, agent settings
  utils/               # File operations, export, node clone, SVG parser, syntax highlight
  routes/              # TanStack Router pages (/, /editor)
server/
  api/ai/              # Nitro API: streaming chat, generation, agent connection, models
```

## Roadmap

- [ ] Component system (reusable components with instances & overrides)
- [ ] Design variables/tokens with CSS sync
- [ ] Boolean operations (union, subtract, intersect)
- [ ] Multi-page support
- [ ] Collaborative editing

## License

[MIT](./LICENSE)
