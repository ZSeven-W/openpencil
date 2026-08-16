<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Click the image to watch the demo video</sub></p>

<br />

<p align="center">
  <a href="https://github.com/InnerAura/hyperweave" title="Star history rendered with HyperWeave">
    <img src="./screenshot/openpencil-star-history.svg" alt="OpenPencil star history rendered by HyperWeave" width="100%" />
  </a>
</p>
<p align="center">
  <sub>The trend above is rendered with <a href="https://github.com/InnerAura/hyperweave"><b>HyperWeave</b></a> — an experimental SVG artifact layer for agents. &nbsp;<a href="https://github.com/InnerAura/hyperweave"><b>Explore&nbsp;→</b></a></sub>
</p>

## Why OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Canvas

Describe any UI in natural language. Watch it appear on the infinite canvas in real-time with streaming animation. Modify existing designs by selecting elements and chatting.

</td>
<td width="50%">

### 🤖 Concurrent Agent Teams

The orchestrator decomposes complex pages into spatial sub-tasks. Multiple AI agents work on different sections simultaneously — hero, features, footer — all streaming in parallel with per-member canvas indicators.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Multi-Model Intelligence

Automatically adapts to each model's capabilities. Claude gets full prompts with thinking; GPT-4o/Gemini disable thinking; smaller models (MiniMax, Qwen, Llama) get simplified prompts for reliable output.

</td>
<td width="50%">

### 🔌 MCP Server

One-click install into Claude Code, Codex, OpenCode, Kiro, or Copilot CLIs. Design from your terminal — read, create, and modify `.op` files through any MCP-compatible agent.

</td>
</tr>
<tr>
<td width="50%">

### 🎨 Style Guides

Built-in style guide library with tag-based fuzzy matching. Apply visual styles (glassmorphism, brutalist, retro, etc.) to AI-generated designs. MCP tools for external agent access.

</td>
<td width="50%">

### 📦 Design-as-Code

`.op` files are JSON — human-readable, Git-friendly, diffable. Design variables generate CSS custom properties. Code export to React + Tailwind or HTML + CSS.

</td>
</tr>
<tr>
<td width="50%">

### 🖥️ Runs Everywhere

Web app + native desktop on macOS, Windows, and Linux — one Rust core, a single self-contained binary, no browser engine. `.op` file association — double-click to open.

</td>
<td width="50%">

### ⌨️ CLI — `op`

Control the design tool from your terminal. `op design`, `op insert` — batch design DSL, node manipulation. Pipe in from files or stdin. Works with desktop app or web server.

</td>
</tr>
<tr>
<td width="50%">

### 🎯 Multi-Platform Code Export

Export to React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native — all from one `.op` file. Design variables become CSS custom properties.

</td>
<td width="50%">

### 🧩 Embeddable SDK

`op-web-sdk` (vanilla) + `op-web-sdk-react` / `op-web-sdk-vue` adapters — embed a read-only `.op` viewer in your own app, powered by the same wasm renderer the editor ships.

</td>
</tr>
<tr>
<td width="50%">

### 🛡️ Design System Kit

Manage reusable UIKits with style switching and component composition. Import/export kits from `.pen` files. Built-in registry with MCP tools for external access.

</td>
<td width="50%">

### 👥 Real-Time Collaboration

Design together over authenticated peer-to-peer sessions with a public relay and built-in regional hubs. Join with a short pairing code, see live remote cursors, and resolve conflicts with per-edit detail and replay.

</td>
</tr>
</table>

## Install

**macOS (Homebrew):**

```bash
brew tap zseven-w/openpencil
brew install --cask openpencil
```

**Windows (Scoop):**

```powershell
scoop bucket add openpencil https://github.com/zseven-w/scoop-openpencil
scoop install openpencil
```

