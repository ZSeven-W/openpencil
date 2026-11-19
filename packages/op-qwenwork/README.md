# QwenWork OpenPencil Connector

**OpenPencil connector, interactive Rust canvas, and local plugin kit for QwenWork / 千问办公 (`@zseven-w/qwenwork-openpencil`).**

[English](./README.md) · [简体中文](./README.zh.md) · [Install Guide](./INSTALL.md) · [Architecture](./docs/architecture.md)

## Features

- **Managed Interactive Rust Canvas**: Runs a dedicated `op-host-web-server` with WASM/CanvasKit per conversation, exposing both MCP Apps (`open_canvas`, `mimeType: "text/html;profile=mcp-app"`) and browser-preview handoff (`canvas_open_browser`).
- **All Three QwenWork Registration Modes**:
  1. `qwenwork-openpencil setup` — configures `~/.qwenworkcn/mcp.json` (auto-detecting `.qwenworkcn`/`.qwenwork`/`.qoderworkcn`/`.qoderwork`), installs managed skills, and verifies startup via `host-check`.
  2. `qwenwork-openpencil print-config` — outputs copy-pasteable `mcpServers` JSON for **扩展 → 连接器 → 添加 → 粘贴 JSON 配置** (works in account-isolated mode).
  3. `qwenwork-openpencil pack-plugin` — packages a zero-dependency ZIP archive (`.qwenwork-plugin/plugin.json`, `.mcp.json`, `CONNECTORS.md`, `skills/`) for **扩展 → 套件 → 导入本地套件**.
- **Full Native Design Tools**: Proxies all native Rust OpenPencil MCP tools (`get_editor_state`, `batch_design`, `batch_get`, `snapshot_layout`, `lint_document`, `get_screenshot`, `codegen_export`, etc.) over a serialized per-conversation session.
- **Selection-Driven Editing & Code Handoff**: Supports `canvas_selection`, `canvas_import_image`, guarded `canvas_save`, and Code panel handoff (`canvas_codegen_*`).
- **Guarded Saves & Recovery Drafts**: Isolated per-session drafts under `~/.qwenwork-openpencil/sessions/` with SHA-256 baseline verification before overwriting source `.op` files.

## Quick Start

```sh
npm install -g @zseven-w/qwenwork-openpencil
qwenwork-openpencil setup
```

For account-isolated QwenWorkCN installs, either paste `qwenwork-openpencil print-config` into **扩展 → 连接器 → 添加**, or run `qwenwork-openpencil pack-plugin` and import the generated ZIP under **扩展 → 套件 → 导入本地套件**.

## CLI Commands

| Command | Purpose |
| --- | --- |
| `qwenwork-openpencil setup` | Verify runtime, register `mcp.json`, install skills, run `host-check` |
| `qwenwork-openpencil print-config` | Print copy-pasteable `mcpServers` JSON for QwenWork connector UI |
| `qwenwork-openpencil pack-plugin [--output PATH]` | Build a QwenWork-compatible local plugin kit `.zip` |
| `qwenwork-openpencil host-check` | Probe the registered launcher under a stripped GUI `PATH` |
| `qwenwork-openpencil canvas [--file PATH]` | Run the persistent terminal canvas |
| `qwenwork-openpencil doctor [--port PORT]` | Check a running OpenPencil desktop MCP server |
| `qwenwork-openpencil uninstall` | Remove `openpencil` from `mcp.json` and remove managed skills |

## Development & Verification

```sh
cd packages/op-qwenwork
npm run check
npm test
npm run verify:packages
```
