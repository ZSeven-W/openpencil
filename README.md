# OpenPencil

Open-source vector design tool with a Design-as-Code philosophy. An alternative to [Pencil.dev](https://pencil.dev).

Available as a **web app** and **desktop app** (macOS / Windows / Linux via Electron).

## Features

### Canvas

- Infinite canvas with pan (Space+drag / middle mouse / Hand tool) and zoom (scroll wheel)
- Smart alignment guides with edge, center, and distance snapping
- Dimension labels during object manipulation
- Frame labels and boundary visualization
- Double-click to enter frames with visual overlay
- Advanced drag-and-drop: drag into auto-layout frames with insertion indicators, reparenting, reorder within layout

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
- Variable binding: bind any property to a design variable via variable picker
- Per-layer export: export individual layers to PNG/SVG with scale options (1x/2x/3x)

### Design Variables & Tokens

- **Variables panel**: Floating resizable panel with theme management (Cmd+Shift+V)
- **Variable types**: Color (picker + hex + opacity), Number, String
- **Multi-theme support**: Create multiple theme axes (e.g. Theme-1, Theme-2), each with variants (e.g. Default, Dark, High Contrast)
- **`$variable` references**: Bind node properties (fill, stroke, opacity, gap, padding) to variables
- **CSS sync**: Auto-generate CSS custom properties (`:root { --color-1: #fff; }`) with per-theme variant blocks
- **Code generation**: React/Tailwind and HTML/CSS output uses `var(--name)` for variable-bound properties
- **Live resolution**: Variables resolved on-the-fly for canvas rendering, preserved as `$refs` in document

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
- CSS Variables from design tokens
- View in code panel (Cmd+Shift+C)

### AI Assistant

- Built-in AI chat panel (Cmd+J)
- AI-powered design generation from text prompts
- Orchestrator-based parallel design generation: decomposes requests into spatial sub-tasks for faster output
- Design block preview with "Apply Design" action
- Streaming responses with thinking state and JSONL real-time canvas insertion
- Context optimizer: sliding window history trimming to prevent unbounded context growth
- Dual provider: Anthropic API or local Claude Code (OAuth)
- Multi-provider settings: Claude Code, Codex CLI, OpenCode

### Editor UI

- Dark / light theme toggle (persisted to localStorage)
- Fullscreen mode
- Draggable, snap-to-corner AI chat panel

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| V | Select tool |
| R | Rectangle |
| O | Ellipse |
| L | Line |
| T | Text |
| F | Frame |
| P | Path (pen tool) |
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
| Cmd+Shift+V | Variables panel |
| Cmd+J | AI chat |
| Cmd+, | Agent settings |
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
- **Desktop:** [Electron](https://www.electronjs.org/) 35 + [electron-builder](https://www.electron.build/)
- **AI:** [Anthropic SDK](https://docs.anthropic.com/) + [Claude Agent SDK](https://github.com/anthropics/claude-agent-sdk) + [OpenCode SDK](https://github.com/opencode-ai/sdk)
- **Runtime:** [Bun](https://bun.sh/)
- **Build:** [Vite](https://vite.dev/) 7
- **CI/CD:** GitHub Actions

## Getting Started

### Web (Development)

```bash
bun install
bun --bun run dev
```

Open http://localhost:3000 and click "New Design" to enter the editor.

### Electron (Desktop)

```bash
# Development: starts Vite dev server + Electron
bun run electron:dev

# Production build (current platform)
bun run electron:build
```

### AI Configuration

The AI assistant works in multiple modes:

- **Anthropic API**: Set `ANTHROPIC_API_KEY` in `.env`
- **Local Claude Code**: No config needed — uses Claude Agent SDK with OAuth login as fallback
- **OpenCode**: Connect via OpenCode SDK for additional model support

## Scripts

| Command | Description |
|---|---|
| `bun --bun run dev` | Start web dev server on port 3000 |
| `bun --bun run build` | Production web build |
| `bun --bun run preview` | Preview production build |
| `bun --bun run test` | Run tests (Vitest) |
| `npx tsc --noEmit` | Type check |
| `bun run electron:dev` | Start Vite + Electron for desktop dev |
| `bun run electron:compile` | Compile electron/ with esbuild |
| `bun run electron:build` | Full Electron package (web build + compile + electron-builder) |

## CI / CD

### CI (`ci.yml`)

Runs on every push and PR to `main` / `v0.0.1`:

1. **Lint & Test** — type check (`tsc --noEmit`) + unit tests (`vitest`)
2. **Build Web** — production web build, uploads `.output/` as artifact

### Build Electron (`build-electron.yml`)

Triggered by version tags (`v*`) or manual dispatch:

1. **Build** — parallel matrix across macOS, Windows, Linux
   - macOS: `.dmg` + `.zip`
   - Windows: `.exe` (NSIS installer + portable)
   - Linux: `.AppImage` + `.deb`
2. **Release** — creates a draft GitHub Release with all platform artifacts

To create a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Project Structure

```text
src/
  canvas/              # Fabric.js canvas engine (25 files: sync, events, guides, drag-drop, pen tool, etc.)
  variables/           # Design variables/tokens system (resolve, replace refs)
  components/
    editor/            # Editor layout, toolbar, tool buttons, top bar, status bar
    panels/            # Layer panel, property panel, AI chat, code panel, variables panel, export section
    shared/            # ColorPicker, NumberInput, VariablePicker, ExportDialog, etc.
    icons/             # Provider logos (Claude, OpenAI, OpenCode)
    ui/                # shadcn/ui primitives (Button, Select, Slider, Switch, etc.)
  hooks/               # Keyboard shortcuts
  lib/                 # Utility functions (cn class merging)
  services/
    ai/                # AI chat, orchestrator, context optimizer, design generation, prompts
    codegen/           # React+Tailwind, HTML+CSS, and CSS variables generators
  stores/              # Zustand stores (canvas, document, history, AI, agent-settings)
  types/               # PenDocument/PenNode types, style types, variables, agent settings, Electron IPC
  utils/               # File operations, export, node clone, SVG parser, syntax highlight
  routes/              # TanStack Router pages (/, /editor)
electron/
  main.ts              # Electron main process (window, Nitro server, IPC, fullscreen handling)
  preload.ts           # Context bridge for renderer ↔ main IPC
server/
  api/ai/              # Nitro API: streaming chat, generation, agent connection, models
  utils/               # Server utilities: Claude CLI resolver, OpenCode client manager
.github/
  workflows/
    ci.yml             # CI: type check, test, web build
    build-electron.yml # Electron build for macOS/Windows/Linux + GitHub Release
```

## Roadmap

- [ ] Component system (reusable components with instances & overrides)
- [x] Design variables/tokens with CSS sync
- [ ] Boolean operations (union, subtract, intersect)
- [ ] Multi-page support
- [ ] Collaborative editing

## License

[MIT](./LICENSE)
