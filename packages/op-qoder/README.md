# Qoder OpenPencil Plugin

**OpenPencil plugin, native MCP Apps Rust canvas (Global + Session Sidebar tabs), and local MCP connector for Qoder (`@zseven-w/qoder-openpencil`).**

[English](./README.md) · [简体中文](./README.zh.md) · [Install Guide](./INSTALL.md) · [Architecture](./docs/architecture.md)

## Features

- **Native Qoder MCP Apps (`2026-01-26` + `qoder/ui`)**:
  - Declares `global` and `thread` entrypoints on `open_canvas`, so OpenPencil appears directly in Qoder's bottom-left **More** menu (fullscreen workspace tab) and each conversation's right-hand **+ → Apps** sidebar!
  - Declares `_meta.ui.csp.frameDomains` for the local `127.0.0.1` panel origin (explicitly permitted by Qoder Desktop's CSP allowlist) and `availableDisplayModes: ["inline", "fullscreen"]` with `preferredDisplayMode: "fullscreen"`.
  - Synchronizes light/dark theme, pushes selection context via `ui/update-model-context` (with `qoder/title` + `structuredContent`), and hands off Code panel requests via `ui/message`.
- **Canonical Qoder Plugin Registration & ZIP Import**:
  - `qoder-openpencil setup` installs into `<QODER_CONFIG_DIR>/plugins/cache/local/openpencil/<version>/`, merges `plugins/installed_plugins_v2.json`, and enables `openpencil@local` in `settings.json` (auto-detecting `~/.qoder` and `~/.qoder-cn`).
  - `qoder-openpencil pack-plugin` builds a zero-dependency ZIP with `.qoder-plugin/plugin.json` and `.mcp.json` (`${QODER_PLUGIN_ROOT}`) directly at the ZIP root for Qoder Desktop UI import or `qoder-settings.install_extension`.
  - `qoder-openpencil setup --mcp-settings` / `print-config` also support Qoder IDE / CLI `settings.json` `mcpServers` setups.
- **Full Native Design Tools & Guarded Persistence**: Proxies all native Rust OpenPencil MCP tools into one serialized per-conversation session with recovery drafts under `~/.qoder-openpencil/sessions/` and SHA-256 verified saves.

## Quick Start

```sh
npm install -g @zseven-w/qoder-openpencil
qoder-openpencil setup
```

Then restart Qoder (or reload **Extensions → Installed**) and open **More → OpenPencil Canvas** or **+ → Apps → OpenPencil Canvas**.

## CLI Commands

| Command | Purpose |
| --- | --- |
| `qoder-openpencil setup` | Verify runtime, install & enable `openpencil@local` in Qoder, run `host-check` |
| `qoder-openpencil pack-plugin [--output PATH]` | Build a Qoder local plugin `.zip` (`.qoder-plugin/plugin.json` at ZIP root) |
| `qoder-openpencil print-config` | Print copy-pasteable `mcpServers` JSON for Qoder IDE / CLI `settings.json` |
| `qoder-openpencil host-check` | Probe the registered launcher under a stripped GUI `PATH` |
| `qoder-openpencil canvas [--file PATH]` | Run the persistent terminal canvas |
| `qoder-openpencil doctor [--port PORT]` | Check a running OpenPencil desktop MCP server |
| `qoder-openpencil uninstall` | Cleanly remove `openpencil@local`, `mcpServers.openpencil`, and managed skills |

## Development & Verification

```sh
cd packages/op-qoder
npm run check
npm test
npm run verify:packages
```
