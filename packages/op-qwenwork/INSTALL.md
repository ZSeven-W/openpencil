# Install OpenPencil Connector for QwenWork (千问办公)

## Prerequisites

- **Node.js 22+**
- **QwenWork Desktop / 千问办公** (`QwenWorkCN.app` on macOS, or QwenWork / QoderWork desktop builds)
- No Rust toolchain or standalone OpenPencil desktop app is required when installing `@zseven-w/qwenwork-openpencil` from npm (its matching platform optional package carries `op-host-web-server`, WASM, and CanvasKit).

## Install from npm

```sh
npm install -g @zseven-w/qwenwork-openpencil
qwenwork-openpencil setup
```

## Install from a monorepo checkout

```sh
git clone https://github.com/ZSeven-W/openpencil.git
cd openpencil/packages/op-qwenwork
npm run build:editor-runtime && npm run stage:editor-runtime
# Or point at an existing verified runtime root:
# npm run configure-runtime -- /absolute/path/to/runtime
npm run setup
```

## Three Supported QwenWork Installation Flows

QwenWorkCN supports account-isolated connector storage as well as shared SDK configuration and local plugin kit import. Choose the method that matches your setup:

### Method 1: CLI `setup` (Recommended)

```sh
qwenwork-openpencil setup
```

`setup` performs all of the following safely:
1. Resolves and SHA-256 verifies the atomic Rust OpenPencil editor runtime (`op-host-web-server` + `web-bundle` + `canvaskit`).
2. Runs an isolated live verification: loads the local panel, performs a native `update_node` edit, saves to `.op`, and reopens the file.
3. Privately records your Node 22+ executable path in `~/.qwenwork-openpencil/node-path` so QwenWork's GUI process works even when its `PATH` omits nvm/fnm/volta/Homebrew.
4. Atomically merges `mcpServers.openpencil` into `~/.qwenworkcn/mcp.json` (or auto-detected `~/.qwenwork`, `~/.qoderworkcn`, `~/.qoderwork`, or `--config-dir` / `QWENWORK_CONFIG_DIR`) while preserving existing servers and `cliConnectors`.
5. Installs `openpencil-design` and `openpencil-setup` skills into `<configDir>/skills/` with ownership markers.
6. Runs `host-check` under a stripped GUI `PATH` to verify MCP `initialize`, `tools/list`, and the `ui://openpencil/canvas/...` MCP App resource.

### Method 2: Paste JSON into QwenWork UI (Account-Isolated Mode)

When QwenWorkCN runs with per-account identity isolation, custom connectors are stored inside the active account's connector database:

1. Run:
   ```sh
   qwenwork-openpencil print-config
   ```
2. Open QwenWork → **扩展 (Extensions) → 连接器 (Connectors) → 添加 (Add) → 粘贴 JSON 配置**.
3. Paste the printed `mcpServers` JSON and save.

### Method 3: Import Local Plugin Kit ZIP (导入本地套件)

To install both the OpenPencil connector and design skills together as a QwenWork plugin kit:

1. Ensure Node 22+ is recorded (`qwenwork-openpencil setup` or `qwenwork-openpencil host-check`).
2. Build the local plugin ZIP:
   ```sh
   qwenwork-openpencil pack-plugin --output ./openpencil-qwenwork-plugin.zip
   ```
3. Open QwenWork → **扩展 (Extensions) → 套件 (Plugins/Kits) → 导入本地套件** and select `openpencil-qwenwork-plugin.zip`.

## Interactive Canvas & Browser Preview

- Call `open_canvas` to start this conversation's managed Rust canvas. Where QwenWork enables standard MCP Apps preview for the connector, the canvas mounts inline via `text/html;profile=mcp-app`; otherwise use `canvas_open_browser` (or open the returned `panelUrl`) in QwenWork's preview/browser while preserving its URL fragment.

## Diagnostics & Uninstall

```sh
qwenwork-openpencil host-check
qwenwork-openpencil doctor
qwenwork-openpencil canvas --file /absolute/path/design.op
qwenwork-openpencil uninstall
```
