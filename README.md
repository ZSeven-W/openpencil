<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>The world's first open-source AI-native vector design tool.</strong><br />
  <sub>Concurrent Agent Teams &bull; Design-as-Code &bull; Built-in MCP Server &bull; Multi-model Intelligence</sub>
</p>

<p align="center">
  <a href="./README.md"><b>English</b></a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Click the image to watch the demo video</sub></p>

<br />

## Why OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Canvas

Describe any UI in natural language. Watch it appear on the infinite canvas in real-time with streaming animation. Modify existing designs by selecting elements and chatting.

</td>
<td width="50%">

### 🤖 Concurrent Agent Teams

The orchestrator decomposes complex pages into spatial sub-tasks. Multiple AI agents work on different sections simultaneously — hero, features, footer — all streaming in parallel.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Multi-Model Intelligence

Automatically adapts to each model's capabilities. Claude gets full prompts with thinking; GPT-4o/Gemini disable thinking; smaller models (MiniMax, Qwen, Llama) get simplified prompts for reliable output.

</td>
<td width="50%">

### 🔌 MCP Server

One-click install into Claude Code, Codex, Gemini, OpenCode, Kiro, or Copilot CLIs. Design from your terminal — read, create, and modify `.op` files through any MCP-compatible agent.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

`.op` files are JSON — human-readable, Git-friendly, diffable. Design variables generate CSS custom properties. Code export to React + Tailwind or HTML + CSS.

</td>
<td width="50%">

### 🖥️ Runs Everywhere

Web app + native desktop on macOS, Windows, and Linux via Electron. Auto-updates from GitHub Releases. `.op` file association — double-click to open.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Control the design tool from your terminal. `op design`, `op insert`, `op export` — batch design DSL, node manipulation, code export. Pipe in from files or stdin. Works with desktop app or web server.

</td>
<td width="50%">

### 🎯 Multi-Platform Code Export

Export to React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native — all from one `.op` file. Design variables become CSS custom properties.

</td>
</tr>
</table>

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

### Docker

Multiple image variants are available — pick the one that fits your needs:

| Image | Size | Includes |
| --- | --- | --- |
| `openpencil:latest` | ~226 MB | Web app only |
| `openpencil-claude:latest` | — | + Claude Code CLI |
| `openpencil-codex:latest` | — | + Codex CLI |
| `openpencil-opencode:latest` | — | + OpenCode CLI |
| `openpencil-copilot:latest` | — | + GitHub Copilot CLI |
| `openpencil-gemini:latest` | — | + Gemini CLI |
| `openpencil-full:latest` | ~1 GB | All CLI tools |

**Run (web only):**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**Run with AI CLI (e.g. Claude Code):**

The AI chat relies on Claude CLI OAuth login. Use a Docker volume to persist the login session:

```bash
# Step 1 — Login (one-time)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# Step 2 — Start
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**Build locally:**

```bash
# Base (web only)
docker build --target base -t openpencil .

# With a specific CLI
docker build --target with-claude -t openpencil-claude .

