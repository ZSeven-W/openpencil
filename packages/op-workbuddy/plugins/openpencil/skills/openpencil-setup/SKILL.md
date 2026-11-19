---
name: openpencil-setup
description: Connect and diagnose the local Rust OpenPencil connector for Tencent WorkBuddy. Use for initial setup, missing tools, Trust approval, or a disconnected local canvas.
---

# Connect OpenPencil to WorkBuddy

This connector opens a managed interactive Rust OpenPencil canvas (embedded via
WorkBuddy's MCP Apps host) or connects to an already running OpenPencil desktop
editor.

## Quick setup

1. Install `@zseven-w/workbuddy-openpencil` (Node.js 22+) and run:
   `workbuddy-openpencil setup` (or `npm run setup` from `packages/op-workbuddy`).
   Setup verifies an isolated native edit/save/reopen before touching WorkBuddy
   configuration, registers the current Node 22+ executable privately in
   `~/.workbuddy-openpencil/node-path`, writes the `openpencil` MCP entry into
   `~/.workbuddy/mcp.json` (preserving unrelated servers), and installs the
   OpenPencil skills into `~/.workbuddy/skills/`.
2. Open WorkBuddy -> **连接器 (Connectors)** -> find **openpencil** and approve
   **信任 / 启用 (Trust / Enable)**, then reconnect or start a new conversation
   so WorkBuddy loads the MCP tools and MCP App resource.
3. Run `workbuddy-openpencil host-check` (or `npm run host-check`) to verify
   that `launch-mcp` starts cleanly under a minimal GUI `PATH`, serves
   `open_canvas`, and exposes the `ui://openpencil/canvas/...` MCP App resource.

## Interactive canvas

1. Call `open_canvas` (optionally with an existing absolute `.op` path).
   WorkBuddy hosts the MCP App resource and allows the loopback
   `http://127.0.0.1` panel iframe declared in `_meta.ui.csp.frameDomains`, so
   the full editable Rust WASM canvas renders inline.
2. If the current WorkBuddy surface does not embed MCP Apps, call
   `canvas_open_browser` and open its returned `panelUrl` in WorkBuddy's
   browser preview. Preserve any theme/launch fragment.
3. Runtime configuration stays private in `~/.workbuddy-openpencil/runtime.json`.
   A custom complete Rust runtime can be selected with
   `workbuddy-openpencil configure-runtime /abs/runtime/root` or
   `OPENPENCIL_EDITOR_RUNTIME_DIR`.
4. Each conversation keeps one canvas. Recovery drafts persist in
   `~/.workbuddy-openpencil/sessions/`.
5. If this conversation has not reloaded newly installed MCP tools yet,
   `workbuddy-openpencil canvas` (or `npm run canvas` from a checkout) opens
   the same managed canvas through a persistent terminal.

## Existing desktop editor

1. Try `get_editor_state` through this connector's tools.
2. If unavailable, run `workbuddy-openpencil doctor` (or
   `node scripts/mcp-bridge.mjs --doctor` from the installed plugin directory).
   The doctor reads `~/.openpencil/.op-mcp-port` and verifies instance identity.
3. For a headless/web endpoint without a discovery record, set
   `OPENPENCIL_MCP_PORT` or pass `--port PORT` to `mcp-bridge.mjs`. Only local
   loopback ports are accepted.

## Manual connector JSON

Run `workbuddy-openpencil print-config` to print the exact `mcpServers` JSON
block for pasting into WorkBuddy's **连接器 -> 自定义连接器** dialog.
