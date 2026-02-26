<p align="center">
  <img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" />
  <img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" />
  <img src="https://img.shields.io/badge/runtime-Bun-f472b6" alt="Bun" />
  <img src="https://img.shields.io/badge/platform-Web%20%7C%20macOS%20%7C%20Windows%20%7C%20Linux-blue" alt="Platform" />
</p>
<p align="center">
  <a href="https://discord.gg/fE9STbMG">
    <img src="./public/logo-discord.svg" alt="Discord" width="18" />
    <strong> Join the OpenPencil Discord Community</strong>
  </a>
</p>

# OpenPencil

Open-source vector design tool with a **Design-as-Code** philosophy. An alternative to Pencil.

Design visually on an infinite canvas, generate code from designs, and let AI build entire screens from a single prompt — all in one tool that runs in your browser or as a native desktop app.

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil demo video (click to play)" width="100%" />
  </a>
</p>
<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4"><strong>▶ Watch demo video</strong></a>
</p>

## Highlights

- **Infinite canvas** — pan, zoom, smart alignment guides, drag-and-drop into auto-layout frames
- **Drawing tools** — Rectangle, Ellipse, Line, Pen (Bezier), Frame, Text, Icon picker, Image import
- **Auto-layout** — Vertical / horizontal layout with gap, padding, justify, align — like CSS Flexbox
- **Design variables** — Color / number / string tokens with multi-theme support and CSS sync
- **Code generation** — Export to React + Tailwind, HTML + CSS, or CSS Variables
- **AI assistant** — Generate full-page designs from text prompts with streaming canvas insertion
- **Desktop app** — Native macOS, Windows, and Linux builds via Electron
- **`.pen` files** — JSON-based, human-readable, Git-friendly

## Quick Start

### Prerequisites

