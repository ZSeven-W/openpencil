# Doubao Desktop OpenPencil Connector

**OpenPencil local `stdio` MCP connector, interactive Rust canvas, and uploadable Skill packager for Doubao Desktop Work Mode / 豆包办公 (`@zseven-w/doubao-openpencil`).**

[English](./README.md) · [简体中文](./README.zh.md) · [Install Guide](./INSTALL.md) · [Architecture](./docs/architecture.md)

## Features

- **Managed Interactive Rust Canvas**: Runs a dedicated `op-host-web-server` (Rust WASM + CanvasKit) per task, returning a capability-protected local `panelUrl` (`canvas_open_browser` / `open_canvas`) plus standard MCP Apps metadata (`text/html;profile=mcp-app`) for hosts with inline preview.
- **Built for Doubao Desktop 「技能·连接器·伙伴」**:
  - `doubao-openpencil print-config` — copy-pasteable `mcpServers` JSON for JSON connector import.
  - `doubao-openpencil print-form` — exact field-by-field values for Doubao Desktop's manual `本地命令 / stdio` connector dialog.
  - `doubao-openpencil pack-skill` — zero-dependency ZIP 2.0 packager that bundles `openpencil-design/SKILL.md` + `references/` (with optional `--flat` root layout) for **我的技能 → 新建 → 上传技能**.
- **GUI-Safe Node 22+ Launcher**: Privately stores Node 22+ path in `~/.doubao-openpencil/node-path` so Doubao's GUI process launches `mcp-bridge.mjs` cleanly even when `PATH` omits nvm/fnm/volta/Homebrew.
- **Full Native Design Tools & Guarded Saves**: Proxies all native Rust OpenPencil MCP tools (`get_editor_state`, `batch_design`, `snapshot_layout`, `lint_document`, `get_screenshot`, `codegen_export`, etc.) with isolated recovery drafts under `~/.doubao-openpencil/sessions/` and SHA-256 save guards.

## Quick Start

```sh
npm install -g @zseven-w/doubao-openpencil
doubao-openpencil setup
doubao-openpencil print-form
```

Then in Doubao Desktop:
1. Add the `openpencil` local `stdio` connector under **技能·连接器·伙伴 → 连接器 → 手动添加**.
2. Upload `~/.doubao-openpencil/exports/openpencil-design-doubao-skill.zip` under **我的技能 → 新建 → 上传技能**.

## CLI Commands

| Command | Purpose |
| --- | --- |
| `doubao-openpencil setup` | Verify runtime, record Node 22+, run `host-check`, export connector JSON + Skill ZIP |
| `doubao-openpencil print-config` | Print copy-pasteable `mcpServers` JSON |
| `doubao-openpencil print-form` | Print field-by-field values for Doubao manual `stdio` connector form |
| `doubao-openpencil pack-skill [--skill NAME] [--flat]` | Build an uploadable Doubao Desktop Skill `.zip` |
| `doubao-openpencil host-check` | Probe `launch-mcp` under a stripped GUI `PATH` |
| `doubao-openpencil canvas [--file PATH]` | Run the persistent terminal canvas |
| `doubao-openpencil doctor [--port PORT]` | Check a running OpenPencil desktop MCP server |
| `doubao-openpencil uninstall` | Remove managed entries from optional `--mcp-config` / `--skills-dir` |

## Development & Verification

```sh
cd packages/op-doubao
npm run check
npm test
npm run verify:packages
```
