# WorkBuddy OpenPencil Connector

**OpenPencil connector and embedded interactive Rust canvas for Tencent WorkBuddy (`@zseven-w/workbuddy-openpencil`).**

[English](./README.md) · [简体中文](./README.zh.md) · [Install Guide](./INSTALL.md) · [Architecture](./docs/architecture.md)

## Features

- **Embedded Interactive Rust Canvas (MCP Apps)**: Exposes `open_canvas` with `mimeType: "text/html;profile=mcp-app"` and `_meta.ui.csp.frameDomains` for the local loopback panel origin, which WorkBuddy's MCP Apps sandbox proxy allows inline.
- **Full Native Design Tools**: Proxies all native Rust OpenPencil MCP tools (`get_editor_state`, `batch_design`, `batch_get`, `snapshot_layout`, `lint_document`, `get_screenshot`, `codegen_export`, etc.) into one serialized per-conversation session.
- **Selection-Driven Editing & Code Handoff**: Synchronizes selection state via `ui/update-model-context`, exposes `canvas_selection` and `canvas_import_image`, and hands off Code panel requests through `ui/message`.
- **Guarded Saves & Recovery Drafts**: Every conversation works on an isolated recovery copy under `~/.workbuddy-openpencil/sessions/`; saves verify SHA-256 baselines and refuse silent overwrites.
- **One-Command Setup**: Registers `~/.workbuddy/mcp.json`, installs managed skills into `~/.workbuddy/skills/`, records the Node 22+ path privately for GUI launches, and verifies the real launcher via `host-check`.

## Quick Start

```sh
npm install -g @zseven-w/workbuddy-openpencil
workbuddy-openpencil setup
```

Then open WorkBuddy -> **Connectors**, trust/enable **openpencil**, and start a new chat.

## CLI Commands

| Command | Purpose |
| --- | --- |
| `workbuddy-openpencil setup` | Verify runtime, register `~/.workbuddy/mcp.json`, install skills, run `host-check` |
| `workbuddy-openpencil print-config` | Print copy-pasteable `mcpServers` JSON for WorkBuddy custom connector UI |
| `workbuddy-openpencil host-check` | Probe the registered launcher under a stripped GUI `PATH` |
| `workbuddy-openpencil canvas [--file PATH]` | Run the persistent terminal canvas |
| `workbuddy-openpencil doctor [--port PORT]` | Check a running OpenPencil desktop MCP server |
| `workbuddy-openpencil uninstall` | Remove `openpencil` from `~/.workbuddy/mcp.json` and remove managed skills |

## Development & Verification

```sh
cd packages/op-workbuddy
npm run check
npm test
npm run verify:packages
```
