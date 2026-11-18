# Codex OpenPencil

This package (`packages/op-codex`) owns the Codex plugin packaging, local MCP bridge,
and design workflows inside the `ZSeven-W/openpencil` monorepo (alongside `packages/op-dsh`).
The Rust editor and MCP tool implementations belong to the root `crates/` workspace.

- Keep the runtime dependency-free and compatible with Node.js 22+.
- Run `npm run check` and `npm test` before committing.
- Use `npm run smoke -- --binary PATH` with a Rust OpenPencil host binary.
  This starts an isolated file-backed host; it must not edit the user's live canvas.
- Do not bundle machine-local paths, ports, credentials, or generated user designs.
- Never retry a failed mutation automatically: a timeout may follow an applied edit.
- Keep individual source files under 800 lines. Use English source comments.
- Conventional commits; new feature branches use `codex/`.
