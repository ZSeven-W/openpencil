---
name: openpencil-setup
description: Connect and diagnose the local Rust OpenPencil connector for QwenWork (千问办公). Use for initial setup, JSON connector import, plugin ZIP upload, missing tools, or a disconnected local canvas.
---

# Connect OpenPencil to QwenWork (千问办公)

This connector opens a managed interactive Rust OpenPencil canvas or connects to
an already running OpenPencil desktop editor.

## Quick setup

1. Install `@zseven-w/qwenwork-openpencil` (Node.js 22+) and run:
   `qwenwork-openpencil setup` (or `npm run setup` from `packages/op-qwenwork`).
   Setup verifies an isolated native edit/save/reopen before changing host
   configuration, records the current Node 22+ executable privately in
   `~/.qwenwork-openpencil/node-path`, writes `openpencil` into
   `~/.qwenworkcn/mcp.json` (or auto-detected `~/.qwenwork` / `~/.qoderworkcn` /
   `~/.qoderwork`, preserving unrelated entries), and installs the OpenPencil
   skills into `<configDir>/skills/`.
2. **Identity-isolated QwenWorkCN desktop accounts**:
   QwenWorkCN stores per-account custom connectors inside its identity-scoped
   store. After running `setup`, import the connector in one of two ways:
   - **Connector JSON**: Run `qwenwork-openpencil print-config` and paste the
     JSON into **扩展 → 连接器 → 添加 → 粘贴 JSON 配置**, then enable it.
   - **Plugin ZIP bundle**: Run `qwenwork-openpencil pack-plugin` to generate
     `openpencil-qwenwork-plugin.zip` (containing `.qwenwork-plugin/plugin.json`,
     `.qoder-plugin/plugin.json`, `.claude-plugin/plugin.json`, `.mcp.json`,
     `CONNECTORS.md`, and `skills/`), and import it under
     **扩展 → 套件 → 导入本地套件**.
3. Run `qwenwork-openpencil host-check` (or `npm run host-check`) to verify that
   `launch-mcp` starts cleanly under a minimal GUI `PATH`, serves `open_canvas`,
   and exposes the `ui://openpencil/canvas/...` MCP App resource.

## Interactive canvas

1. Call `open_canvas` or `canvas_open_browser` (optionally with an existing
   absolute `.op` path). Where standard MCP Apps preview is enabled, QwenWork
   embeds the interactive Rust WASM canvas directly; otherwise open the returned
   local `panelUrl` in QwenWork's browser/preview or the user's browser while
   preserving its theme/launch fragment.
2. Runtime configuration stays private in `~/.qwenwork-openpencil/runtime.json`.
   A custom complete Rust runtime can be selected with
   `qwenwork-openpencil configure-runtime /abs/runtime/root` or
   `OPENPENCIL_EDITOR_RUNTIME_DIR`.
3. Each conversation keeps one canvas. Recovery drafts persist in
   `~/.qwenwork-openpencil/sessions/`.
4. If the current chat has not reloaded newly installed MCP tools yet,
   `qwenwork-openpencil canvas` (or `npm run canvas` from a checkout) opens the
   same managed canvas through a persistent terminal.

## Existing desktop editor

1. Try `get_editor_state` through this connector's tools.
2. If unavailable, run `qwenwork-openpencil doctor` (or
   `node scripts/mcp-bridge.mjs --doctor` from the installed plugin directory).
   The doctor reads `~/.openpencil/.op-mcp-port` and verifies instance identity.
3. For a headless/web endpoint without a discovery record, set
   `OPENPENCIL_MCP_PORT` or pass `--port PORT` to `mcp-bridge.mjs`. Only local
   loopback ports are accepted.
