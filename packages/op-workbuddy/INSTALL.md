# Install OpenPencil Connector for WorkBuddy

## Prerequisites

- **Node.js 22+**
- **Tencent WorkBuddy Desktop** (`WorkBuddy.app` on macOS, or Windows/Linux desktop build)
- No Rust toolchain or standalone OpenPencil desktop app is required when installing `@zseven-w/workbuddy-openpencil` from npm (its matching platform optional package carries `op-host-web-server`, WASM, and CanvasKit).

## Install from npm

```sh
npm install -g @zseven-w/workbuddy-openpencil
workbuddy-openpencil setup
```

## Install from a monorepo checkout

```sh
git clone https://github.com/ZSeven-W/openpencil.git
cd openpencil/packages/op-workbuddy
npm run build:editor-runtime && npm run stage:editor-runtime
# Or point at an existing verified runtime root:
# npm run configure-runtime -- /absolute/path/to/runtime
npm run setup
```

## What `setup` does

1. Resolves and SHA-256 verifies the atomic Rust OpenPencil editor runtime (`op-host-web-server` + `web-bundle` + `canvaskit`).
2. Runs an isolated live verification: loads the local panel, performs a native `update_node` edit, saves to `.op`, and reopens the file.
3. Privately records your Node 22+ executable path in `~/.workbuddy-openpencil/node-path` so WorkBuddy's GUI process works even when its `PATH` omits nvm/fnm/volta/Homebrew.
4. Atomically merges the `openpencil` stdio MCP connector into `~/.workbuddy/mcp.json` (respects `WORKBUDDY_CONFIG_DIR` or `--config-dir`) without touching any other MCP servers.
5. Installs `openpencil-design` and `openpencil-setup` skills into `~/.workbuddy/skills/` with ownership markers.
6. Runs `host-check` under a minimal GUI `PATH` to verify MCP `initialize`, `tools/list`, and the `ui://openpencil/canvas/...` MCP App resource.

## Enable in WorkBuddy

1. Open WorkBuddy -> **连接器 (Connectors)**.
2. Locate **openpencil** and click **信任 / 启用 (Trust / Enable)**.
3. Start a new chat and ask WorkBuddy to design with OpenPencil — WorkBuddy calls `open_canvas` and mounts the interactive Rust WASM canvas directly inside the MCP App panel.

## Manual UI configuration

To paste the connector JSON directly in WorkBuddy's **连接器 -> 自定义连接器** dialog:

```sh
workbuddy-openpencil print-config
```

## Diagnostics & uninstall

```sh
workbuddy-openpencil host-check
workbuddy-openpencil doctor
workbuddy-openpencil canvas --file /absolute/path/design.op
workbuddy-openpencil uninstall
```
