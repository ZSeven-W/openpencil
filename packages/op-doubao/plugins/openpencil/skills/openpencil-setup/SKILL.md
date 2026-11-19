---
name: openpencil-setup
description: Connect and diagnose the local Rust OpenPencil connector and uploadable skills for Doubao Desktop (豆包办公 / 豆包电脑版). Use for initial setup, custom stdio connector configuration, skill ZIP upload, missing tools, or a disconnected local canvas.
---

# Connect OpenPencil to Doubao Desktop (豆包办公 / 豆包电脑版)

This connector opens a managed interactive Rust OpenPencil canvas or connects to
an already running OpenPencil desktop editor over local `stdio` MCP.

## Quick setup

1. Install `@zseven-w/doubao-openpencil` (Node.js 22+) and run:
   `doubao-openpencil setup` (or `npm run setup` from `packages/op-doubao`).
   Setup verifies an isolated native edit/save/reopen before exporting host
   assets, records the current Node 22+ executable privately in
   `~/.doubao-openpencil/node-path`, runs `host-check` under a stripped GUI
   `PATH`, and exports both:
   - `~/.doubao-openpencil/exports/doubao-connector.json` (ready-to-paste `mcpServers` JSON)
   - `~/.doubao-openpencil/exports/openpencil-design-doubao-skill.zip` (ready-to-upload Skill ZIP)
2. **Add the local stdio connector in Doubao Desktop**:
   Open Doubao Desktop → **技能·连接器·伙伴 → 连接器 → 手动添加 / 自定义连接器**:
   - If the dialog accepts JSON, paste the output of `doubao-openpencil print-config`.
   - If the dialog uses form fields (`本地命令 / stdio`), copy the values from
     `doubao-openpencil print-form`.
   - Save, enable **openpencil**, and reload/restart Doubao Desktop if prompted.
3. **Upload the design skill**:
   Open Doubao Desktop → **技能·连接器·伙伴 → 我的技能 → 新建 → 上传技能**,
   and select the `.zip` built by `doubao-openpencil pack-skill` (or the export
   under `~/.doubao-openpencil/exports/`). Invoke it in chat via `/openpencil-design`.
4. Run `doubao-openpencil host-check` (or `npm run host-check`) to verify that
   `launch-mcp` starts cleanly under a minimal GUI `PATH` and serves `open_canvas`
   and `canvas_open_browser`.

## Interactive canvas

1. Call `canvas_open_browser` or `open_canvas` (optionally with an existing
   absolute `.op` path), then open the returned local `panelUrl` in Doubao's
   browser or your system browser while preserving its URL fragment.
2. Runtime configuration stays private in `~/.doubao-openpencil/runtime.json`.
   A custom complete Rust runtime can be selected with
   `doubao-openpencil configure-runtime /abs/runtime/root` or
   `OPENPENCIL_EDITOR_RUNTIME_DIR`.
3. Each task keeps one canvas. Recovery drafts persist in
   `~/.doubao-openpencil/sessions/`.
4. If the current task has not reloaded newly installed MCP tools yet,
   `doubao-openpencil canvas` (or `npm run canvas` from a checkout) opens the
   same managed canvas through a persistent terminal.

## Existing desktop editor

1. Try `get_editor_state` through this connector's tools.
2. If unavailable, run `doubao-openpencil doctor` (or
   `node scripts/mcp-bridge.mjs --doctor` from the installed connector directory).
   The doctor reads `~/.openpencil/.op-mcp-port` and verifies instance identity.
3. For a headless/web endpoint without a discovery record, set
   `OPENPENCIL_MCP_PORT` or pass `--port PORT` to `mcp-bridge.mjs`. Only local
   loopback ports are accepted.
