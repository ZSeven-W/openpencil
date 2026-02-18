# OpenPencil

Open-source vector design tool with a Design-as-Code philosophy. An alternative to [Pencil.dev](https://pencil.dev).

## Features

- **Canvas**: Infinite canvas with pan (Space+drag / middle mouse / Hand tool) and zoom (scroll wheel), smart guides & snapping
- **Drawing tools**: Rectangle, Ellipse, Line, Frame, Text
- **Property editing**: Position, size, rotation, fill (solid + gradient), stroke, corner radius, opacity, effects (shadow, blur)
- **Layer panel**: Tree view, drag reorder, visibility toggle, lock, context menu, selection sync, rename
- **Undo/Redo**: Full history with batched drag operations (Cmd+Z / Cmd+Shift+Z)
- **Clipboard**: Copy, cut, paste, duplicate (Cmd+C/X/V/D)
- **Grouping**: Group / ungroup selected elements (Cmd+G / Cmd+Shift+G)
- **File operations**: Save/open .pen files (JSON-based, Git-friendly), auto-save support
- **Export**: PNG and SVG export with scale options (Cmd+Shift+E)
- **Code generation**: Generate React+Tailwind or HTML+CSS code from designs (Cmd+Shift+C)
- **AI assistant**: Built-in AI chat panel for design assistance (Cmd+J)
- **Keyboard shortcuts**: Tool keys (V/R/O/L/T/F/H), Delete, arrow nudge, bracket keys for z-order, Cmd+A select all

## Tech Stack

- **Framework:** [TanStack Start](https://tanstack.com/start) (React 19, SSR, file-based routing)
- **Canvas:** [Fabric.js](http://fabricjs.com/) v7
- **State:** [Zustand](https://zustand-demo.pmnd.rs/) v5
- **Styling:** [Tailwind CSS](https://tailwindcss.com/) v4
- **Icons:** [Lucide React](https://lucide.dev/)
- **Runtime:** [Bun](https://bun.sh/)
- **Build:** [Vite](https://vite.dev/) 7

## Getting Started

```bash
bun install
bun --bun run dev
```

Open http://localhost:3000 and click "New Design" to enter the editor.

## Scripts

| Command | Description |
|---|---|
| `bun --bun run dev` | Start dev server on port 3000 |
| `bun --bun run build` | Production build |
| `bun --bun run preview` | Preview production build |
| `bun --bun run test` | Run tests (Vitest) |

## Project Structure

```
src/
  canvas/          # Fabric.js canvas engine, drawing, sync, guides
  components/
    editor/        # Editor layout, toolbar
    panels/        # Layer panel, property panel, AI chat, code panel
    shared/        # Reusable UI (ColorPicker, NumberInput, ExportDialog, etc.)
  hooks/           # Keyboard shortcuts
  services/
    ai/            # AI chat service, prompts, design generation
    codegen/       # React and HTML code generators
  stores/          # Zustand stores (canvas, document, history, AI)
  types/           # PenDocument/PenNode types, style types
  utils/           # File operations, export, node clone, syntax highlight
  routes/          # TanStack Router pages (/, /editor)
server/
  api/             # Server-side API endpoints
```

## Roadmap

- [ ] Component system (reusable components with instances & overrides)
- [ ] Design variables/tokens with CSS sync
- [ ] Path / pen tool
- [ ] Boolean operations (union, subtract, intersect)
- [ ] Multi-page support
- [ ] Collaborative editing

## License

MIT
