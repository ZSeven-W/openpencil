# OpenPencil Connector for Qoder

This plugin bundles the local `openpencil` stdio MCP server (`.mcp.json`) and exposes the interactive Rust OpenPencil canvas via Qoder's MCP Apps 2026-01-26 host protocol (`global` + `thread` sidebar entrypoints + `fullscreen` / `inline` display modes).

## Prerequisites

- **Node.js 22+** (run `qoder-openpencil setup` once so `~/.qoder-openpencil/node-path` is recorded for GUI launches)
- **OpenPencil Rust editor runtime** (`@zseven-w/qoder-openpencil-<platform>`, staged development runtime, or `OPENPENCIL_EDITOR_RUNTIME_DIR`)
- No API keys or external cloud tokens are required; all canvas traffic stays on `127.0.0.1` with per-session capability tokens.
