# Agent instructions

This package is the zero-dependency OpenPencil connector and MCP Apps plugin for Tencent WorkBuddy (`@zseven-w/workbuddy-openpencil`).

- Keep runtime dependencies empty outside the six atomic `@zseven-w/workbuddy-openpencil-<platform>` `optionalDependencies` (`Node.js >= 22` built-ins only).
- Keep plugin manifests (`.workbuddy-plugin/plugin.json`, `.codebuddy-plugin/plugin.json`, `plugin.json`) and marketplace manifests synchronized via `npm run release:sync`.
- Never write unmanaged skills over user-owned directories in `~/.workbuddy/skills/`, and never clobber unrelated MCP servers in `~/.workbuddy/mcp.json`.
- Validate changes with `npm run check`, `npm test`, and `npm run verify:packages`.
- Keep single files at or below 800 lines, and write source comments in English.
