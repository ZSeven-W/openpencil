# Install OpenPencil Plugin for Qoder

## Prerequisites

- **Node.js 22+**
- **Qoder Desktop (`Qoder.app` on macOS / Windows)**, Qoder CN, or Qoder IDE / CLI
- No Rust toolchain or standalone OpenPencil desktop app is required when installing `@zseven-w/qoder-openpencil` from npm (its matching platform optional package carries `op-host-web-server`, WASM, and CanvasKit).

## Install from npm

```sh
npm install -g @zseven-w/qoder-openpencil
qoder-openpencil setup
```

## Install from a monorepo checkout

```sh
git clone https://github.com/ZSeven-W/openpencil.git
cd openpencil/packages/op-qoder
npm run build:editor-runtime && npm run stage:editor-runtime
# Or point at an existing verified runtime root:
# npm run configure-runtime -- /absolute/path/to/runtime
npm run setup
```

## Supported Qoder Installation Flows

### Method 1: CLI `setup` — Canonical Qoder Plugin Install (Recommended)

```sh
qoder-openpencil setup
```

`setup` performs all of the following safely:
1. Resolves and SHA-256 verifies the atomic Rust OpenPencil editor runtime (`op-host-web-server` + `web-bundle` + `canvaskit`).
2. Runs an isolated live verification: loads the local panel, performs a native `update_node` edit, saves to `.op`, and reopens the file.
3. Privately records your Node 22+ executable path in `~/.qoder-openpencil/node-path` so Qoder's GUI process works even when its `PATH` omits nvm/fnm/volta/Homebrew.
4. Copies the plugin into Qoder's canonical versioned local cache (`<QODER_CONFIG_DIR>/plugins/cache/local/openpencil/<version>/`), atomically registers `openpencil@local` in `plugins/installed_plugins_v2.json`, and enables `enabledPlugins["openpencil@local"] = true` in `settings.json` (respecting `QODER_CONFIG_DIR`, `QODERCN_CONFIG_DIR`, `QODER_CLI_HOME`, `~/.qoder`, `~/.qoder-cn`).
5. Runs `host-check` under a stripped GUI `PATH` to verify MCP `initialize`, `tools/list` (`qoder/ui` `global` + `thread` entrypoints), and the `ui://openpencil/canvas/...` MCP App resource.

Restart Qoder (or reload **Extensions → Installed**) to activate.

### Method 2: Import Local Plugin ZIP in Qoder Desktop

To import through Qoder Desktop's UI (or `qoder-settings.install_extension`):

```sh
qoder-openpencil pack-plugin --output ./openpencil-qoder-plugin.zip
```

The generated ZIP places `.qoder-plugin/plugin.json` and `.mcp.json` directly at the ZIP root (per Qoder's archive specification). Import it in **Extensions → Import local plugin**.

### Method 3: Qoder IDE / CLI `settings.json` `mcpServers` Mode

If you prefer standalone `mcpServers` + `skills/` instead of the plugin cache:

```sh
qoder-openpencil setup --mcp-settings
# Or print JSON to paste manually:
qoder-openpencil print-config
```

## Opening OpenPencil Canvas in Qoder Desktop

- **Global Fullscreen App**: Click **More → OpenPencil Canvas** at the bottom-left navigation bar.
- **Session Sidebar Tab**: Click **Open sidebar → + → Apps → OpenPencil Canvas** to keep the canvas side-by-side with chat.
- **In Chat**: Ask Qoder to design with OpenPencil (`open_canvas`).

## Diagnostics & Uninstall

```sh
qoder-openpencil host-check
qoder-openpencil doctor
qoder-openpencil canvas --file /absolute/path/design.op
qoder-openpencil uninstall
```
