# Agent instructions

This package is the zero-dependency OpenPencil plugin, interactive Rust MCP App canvas, and local MCP connector for Qoder (`@zseven-w/qoder-openpencil`).

- Keep runtime dependencies empty outside the six atomic `@zseven-w/qoder-openpencil-<platform>` `optionalDependencies` (`Node.js >= 22` built-ins only).
- Follow Qoder's official plugin specification: `.qoder-plugin/plugin.json` is the canonical manifest, `.mcp.json` uses `${QODER_PLUGIN_ROOT}`, and `pack-plugin.mjs` places `.qoder-plugin/plugin.json` directly at the ZIP root.
- Preserve `qoder/ui` entrypoints (`global`, `thread`), `availableDisplayModes` (`inline`, `fullscreen`), and loopback `_meta.ui.csp.frameDomains` on `open_canvas` and its UI resource.
- Never overwrite unmanaged plugin cache directories or user skills, and always merge `<QODER_CONFIG_DIR>/plugins/installed_plugins_v2.json` and `settings.json` atomically without clobbering unrelated entries.
- Validate changes with `npm run check`, `npm test`, and `npm run verify:packages`.
- Keep single files at or below 800 lines, and write source comments in English.
