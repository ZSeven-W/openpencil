---
name: openpencil-setup
description: Connect and diagnose the local Rust OpenPencil plugin for Qoder (Qoder Desktop, Qoder CN, Qoder IDE/CLI). Use for initial setup, local plugin ZIP import, missing tools, or a disconnected local canvas.
---

# Connect OpenPencil to Qoder

This plugin opens a managed interactive Rust OpenPencil canvas directly inside
Qoder (via MCP Apps 2026-01-26) or connects to an already running OpenPencil
desktop editor.

## Quick setup

1. Install `@zseven-w/qoder-openpencil` (Node.js 22+) and run:
   `qoder-openpencil setup` (or `npm run setup` from `packages/op-qoder`).
   Setup verifies an isolated native edit/save/reopen before changing host
   configuration, records the current Node 22+ executable privately in
   `~/.qoder-openpencil/node-path`, installs the plugin into Qoder's canonical
   cache (`<QODER_CONFIG_DIR>/plugins/cache/local/openpencil/<version>/`),
   registers `openpencil@local` in `plugins/installed_plugins_v2.json`, and
   enables `enabledPlugins["openpencil@local"] = true` in `settings.json`
   (preserving all unrelated plugins and settings).
2. **Import via Qoder Desktop UI (ZIP)**:
   Run `qoder-openpencil pack-plugin` to generate `openpencil-qoder-plugin.zip`
   with `.qoder-plugin/plugin.json` and `.mcp.json` directly at the ZIP root,
   then import it through **Extensions → Import local plugin** (or pass it to
   `qoder-settings.install_extension`).
3. **Qoder IDE / CLI `mcpServers` mode**:
   Run `qoder-openpencil print-config` to paste into `settings.json`, or run
   `qoder-openpencil setup --mcp-settings` to merge `mcpServers.openpencil` and
   install standalone skills into `<QODER_CONFIG_DIR>/skills/`.
4. Run `qoder-openpencil host-check` (or `npm run host-check`) to verify that
   `launch-mcp` starts cleanly under a minimal GUI `PATH`, serves `open_canvas`
   with `qoder/ui` entrypoints, and exposes `ui://openpencil/canvas/...`.

## Interactive canvas

1. Open directly from Qoder's left **More → OpenPencil Canvas** menu (Global
   fullscreen workspace tab), from the session sidebar **+ → Apps → OpenPencil
   Canvas** (side-by-side tab), or by calling `open_canvas` in chat.
2. Runtime configuration stays private in `~/.qoder-openpencil/runtime.json`.
   A custom complete Rust runtime can be selected with
   `qoder-openpencil configure-runtime /abs/runtime/root` or
   `OPENPENCIL_EDITOR_RUNTIME_DIR`.
3. Each conversation keeps one canvas. Recovery drafts persist in
   `~/.qoder-openpencil/sessions/`.
4. If the current chat has not reloaded newly installed MCP tools yet,
   `qoder-openpencil canvas` (or `npm run canvas` from a checkout) opens the
   same managed canvas through a persistent terminal.

## Existing desktop editor

1. Try `get_editor_state` through this plugin's tools.
2. If unavailable, run `qoder-openpencil doctor` (or
   `node scripts/mcp-bridge.mjs --doctor` from the installed plugin directory).
   The doctor reads `~/.openpencil/.op-mcp-port` and verifies instance identity.
3. For a headless/web endpoint without a discovery record, set
   `OPENPENCIL_MCP_PORT` or pass `--port PORT` to `mcp-bridge.mjs`. Only local
   loopback ports are accepted.
