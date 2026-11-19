# Agent instructions

This package is the zero-dependency OpenPencil local stdio MCP connector, interactive Rust canvas, and uploadable Skill packager for Doubao Desktop Work Mode / 豆包办公 (`@zseven-w/doubao-openpencil`).

- Keep runtime dependencies empty outside the six atomic `@zseven-w/doubao-openpencil-<platform>` `optionalDependencies` (`Node.js >= 22` built-ins only).
- Keep `pack-skill.mjs` zero-dependency and compliant with Doubao Desktop's uploadable Skill ZIP format (`SKILL.md` frontmatter + linked `references/`).
- Never overwrite unmanaged skill directories when `--skills-dir` is used, and always merge `--mcp-config` JSON atomically while preserving unrelated servers.
- Validate changes with `npm run check`, `npm test`, and `npm run verify:packages`.
- Keep single files at or below 800 lines, and write source comments in English.