**Linux / Windows direct download:** [GitHub Releases](https://github.com/ZSeven-W/openpencil/releases) — `.exe` (Windows), `.AppImage` / `.deb` (Linux)

> Antivirus flagged the Windows installer? That's a known heuristic false positive on new, low-prevalence releases — see [how to verify your download](docs/security/antivirus-false-positives.md).

**Nix (Linux x86_64):**

```bash
nix develop
nix run .                         # launch the desktop app
nix build .#openpencil            # native web host + CanvasKit web bundle
nix build .#op-cli                # the `op` CLI
nix build .#prebuilt              # use the matching upstream desktop archive
nix build .#prebuilt-cli          # use the matching upstream CLI archive
nix build .#web-server            # native GL-free web server + web bundle
nix build .#runtime-prebuilt      # prebuilt desktop + `op` CLI runtime
nix build .#web-sdk-packages      # npm tarballs for the web SDKs
nix build .#appimage              # portable desktop AppImage
```

The flake uses the pinned Rust toolchain from `rust-toolchain.toml` and is
currently published for `x86_64-linux`. A Debian package is not produced by
the flake yet; use the upstream release artifacts when a `.deb` is required.
The `prebuilt` outputs use the release version and hashes pinned in
`nix/release-manifest.json`, independently of the workspace source version.
After a release is published, the release workflow opens a PR to refresh that
manifest. Until it is merged, the prebuilt outputs continue to use the prior
published release; the source-built outputs always build the checked-out source.

**CLI (`op`):**

```bash
brew install zseven-w/openpencil/op
```

Or use the install script (macOS / Linux):

```bash
curl -fsSL https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.sh | bash
```

To allow the newest pre-release:

```bash
curl -fsSL https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.sh | OP_PRERELEASE=1 bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.ps1 | iex
```

To allow the newest pre-release:

```powershell
$env:OP_PRERELEASE = "1"; irm https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.ps1 | iex
```

## Cloning (with submodules)

```bash
git clone --recurse-submodules https://github.com/ZSeven-W/openpencil.git
# Or already cloned — sync first so stale submodule URLs pick up .gitmodules changes:
git submodule sync --recursive && git submodule update --init --recursive
```

Three submodules live under `vendor/`, all public and fetched over HTTPS (no SSH key needed): `jian` (GPU-Skia UI framework — widgets/render/events), `casement` (winit fork), and `agent` (`agent-rs` — cross-product Rust agent runtime, shared by OP + Zode). `vendor/anthropic-agent-sdk` is tracked in-tree, not a submodule.

## Quick Start (Development)

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

Browser-entered built-in provider, image-generation, and Openverse credentials
are kept in same-origin `localStorage`
(`openpencil-rust-web-credentials`) by default,
separate from ordinary web preferences. A credential needed by an AI request is
sent with that request, but the daemon does not write the browser snapshot to
`settings.json`.

Private deployments can opt into server persistence with
`OPENPENCIL_PERSIST_WEB_CREDENTIALS_SERVER=true`. This setting is
deployment-wide; each accepted browser snapshot replaces the shared
browser-owned copy. Enable it only for a trusted, access-controlled deployment
with one active administrator profile, behind HTTPS. Turning the setting off
removes previously persisted browser-owned credentials at the next startup
while preserving operator-owned configuration. As with all `localStorage`,
same-origin scripts can read these values, so the web origin must be protected
from untrusted scripts and XSS. Before downgrading to a release that predates
this switch, turn it off and start the current release once to scrub the
browser-owned copy.

The web host supports built-in API-key agents only. CLI and ACP agents remain
native-only: they are not shown in the web model picker, stored in browser
credential snapshots, synchronized to the server, or exposed through web
connect routes. Browser requests to credential and AI endpoints allow
loopback/localhost origins by default. A public reverse proxy must separately set
`OPENPENCIL_WEB_ALLOWED_ORIGINS` to a comma-separated list of exact origins,
for example `https://demo.openpencil.dev`.

Server persistence does not permit browser-controlled requests to loopback,
private, link-local, metadata, or reserved provider endpoints. A trusted private
deployment that intentionally uses an internal provider must separately set
`OPENPENCIL_WEB_AI_ENDPOINT_ALLOWLIST` to a comma-separated list of exact URL
origins, for example `http://127.0.0.1:11434,https://llm.internal.example`.

Or run as a desktop app:

```bash
cargo run -p op-host-desktop
```

> **Prerequisites:** [Rust](https://www.rust-lang.org/) (stable) to build the product. [Bun](https://bun.sh/) >= 1.0 and [Node.js](https://nodejs.org/) >= 18 are only needed for the web SDK under `packages/`.

### Docker

Tagged Rust releases publish a single web-host image. The retired TypeScript images with bundled AI CLIs are no longer published.

| Image | Includes |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:vX.Y.Z` | Rust web host, wasm bundle, and CanvasKit assets |

The web UI exposes built-in agent profiles only; Claude/Codex/OpenCode/Copilot CLI tooling is not bundled into Docker images.

**Run:**

```bash
VERSION="$(scripts/workspace-version.sh)"
docker run -d -p 3100:3100 "ghcr.io/zseven-w/openpencil-web:v${VERSION}"
```

Then open `http://localhost:3100/`.

**Build locally:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## AI-Native Design

**Prompt to UI**

- **Text-to-design** — describe a page, get it generated on canvas in real-time with SSE streaming animation
- **Orchestrator** — decomposes complex pages into spatial sub-tasks for parallel generation
- **Agent Teams** — concurrent team members with delegate tool, per-member canvas indicators, and fallback strategies
- **Design modification** — select elements, then describe changes in natural language
- **Vision input** — attach screenshots or mockups for reference-based design
- **Style Guides** — apply visual styles (glassmorphism, brutalist, retro, etc.) via tag-based fuzzy matching
- **Anti-slop** — cross-generation diversity tracking to avoid repetitive AI output

**Multi-Agent Support**

| Agent                       | Setup                                                                                             |
| --------------------------- | ------------------------------------------------------------------------------------------------- |
| **Built-in (9+ providers)** | Select from provider presets with region switcher — Anthropic, OpenAI, Google, DeepSeek, and more |
| **Claude Code**             | No config — uses Claude Agent SDK with local OAuth                                                |
| **Codex CLI**               | Connect in Agent Settings (`Cmd+,`)                                                               |
| **OpenCode**                | Connect in Agent Settings (`Cmd+,`)                                                               |
| **GitHub Copilot**          | `copilot login` then connect in Agent Settings (`Cmd+,`)                                          |

**Model Capability Profiles** — automatically adapts prompts, thinking mode, and timeouts per model tier. Full-tier models (Claude) get complete prompts; standard-tier (GPT-4o, Gemini, DeepSeek) disable thinking; basic-tier (MiniMax, Qwen, Llama, Mistral) get simplified nested-JSON prompts for maximum reliability.

**i18n** — Full interface localization in 15 languages: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**MCP Server**

- Built-in MCP server (`op-mcp` crate) — one-click install into Claude Code / Codex / OpenCode / Kiro / Copilot CLIs
- No Node.js required — stdio transport via the desktop binary (`--mcp <path>`), plus a live HTTP endpoint (`127.0.0.1:<port>/mcp`) from the running app
- Design automation from terminal: read, create, and modify `.op` files via any MCP-compatible agent
- **Layered design workflow** — `design_skeleton` → `design_content` → `design_refine` for higher-fidelity multi-section designs
- **Segmented prompt retrieval** — load only the design knowledge you need (schema, layout, roles, icons, planning, etc.)
- **Style guide tools** — `get_style_guide_tags` and `get_style_guide` for applying visual styles via MCP
- Multi-page support — create, rename, reorder, and duplicate pages via MCP tools

**Code Generation**

- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Install via Homebrew or the install script, then control the design tool from your terminal:

```bash
brew install zseven-w/openpencil/op
```

```bash
curl -fsSL https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.sh | bash
```

```bash
curl -fsSL https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.sh | OP_PRERELEASE=1 bash
```

```powershell
irm https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.ps1 | iex
```

```powershell
$env:OP_PRERELEASE = "1"; irm https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.ps1 | iex
```

```bash
op start                     # Launch desktop app
op start --headless --file design.op # Launch headless file-backed server
op design @landing.txt       # Batch design from file
op design @ui.js             # Sandboxed JavaScript with loops
op insert '{"type":"rectangle"}' # Insert a node
op import:figma design.fig   # Import Figma file
cat design.dsl | op design - # Pipe from stdin
```

Supports three input methods: inline string, `@filepath` (read from file), or `-` (read from stdin). Works with the desktop app, web server, or headless file-backed server. See the [CLI command reference](./crates/op-cli/src/usage.txt) for all commands.

**LLM Skill** — install the [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) plugin to teach AI agents how to design with `op`. Run `op install` for detected agents, or target one explicitly with `op install --target codex`.

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
- Reusable UIKits — import/export component kits from `.pen` files

**AI & Agents**

- Prompt-to-canvas with streaming generation and orchestrator-driven spatial decomposition
- Concurrent Agent Teams — multiple designers work on different sections in parallel with per-member canvas indicators
- Layered workflow — `design_skeleton` → `design_content` → `design_refine` with focused prompts per phase
- Style Guides — 50+ built-in styles (glassmorphism, brutalist, retro, etc.) with tag-based fuzzy matching, wired into planning and generation
- Multi-model capability profiles — auto-adapts thinking mode, effort, and prompt shape per model tier
- Built-in Rust agent runtime + Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Google Gemini API providers
- Anthropic-format passthrough for Chinese LLM providers — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Collaboration**

- Authenticated peer-to-peer sessions with a public relay fallback and built-in regional collaboration hubs
- Join with a 10-character region-tagged pairing code — no account required for LAN sessions
- Live remote cursors, cross-account collaboration, and a conflict panel with per-edit detail and replay of discarded edits
- Online multi-tenant mode for the `--serve-web` daemon, authenticated against the op-hub with cross-account tenant sharing
- Device login — sign in from the browser through the serve-web daemon, with profile avatars and usernames in the editor

**Presentation Decks**

- Six 16:9 deck templates with a template picker, plus AI deck planning at projector size — one slide per screen
- Present a deck as a slideshow with presenter controls
- Export a deck as PDF (one page per slide), a self-contained slideshow HTML file, an editable PowerPoint (`.pptx`), or a hyperframes video composition
- Slides rail navigator; the agent validates deck board geometry (aspect, overflow, centering) as well as the prompt

**Templates & Web Capture**

- Scene template center — a browsable catalogue of 58 templates across six scenes, opened from File ▸ New from template
- Prompt center with web, dashboard, component, and modify entries and visual prompt previews
- Asset center — a full-window responsive gallery with dual-action templates and DESIGN.md style import
- [Chrome web-capture extension](https://chromewebstore.google.com/detail/openpencil/ahfofgelkdaobcmglejmjbpnajhpgcij) — snapshot any webpage into OpenPencil with faithful HTML / layout capture; snapshots are delivered to your account's inbox on the hub, or downloaded as a ready-to-open `.op` file offline

**Git Integration**

- Clone wizard with SSH / HTTPS auth and SSH key management
- Branch picker — create, switch, delete, merge, all from the git panel
- Pull / push cascades with auth retry and non-fast-forward handling
- Folder-mode three-way merge with on-disk `MERGE_HEAD` state tracking
- Conflict panel with per-node / per-field three-way cards, inline JSON editor, bulk actions, and inline diff block
- Remote settings and SSH keys UI; 15-locale i18n across the whole Git surface

**Export**

- Canvas export — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- Code export — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Incremental MCP codegen pipeline — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Figma Import**

- Import `.fig` files with layout, fills, strokes, effects, text, images, and vectors preserved

**Desktop App**

- Native macOS, Windows, and Linux — a single self-contained binary (winit + GPU Skia, no Electron)
- `.op` file association — double-click to open, single-instance lock
- Background update check against GitHub Releases
- Native application menu with Save As, Open Recent, and an unsaved-changes dialog on close
- Recent files persistence

## Tech Stack

|                 |                                                                                            |
| --------------- | ------------------------------------------------------------------------------------------ |
| **Core**        | Rust workspace (`crates/`) — editor state, widgets, hosts, MCP, AI, codegen                 |
| **Rendering**   | GPU Skia everywhere — `skia-safe` (GL) on native, CanvasKit (WASM/WebGL2) in the browser    |
| **UI framework** | jian — vendored pure-Rust GPU-Skia UI framework: widgets, layout, events, hot reload (`vendor/jian`) |
| **Windowing**   | winit (vendored `casement` fork)                                                            |
| **Desktop**     | Native binary `openpencil-desktop` — no browser engine                                      |
| **Web SDK**     | `op-web-sdk` + React 19 / Vue 3 adapters — read-only `.op` viewer (TypeScript)              |
| **CLI**         | `op` — terminal control, batch design DSL                                                   |
| **AI**          | Built-in Rust agent runtime · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**        | clippy · rustfmt (Rust) · oxlint · oxfmt (web SDK)                                          |
| **File format** | `.op` — JSON-based, human-readable, Git-friendly                                            |

## Ecosystem

OpenPencil is part of a family of pure-Rust, AI-native tools from **[ZSeven-W](https://github.com/ZSeven-W)**. They compose: `jian` renders OpenPencil, `agent-rs` runs its agents, `noema` remembers, and `zode` designs from the terminal.

| Project | What it is |
| ------- | ---------- |
| <img src="./screenshot/logo-dsh-openpencil.png" alt="DSH OpenPencil" width="40" /> **[DSH OpenPencil](https://github.com/ZSeven-W/dsh-openpencil)** | DeepSeek Harness plugin for OpenPencil — exact multi-frame `.op` previews, an interactive canvas, and a managed editor with agent-native design tools, right inside a conversation. |
| <img src="./screenshot/logo-zode.png" alt="Zode logo" width="40" /> **[Zode](https://github.com/ZSeven-W/zode)** | Open-source, AI-native coding assistant for your terminal — a fast Rust TUI (`ratatui`) that reads your code, runs commands, searches files, and manages git. Drives OpenPencil over MCP. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | A pure-Rust async runtime for shipping LLM agents — multi-provider, tool-capable end-to-end, structured permissions, real MCP, zero `unsafe`. Powers OpenPencil's built-in agent runtime (`vendor/agent`) and Zode. |
| <img src="./screenshot/logo-jian-rounded.png" alt="jian logo" width="40" /> **[jian](https://github.com/ZSeven-W/jian)** | Pure-Rust, GPU-Skia UI framework — widgets, layout, events, and hot reload in one stack. Turns a declarative `.op` document into a native, AI-controllable app with no JS runtime, no DOM, no Electron. OpenPencil's UI framework (`vendor/jian`). |
| <img src="./screenshot/logo-noema-rounded.png" alt="noema logo" width="84" /> **[noema](https://github.com/ZSeven-W/noema)** | Local-first, non-vector memory system for coding agents. Durable memory as inspectable files, a review queue for new entries, and lexical (embedding-free) recall — works across Zode, Codex, Claude Code, and MCP runtimes. |

## Why Rust

OpenPencil was rewritten from the ground up in **Rust** ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). The rewrite is complete — the TypeScript + Electron editor was retired at `v0.7.5`, and the Rust workspace in this repo is the product: one native core that is dramatically smaller and faster, and runs on more platforms from a single codebase.

|                       | TypeScript + Electron (retired, `v0.7.5`)      | Rust (today)                                                        |
| --------------------- | ---------------------------------------------- | ------------------------------------------------------------------- |
| **Desktop runtime**   | Electron — bundles Chromium + Node.js          | Native window (`winit` + GPU Skia), no browser engine               |
| **Desktop footprint** | Full Chromium runtime per install              | Single self-contained binary — **55.5 MB**                          |
| **Web payload**       | JS + WASM bundle                               | **8.2 MB** wasm / **2.18 MB** gzip over the wire                    |
| **Rendering**         | CanvasKit/Skia on web                          | One GPU-accelerated Skia backend on **every** target                |
| **Memory**            | JavaScript GC pauses                           | No GC — Rust ownership, predictable latency                         |
| **Codebase**          | Web stack + Electron                           | One Rust workspace: editor · CLI · MCP · AI · codegen · Figma · Git  |
| **Targets**           | Web + desktop, two separate stacks             | Desktop (macOS/Win/Linux) · mobile (iOS/Android) · browser — one core |

**Measured improvements**

- **Tiny footprint** — the whole desktop app is one **55.5 MB** native binary instead of a bundled browser engine plus a Node runtime. The web build is **8.2 MB** raw / **2.18 MB** gzip after splitting the icon catalog (−48% over the wire).
- **Scales to large documents** — a **10,000-node** live canvas (nested auto-layout, four levels deep) writes, reads, and snapshots layout with **no panics and ~0% idle CPU**; a full layout snapshot of all 10k nodes returns in **~0.68 s**.
- **Fast interaction** — pan/zoom no longer re-serializes the document every frame (a single hot-path fix cut wheel-zoom CPU from **~69% to ~0%**); drags patch the scene incrementally, text measurement is cached, and repaints coalesce to one per frame.
- **One core, every screen** — the same editor state and the same render backend compile to native desktop, mobile, and the browser via WASM — no parallel reimplementations to keep in sync.
- **GPU Skia everywhere** — native renders through `skia-safe` on a GL context; the browser renders through CanvasKit on WebGL2 — the same drawing code, the same output.
- **Native accessibility** — AccessKit on macOS, Windows, and Linux, plus a DOM mirror on web, instead of leaning on a browser's a11y tree.
- **One type-checked workspace** — the MCP host, CLI, AI providers, code generation, Figma import, and Git integration all live in a single Rust workspace, with `cargo-deny` supply-chain gating in CI.

> **Status:** the TypeScript editor was retired at `v0.7.5` and lives only in git history; this repository is the Rust workspace. The Rust product is under active development (see the Roadmap below).

## Project Structure

```text
openpencil/
├── crates/                   Rust workspace — the product
│   ├── op-editor-core/       Canonical `.op` (PenDocument) editor state + EditorCommand + design variables
│   ├── op-editor-ui/         Platform-free widgets + RenderBackend facade (wasm32-clean)
│   ├── op-editor-host-core/  Transport-free host state machines shared by all hosts
│   ├── op-collab/            Open collaboration wire types, canonical hashes, and exact document apply
│   ├── op-collab-transport/  Native Noise/TCP transport, bounded admission, and optional LAN discovery
│   ├── op-collab-smoke/      Two-process authenticated collaboration recovery matrix
│   ├── op-host-native/       Native host lib — winit + skia-safe GL (desktop + mobile)
│   ├── op-engine-ffi/        Player C ABI — embed the canvas in iOS / Android shells
│   ├── op-engine-jni/        Android JNI marshalling layer for op-engine-ffi
│   ├── op-host-web/          Browser bundle — wasm32 cdylib, CanvasKit renderer
│   ├── op-host-desktop/      Desktop binary `openpencil-desktop`; also the `--serve-web` daemon
│   ├── op-host-services/     Headless serve-web / MCP daemon lib
│   ├── op-host-web-server/   Thin GL-free web-server binary
│   ├── op-cli/               CLI tool — `op` command
│   ├── op-mcp/               MCP server — tools, batch design, layered workflow
│   ├── op-ai/                AI providers, chat runtime, streaming
│   ├── op-ai-skills/         AI prompt skill engine (phase-driven prompt loading)
│   ├── op-orchestrator/      Concurrent agent-team orchestration
│   ├── op-codegen/           Code generators (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Figma .fig file parser and converter
│   ├── op-git/               Git integration — clone, branch, push/pull, merge
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Web SDK workspace (Bun)
│   ├── op-web-sdk/           Read-only `.op` web viewer SDK (wraps the wasm bundle)
│   ├── op-web-sdk-react/     React 19 adapter
│   └── op-web-sdk-vue/       Vue 3 adapter
├── vendor/                   Vendored subsystems (git submodules)
│   ├── jian/                 GPU-Skia UI framework — widgets/render/events
│   ├── casement/             winit fork
│   └── agent/                Cross-product Rust agent runtime (agent-rs)
└── .githooks/                Pre-commit version drift check
```

## Keyboard Shortcuts

| Key         | Action            |     | Key           | Action                    |
| ----------- | ----------------- | --- | ------------- | ------------------------- |
| `V`         | Select            |     | `Cmd+S`       | Save                      |
| `R`         | Rectangle         |     | `Cmd+Z`       | Undo                      |
| `O`         | Ellipse           |     | `Cmd+Shift+Z` | Redo                      |
| `L`         | Line              |     | `Cmd+C/X/V/D` | Copy/Cut/Paste/Duplicate  |
| `T`         | Text              |     | `Cmd+G`       | Group                     |
| `F`         | Frame             |     | `Cmd+Shift+G` | Ungroup                   |
| `P`         | Pen tool          |     | `Cmd+Shift+P` | Export (PNG/JPG/WEBP/PDF) |
| `H`         | Hand (pan)        |     | `Cmd+Shift+C` | Code panel                |
| `Del`       | Delete            |     | `Cmd+Shift+V` | Variables panel           |
| `[ / ]`     | Reorder           |     | `Cmd+J`       | AI chat                   |
| Arrows      | Nudge 1px         |     | `Cmd+,`       | Agent settings            |
| `Cmd+Alt+U` | Boolean union     |     | `Cmd+Alt+S`   | Boolean subtract          |
| `Cmd+Alt+I` | Boolean intersect |     | `Cmd+Shift+S` | Save As                   |

## Scripts

```bash
# Product (Rust — run from the repo root)
cargo build --workspace              # Build all crates (add --release for prod)
cargo test --workspace               # Run all tests
cargo check --workspace              # Type check
cargo clippy --workspace --all-targets -- -D warnings   # Lint
cargo fmt --all                      # Format
bash scripts/start-web-rust.sh       # Web dev server (wasm bundle + headless host)
cargo run -p op-host-desktop         # Desktop app (binary: openpencil-desktop)
cargo run -p op-cli -- <args>        # CLI (binary: op)

# Web SDK / JS tooling (run from packages/)
cd packages && bun run lint          # Lint the web SDK (oxlint); also: bun run format
cd packages && bun run generate-iconify-catalog   # Regenerate the Rust icon catalog assets

# Version synchronization (run from repository root)
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

### Rust workspace details

The Rust-ification is complete — the Rust workspace is the product. Kickoff spec: `openpencil-docs/superpowers/specs/2026-05-02-rust-ification-kickoff.md` (v7 FROZEN).

```bash
# Install Rust toolchain (rust-toolchain.toml auto-pins 1.94)
rustup toolchain install 1.94
rustup target add wasm32-unknown-unknown

# build / test / lint
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check --target wasm32-unknown-unknown -p op-collab --no-default-features
cargo check --target wasm32-unknown-unknown -p op-host-web --no-default-features --features web  # kickoff §1.2 wasm32 invariant
cargo deny check && cargo deny --target wasm32-unknown-unknown check bans        # cargo-deny (native + wasm32 bans; CI uses cargo-deny-action@v2)
```

**Entry crates (`crates/` — build targets):**

| Crate           | Category                                                                          | wasm32           |
| --------------- | -------------------------------------------------------------------------------- | ---------------- |
| op-host-web     | Browser bundle entry — wasm32 cdylib, renders via CanvasKit                       | ✅               |
| op-host-native  | Native host lib — WidgetHostNative + skia-safe GL backend (desktop + mobile)      | ❌ (native only) |
| op-engine-ffi   | Player C ABI — one engine, three shells (iOS Metal / Android EGL / host raster)     | ❌ (native only) |
| op-engine-jni   | Android JNI marshalling layer for op-engine-ffi (engine thread, registry, callbacks) | ❌ (native only) |
| op-host-desktop | Desktop binary `openpencil-desktop` (winit + skia-safe GL) — also the `--serve-web` daemon that hosts the web bundle | ❌ (native only) |
| op-cli          | `op` command-line tool                                                           | ❌ (native only) |

**Shared library crates:** `op-editor-core` (canonical `.op` state), `op-editor-ui` (platform-free widgets + `RenderBackend`), `op-editor-host-core` (transport-free host state machines), `op-collab` (open, transport-free collaboration protocol/hash/exact apply), `op-collab-transport` (native Noise/TCP, bounded admission, and optional LAN discovery), `op-mcp`, `op-ai`, `op-ai-skills`, `op-codegen`, `op-orchestrator`, `op-figma`, `op-git`, `op-opmerge`, `op-pen-loader`, `op-design-lint`, `op-config-store`, `op-process-io`, `op-acp`, `op-i18n`, `op-rpc-transport` — plus `op-collab-smoke` (authenticated two-process collaboration/recovery matrix) and `op-smoke` (headless design-turn test runner). The mobile players live in `packaging/` (`ios` + `android` shells over `op-engine-ffi` / `op-engine-jni`). The browser bundle renders through the official CanvasKit skia WASM (loaded separately), so the retired `skia-safe-op` wasm fork + `wasm-libc-shim` no longer exist.

**Submodule:** `vendor/agent` → `github.com/ZSeven-W/agent-rs` (cross-product Rust agent runtime).

## Contributing

Contributions are welcome! See [CLAUDE.md](./CLAUDE.md) for architecture details and code style.

1. Fork and clone
2. Enable the version drift check: `git config core.hooksPath .githooks`
3. Create a branch: `git checkout -b feat/my-feature`
4. Run checks: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
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
- [x] Cargo workspace with reusable Rust crates and web SDK packages
- [x] Rust desktop & web editor
- [x] CLI tool (`op`) for terminal control
- [x] Built-in Rust agent runtime with multi-provider support
- [x] i18n — 15 languages
- [x] Wasm-backed viewer SDKs for JavaScript, React & Vue
- [x] Style Guides with tag-based matching and MCP tools
- [x] Concurrent Agent Teams with delegation and canvas indicators
- [x] Git integration — clone, branch, push/pull, folder-mode three-way merge
- [x] Canvas export (SVG / PNG / JPEG / WEBP / PDF)
- [x] Collaborative editing — authenticated P2P, public relay, and regional hubs
- [x] Presentation decks — templates, slideshow presenter, and PDF/HTML/PPTX/video export
- [x] Device login and online multi-tenant web hosting
- [x] Chrome web-capture extension with HTML / browser-snapshot import
- [ ] Plugin system

## Contributors

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Sponsors

OpenPencil is free and open-source. Development is funded by people who find it useful — thank you for keeping the canvas open.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

Thanks to **[MrQyun](https://github.com/mrqyun)** — want your name here too? **[Become a sponsor →](https://github.com/sponsors/ZSeven-W)**

## Community

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Join our Discord</strong>
</a>
— Ask questions, share designs, suggest features.

**Recognized community: [LINUX DO](https://linux.do/)**

## Forked Third-Party Libraries

We are grateful to the upstream maintainers whose work OpenPencil builds on. These copies are maintained only for OpenPencil-specific integration needs:

- **[casement](https://github.com/ZSeven-W/casement)** — forked from **[winit](https://github.com/rust-windowing/winit)**.
- **[anthropic-agent-sdk](./vendor/anthropic-agent-sdk)** — vendored from **[bartolli/anthropic-agent-sdk](https://github.com/bartolli/anthropic-agent-sdk)** and maintained as a local fork.

Each project remains subject to its respective upstream license.

## Assessments

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## License

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