# Full (all CLIs)
docker build --target full -t openpencil-full .
```

## AI-Native Design

**Prompt to UI**
- **Text-to-design** — describe a page, get it generated on canvas in real-time with streaming animation
- **Orchestrator** — decomposes complex pages into spatial sub-tasks for parallel generation
- **Design modification** — select elements, then describe changes in natural language
- **Vision input** — attach screenshots or mockups for reference-based design

**Multi-Agent Support**

| Agent | Setup |
| --- | --- |
| **Claude Code** | No config — uses Claude Agent SDK with local OAuth |
| **Codex CLI** | Connect in Agent Settings (`Cmd+,`) |
| **OpenCode** | Connect in Agent Settings (`Cmd+,`) |
| **GitHub Copilot** | `copilot login` then connect in Agent Settings (`Cmd+,`) |
| **Gemini CLI** | Connect in Agent Settings (`Cmd+,`) |

**Model Capability Profiles** — automatically adapts prompts, thinking mode, and timeouts per model tier. Full-tier models (Claude) get complete prompts; standard-tier (GPT-4o, Gemini, DeepSeek) disable thinking; basic-tier (MiniMax, Qwen, Llama, Mistral) get simplified nested-JSON prompts for maximum reliability.

**MCP Server**
- Built-in MCP server — one-click install into Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs
- Auto-detects Node.js — if not installed, falls back to HTTP transport and auto-starts the MCP HTTP server
- Design automation from terminal: read, create, and modify `.op` files via any MCP-compatible agent
- **Layered design workflow** — `design_skeleton` → `design_content` → `design_refine` for higher-fidelity multi-section designs
- **Segmented prompt retrieval** — load only the design knowledge you need (schema, layout, roles, icons, planning, etc.)
- Multi-page support — create, rename, reorder, and duplicate pages via MCP tools

**Code Generation**
- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Install globally and control the design tool from your terminal:

```bash
npm install -g @zseven-w/openpencil
```

```bash
op start                     # Launch desktop app
op design @landing.txt       # Batch design from file
op insert '{"type":"RECT"}'  # Insert a node
op export react --out .      # Export to React + Tailwind
op import:figma design.fig   # Import Figma file
cat design.dsl | op design - # Pipe from stdin
```

Supports three input methods: inline string, `@filepath` (read from file), or `-` (read from stdin). Works with desktop app or web dev server. See [CLI README](./apps/cli/README.md) for full command reference.

## Features

**Canvas & Drawing**
- Infinite canvas with pan, zoom, smart alignment guides, and snapping
- Rectangle, Ellipse, Line, Polygon, Pen (Bezier), Frame, Text
- Boolean operations — union, subtract, intersect with contextual toolbar
- Icon picker (Iconify) and image import (PNG/JPEG/SVG/WebP/GIF)
- Auto-layout — vertical/horizontal with gap, padding, justify, align
- Multi-page documents with tab navigation

**Design System**
- Design variables — color, number, string tokens with `$variable` references
- Multi-theme support — multiple axes, each with variants (Light/Dark, Compact/Comfortable)
- Component system — reusable components with instances and overrides
- CSS sync — auto-generated custom properties, `var(--name)` in code output

**Figma Import**
- Import `.fig` files with layout, fills, strokes, effects, text, images, and vectors preserved

**Desktop App**
- Native macOS, Windows, and Linux via Electron
- `.op` file association — double-click to open, single-instance lock
- Auto-update from GitHub Releases
- Native application menu and file dialogs

## Tech Stack

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Canvas** | CanvasKit/Skia (WASM, GPU-accelerated) |
| **State** | Zustand v5 |
| **Server** | Nitro |
| **Desktop** | Electron 35 |
| **CLI** | `op` — terminal control, batch design DSL, code export |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Runtime** | Bun · Vite 7 |
| **File format** | `.op` — JSON-based, human-readable, Git-friendly |

## Project Structure

```text
openpencil/
├── apps/
│   ├── web/                 TanStack Start web app
│   │   ├── src/
│   │   │   ├── canvas/      CanvasKit/Skia engine — drawing, sync, layout
│   │   │   ├── components/  React UI — editor, panels, shared dialogs, icons
│   │   │   ├── services/ai/ AI chat, orchestrator, design generation, streaming
│   │   │   ├── stores/      Zustand — canvas, document, pages, history, AI
│   │   │   ├── mcp/         MCP server tools for external CLI integration
│   │   │   ├── hooks/       Keyboard shortcuts, file drop, Figma paste
│   │   │   └── uikit/       Reusable component kit system
│   │   └── server/
│   │       ├── api/ai/      Nitro API — streaming chat, generation, validation
│   │       └── utils/       Claude CLI, OpenCode, Codex, Copilot wrappers
│   ├── desktop/             Electron desktop app
│   │   ├── main.ts          Window, Nitro fork, native menu, auto-updater
│   │   ├── ipc-handlers.ts  Native file dialogs, theme sync, prefs IPC
│   │   └── preload.ts       IPC bridge
│   └── cli/                 CLI tool — `op` command
│       ├── src/commands/    Design, document, export, import, node, page, variable commands
│       ├── connection.ts    WebSocket connection to running app
│       └── launcher.ts      Auto-detect and launch desktop app or web server
├── packages/
│   ├── pen-types/           Type definitions for PenDocument model
│   ├── pen-core/            Document tree ops, layout engine, variables
│   ├── pen-codegen/         Code generators (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Figma .fig file parser and converter
│   ├── pen-renderer/        Standalone CanvasKit/Skia renderer
│   └── pen-sdk/             Umbrella SDK (re-exports all packages)
└── .githooks/               Pre-commit version sync from branch name
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
| `Cmd+Alt+U` | Boolean union | | `Cmd+Alt+S` | Boolean subtract |
| `Cmd+Alt+I` | Boolean intersect | | | |

## Scripts

```bash
bun --bun run dev          # Dev server (port 3000)
bun --bun run build        # Production build
bun --bun run test         # Run tests (Vitest)
npx tsc --noEmit           # Type check
bun run bump <version>     # Sync version across all package.json
bun run electron:dev       # Electron dev
bun run electron:build     # Electron package
bun run cli:dev            # Run CLI from source
bun run cli:compile        # Compile CLI to dist
```

## Contributing

Contributions are welcome! See [CLAUDE.md](./CLAUDE.md) for architecture details and code style.

1. Fork and clone
2. Set up version sync: `git config core.hooksPath .githooks`
3. Create a branch: `git checkout -b feat/my-feature`
4. Run checks: `npx tsc --noEmit && bun --bun run test`
5. Commit with [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
6. Open a PR against `main`

## Roadmap

- [x] Design variables & tokens with CSS sync
- [x] Component system (instances & overrides)
- [x] AI design generation with orchestrator
- [x] MCP server integration with layered design workflow
- [x] Multi-page support
- [x] Figma `.fig` import
- [x] Boolean operations (union, subtract, intersect)
- [x] Multi-model capability profiles
- [x] Monorepo restructure with reusable packages
- [x] CLI tool (`op`) for terminal control
- [ ] Collaborative editing
- [ ] Plugin system

## Contributors

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Community

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Join our Discord</strong>
</a>
— Ask questions, share designs, suggest features.

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## License

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