- [Bun](https://bun.sh/) >= 1.0
- [Node.js](https://nodejs.org/) >= 18 (for `npx` tooling)

### Web (Development)

```bash
bun install
bun --bun run dev
```

Open http://localhost:3000 and click **New Design** to enter the editor.

### Desktop (Electron)

```bash
# Development: Vite dev server + Electron
bun run electron:dev

# Production build (current platform)
bun run electron:build
```

### Desktop Auto Update (GitHub Releases)

- The packaged app checks GitHub Releases automatically on startup and every hour.
- When a new version finishes downloading, the app shows a **Restart & Install** prompt.
- Releases must be **published (not draft)** and include update metadata (`latest*.yml`, `*.blockmap`).

### AI Configuration

The AI assistant supports multiple providers:

| Provider | Setup |
|---|---|
| **Anthropic API** | Set `ANTHROPIC_API_KEY` in `.env` |
| **Claude Code** | No config — uses Claude Agent SDK with local OAuth |
| **OpenCode** | Connect via OpenCode SDK in Agent Settings (Cmd+,) |

## Features

### Canvas

- Infinite canvas with pan (Space+drag / middle mouse) and zoom (scroll wheel)
- Smart alignment guides with edge, center, and distance snapping
- Dimension labels during manipulation
- Frame labels and boundaries
- Double-click to enter frames (Figma-style)
- Drag-and-drop into auto-layout frames with insertion indicators

### Drawing Tools

- **Shapes** — Rectangle, Ellipse, Line, Polygon
- **Frame** — Container with optional auto-layout (vertical / horizontal)
- **Text** — Click-to-place with full typography controls
- **Pen tool** — Bezier curve drawing with anchor points, control handles, path closure
- **Icon picker** — Search and import icons via Iconify API
- **Image import** — PNG, JPEG, SVG, WebP, GIF (SVG parsed into editable PenNodes)

### Property Editing

- Position, size, rotation
- Fill: solid color and gradients (linear, radial)
- Stroke: color, width, dash patterns
- Corner radius (unified or per-corner)
- Opacity, visibility, lock, flip
- Effects: shadow and blur
- Auto-layout: direction, gap, padding, justify-content, align-items
- Variable binding via variable picker

### Design Variables & Tokens

- **Variables panel** (Cmd+Shift+V): floating, resizable
- **Types**: Color (picker + hex + opacity), Number, String
- **Multi-theme**: multiple theme axes, each with variants (e.g. Light / Dark)
- **`$variable` references**: bind fill, stroke, opacity, gap, padding to tokens
- **CSS sync**: auto-generated CSS custom properties with per-theme variant blocks
- **Code output**: `var(--name)` for variable-bound properties

### Layer Panel

- Hierarchical tree view with expand/collapse
- Drag-and-drop reordering (above / below / inside zones)
- Visibility and lock toggles, rename, context menu
- Selection synced with canvas

### Code Generation

- React + Tailwind CSS
- HTML + CSS
- CSS Variables from design tokens
- Code panel: Cmd+Shift+C

### AI Assistant

- Chat panel (Cmd+J) with streaming responses
- Orchestrator-based parallel generation: decomposes pages into spatial sub-tasks
- Real-time JSONL streaming with staggered fade-in animation on canvas
- Design modification mode: select elements, then describe changes
- Multi-provider: Anthropic API, Claude Code, OpenCode

### History & Clipboard

- Undo / Redo with batched operations (Cmd+Z / Cmd+Shift+Z, up to 300 states)
- Copy, Cut, Paste, Duplicate (Cmd+C/X/V/D)
- Group / Ungroup (Cmd+G / Cmd+Shift+G)

### File Operations

- Save / open `.pen` files (JSON-based, Git-friendly)
- Auto-save with File System Access API
- Per-layer export: PNG / SVG at 1x / 2x / 3x (Cmd+Shift+E)

## Keyboard Shortcuts

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
| Cmd+Z / Cmd+Shift+Z | Undo / Redo |
| Cmd+C / X / V / D | Copy / Cut / Paste / Duplicate |
| Cmd+G / Cmd+Shift+G | Group / Ungroup |
| Cmd+S | Save |
| Cmd+Shift+E | Export |
| Cmd+Shift+C | Code panel |
| Cmd+Shift+V | Variables panel |
| Cmd+J | AI chat |
| Cmd+, | Agent settings |
| Delete / Backspace | Delete selected |
| Arrow keys | Nudge 1px (+Shift = 10px) |
| \[ / \] | Reorder layers |
| Escape | Deselect / Cancel |

## Tech Stack

| Layer | Technology |
|---|---|
| **Framework** | [TanStack Start](https://tanstack.com/start) (React 19, SSR, file-based routing) |
| **Canvas** | [Fabric.js](http://fabricjs.com/) v7 |
| **State** | [Zustand](https://zustand-demo.pmnd.rs/) v5 |
| **UI** | [shadcn/ui](https://ui.shadcn.com/) (Radix + Tailwind) |
| **Styling** | [Tailwind CSS](https://tailwindcss.com/) v4 |
| **Icons** | [Lucide React](https://lucide.dev/) |
| **Server** | [Nitro](https://nitro.build/) |
| **Desktop** | [Electron](https://www.electronjs.org/) 35 + [electron-builder](https://www.electron.build/) |
| **AI** | [Anthropic SDK](https://docs.anthropic.com/) + [Claude Agent SDK](https://github.com/anthropics/claude-agent-sdk) |
| **Runtime** | [Bun](https://bun.sh/) |
| **Build** | [Vite](https://vite.dev/) 7 |
| **CI/CD** | GitHub Actions |

## Scripts

| Command | Description |
|---|---|
| `bun --bun run dev` | Start web dev server (port 3000) |
| `bun --bun run build` | Production web build |
| `bun --bun run preview` | Preview production build |
| `bun --bun run test` | Run tests (Vitest) |
| `npx tsc --noEmit` | Type check |
| `bun run electron:dev` | Vite + Electron dev |
| `bun run electron:compile` | Compile electron/ with esbuild |
| `bun run electron:build` | Full Electron package |

## Project Structure

```
src/
  canvas/              # Fabric.js canvas engine (29 files)
  variables/           # Design variables system (resolve, replace refs)
  components/
    editor/            # Editor layout, toolbar, tool buttons, top bar, status bar
    panels/            # Layer, property, AI chat, code, variables, export panels (24 files)
    shared/            # ColorPicker, NumberInput, VariablePicker, dialogs
    icons/             # Provider logos (Claude, OpenAI, OpenCode)
    ui/                # shadcn/ui primitives
  hooks/               # Keyboard shortcuts
  lib/                 # Utility functions (cn class merging)
  services/
    ai/                # AI chat, orchestrator, design generation, prompts (18 files)
    codegen/           # React+Tailwind, HTML+CSS, CSS variables generators
  stores/              # Zustand stores (7 files: canvas, document, history, AI, agent-settings, uikit)
  types/               # PenDocument/PenNode types, styles, variables, agent settings
  utils/               # File operations, export, SVG parser, normalization
  uikit/               # UI kit system (built-in registry, import/export)
  mcp/                 # MCP server integration (tools, document manager)
  routes/              # TanStack Router pages (/, /editor)
electron/
  main.ts              # Electron main process
  preload.ts           # Context bridge for renderer ↔ main IPC
server/
  api/ai/              # Nitro API: streaming chat, generation, agent connection, validation
  utils/               # Claude CLI resolver, OpenCode client, Codex client
```

## CI / CD

### CI (`ci.yml`)

Runs on every push and PR:

1. Type check (`tsc --noEmit`) + unit tests (`vitest`)
2. Production web build, uploads `.output/` as artifact

### Electron Builds (`build-electron.yml`)

Triggered by version tags (`v*`) or manual dispatch:

1. Parallel matrix build: macOS (`.dmg` + `.zip`), Windows (`.exe`), Linux (`.AppImage` + `.deb`)
2. Creates a draft GitHub Release with all platform artifacts

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Contributing

Contributions are welcome! Here's how to get started:

1. **Fork** the repository
2. **Create a branch** for your feature or fix: `git checkout -b feat/my-feature`
3. **Make your changes** — follow the code style in [CLAUDE.md](./CLAUDE.md)
4. **Run checks** before committing:
   ```bash
   npx tsc --noEmit    # Type check
   bun --bun run test   # Tests
   bun --bun run build  # Build
   ```
5. **Commit** with [Conventional Commits](https://www.conventionalcommits.org/) format:
   ```
   feat(canvas): add multi-select rotation support
   ```
6. **Open a Pull Request** against the `v0.0.1` branch

### Code Style

- Files must not exceed **800 lines** — split into focused modules when they grow
- One component per file, single responsibility
- File names in **kebab-case**: `canvas-store.ts`, `use-keyboard-shortcuts.ts`
- Use shadcn/ui design tokens (`bg-card`, `text-foreground`) — no hardcoded Tailwind colors

## Roadmap

- [x] Design variables / tokens with CSS sync
- [x] Component system (reusable components with instances & overrides)
- [x] AI-powered design generation with orchestrator
- [x] MCP server integration
- [ ] Boolean operations (union, subtract, intersect)
- [ ] Multi-page support
- [ ] Collaborative editing
- [ ] Plugin system

## License

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven—W
