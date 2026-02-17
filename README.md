# OpenPencil

Open-source vector design tool with a Design-as-Code philosophy. An alternative to [Pencil.dev](https://pencil.dev).

## Features

- Infinite canvas with pan (Space+drag / middle mouse / Hand tool) and zoom (scroll wheel)
- Drawing tools: Rectangle, Ellipse, Line, Frame, Text
- Real-time property editing: position, size, rotation, fill, stroke, corner radius, opacity
- Layer panel with tree view, selection sync, rename
- Keyboard shortcuts (V/R/O/L/T/F/H, Delete, arrow keys, bracket keys for z-order)

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
  canvas/          # Fabric.js canvas engine, drawing, sync
  components/
    editor/        # Editor layout, toolbar
    panels/        # Layer panel, property panel sections
    shared/        # Reusable UI (ColorPicker, NumberInput, etc.)
  hooks/           # Keyboard shortcuts
  stores/          # Zustand stores (canvas-store, document-store)
  types/           # PenDocument/PenNode types, style types
  routes/          # TanStack Router pages (/, /editor)
```

## Roadmap

- [ ] .pen file save/load (JSON-based, Git-friendly)
- [ ] Undo/redo
- [ ] Component system (reusable components with instances & overrides)
- [ ] Design variables/tokens with CSS sync
- [ ] Code generation (React/HTML/Tailwind)

## License

MIT
