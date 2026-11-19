# Agent instructions

This package is the zero-dependency OpenPencil connector, interactive Rust canvas, and local plugin package for QwenWork / 千问办公 (`@zseven-w/qwenwork-openpencil`).

- Keep runtime dependencies empty outside the six atomic `@zseven-w/qwenwork-openpencil-<platform>` `optionalDependencies` (`Node.js >= 22` built-ins only).
- Keep all four plugin manifests (`.qwenwork-plugin/plugin.json`, `.qoder-plugin/plugin.json`, `.claude-plugin/plugin.json`, `plugin.json`) synchronized via `npm run release:sync`.
- Keep `pack-plugin.mjs` zero-dependency and compliant with QwenWorkCN's ZIP import contract (`/(^|\/)\.qwenwork-plugin\/plugin\.json\/?$/`, POSIX `0o755` on `scripts/launch-mcp`).
- Never write unmanaged skills over user-owned directories in `~/.qwenworkcn/skills/` (or `~/.qwenwork/skills/`), and never clobber unrelated MCP servers or `cliConnectors` in `mcp.json`.
- Validate changes with `npm run check`, `npm test`, and `npm run verify:packages`.
- Keep single files at or below 800 lines, and write source comments in English.
