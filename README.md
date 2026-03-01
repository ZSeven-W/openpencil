<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Open-source design tool with a Design-as-Code philosophy.</strong><br />
  Design on canvas. Generate code. Let AI build screens from a prompt.
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> ·
  <a href="#features">Features</a> ·
  <a href="#ai-agents">AI Agents</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contributing">Contributing</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Click the image to watch the demo video</sub></p>

<br />

## Quick Start

```bash
# Install dependencies
bun install

# Start dev server at http://localhost:3000
bun --bun run dev
```

Or run as a desktop app:

```bash
bun run electron:dev
```

> **Prerequisites:** [Bun](https://bun.sh/) >= 1.0 and [Node.js](https://nodejs.org/) >= 18

## Features

**Canvas & Drawing**
- Infinite canvas with pan, zoom, smart alignment guides, and snapping
- Rectangle, Ellipse, Line, Polygon, Pen (Bezier), Frame, Text
- Icon picker (Iconify) and image import (PNG/JPEG/SVG/WebP/GIF)
- Auto-layout — vertical/horizontal with gap, padding, justify, align
- Multi-page documents with tab navigation

**Design System**
- Design variables — color, number, string tokens with `$variable` references
- Multi-theme support — multiple axes, each with variants (Light/Dark, Compact/Comfortable)
- Component system — reusable components with instances and overrides
- CSS sync — auto-generated custom properties, `var(--name)` in code output

**Code Generation**
- React + Tailwind CSS
- HTML + CSS
- CSS Variables from design tokens

**Figma Import**
- Import `.fig` files with layout, fills, strokes, effects, text, images, and vectors preserved

**Desktop App**
- Native macOS, Windows, and Linux via Electron
- Auto-update from GitHub Releases
- Native application menu and file dialogs

## AI Agents

OpenPencil integrates with multiple AI coding agents to generate full-page designs from a single prompt — directly on canvas with streaming animation.

| Agent | Setup |
| --- | --- |
| **Anthropic API** | Set `ANTHROPIC_API_KEY` in `.env` |
| **Claude Code** | No config — uses Claude Agent SDK with local OAuth |
| **Codex CLI** | Connect in Agent Settings (`Cmd+,`) |
| **OpenCode** | Connect in Agent Settings (`Cmd+,`) |

- **Text-to-design** — describe a page, get it generated on canvas in real-time
- **Image attachments** — attach screenshots or mockups for vision-based analysis
- **Orchestrator** — decomposes complex pages into spatial sub-tasks for parallel generation
- **Design modification** — select elements, then describe changes in natural language
- **MCP server** — install into Claude Code / Codex / Gemini / OpenCode / Kiro CLIs for design automation from terminal

## Tech Stack

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Canvas** | Fabric.js v7 |
| **State** | Zustand v5 |
| **Server** | Nitro |
| **Desktop** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **Runtime** | Bun · Vite 7 |
| **File format** | `.pen` — JSON-based, human-readable, Git-friendly |

## Project Structure

```text
src/
  canvas/          Fabric.js engine — drawing, sync, layout, guides, pen tool
  components/      React UI — editor, panels, shared dialogs, icons
  services/ai/     AI chat, orchestrator, design generation, streaming
  services/figma/  Figma .fig binary import pipeline
  services/codegen React+Tailwind and HTML+CSS code generators
  stores/          Zustand — canvas, document, pages, history, AI, settings
  variables/       Design token resolution and reference management
  mcp/             MCP server tools for external CLI integration
  uikit/           Reusable component kit system
server/
  api/ai/          Nitro API — streaming chat, generation, validation
  utils/           Claude CLI, OpenCode, Codex client wrappers
electron/
  main.ts          Window, Nitro fork, native menu, auto-updater
  preload.ts       IPC bridge
```

## Keyboard Shortcuts

| Key | Action | | Key | Action |
| --- | --- | --- | --- | --- |
| `V` | Select | | `Cmd+S` | Save |
| `R` | Rectangle | | `Cmd+Z` | Undo |
| `O` | Ellipse | | `Cmd+Shift+Z` | Redo |
| `L` | Line | | `Cmd+C/X/V/D` | Copy/Cut/Paste/Duplicate |
| `T` | Text | | `Cmd+G` | Group |
| `F` | Frame | | `Cmd+Shift+G` | Ungroup |
| `P` | Pen tool | | `Cmd+Shift+E` | Export |
| `H` | Hand (pan) | | `Cmd+Shift+C` | Code panel |
| `Del` | Delete | | `Cmd+Shift+V` | Variables panel |
| `[ / ]` | Reorder | | `Cmd+J` | AI chat |
| Arrows | Nudge 1px | | `Cmd+,` | Agent settings |

## Scripts

```bash
bun --bun run dev          # Dev server (port 3000)
bun --bun run build        # Production build
bun --bun run test         # Run tests (Vitest)
npx tsc --noEmit           # Type check
bun run electron:dev       # Electron dev
bun run electron:build     # Electron package
```

## Contributing

Contributions are welcome! See [CLAUDE.md](./CLAUDE.md) for architecture details and code style.

1. Fork and clone
2. Create a branch: `git checkout -b feat/my-feature`
3. Run checks: `npx tsc --noEmit && bun --bun run test`
4. Commit with [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
5. Open a PR against `main`

## Roadmap

- [x] Design variables & tokens with CSS sync
- [x] Component system (instances & overrides)
- [x] AI design generation with orchestrator
- [x] MCP server integration
- [x] Multi-page support
- [x] Figma `.fig` import
- [ ] Boolean operations (union, subtract, intersect)
- [ ] Collaborative editing
- [ ] Plugin system

## Contributors

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Community

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Join our Discord</strong>
</a>
— Ask questions, share designs, suggest features.

## License

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
