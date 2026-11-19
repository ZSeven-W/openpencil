# Codex OpenPencil

This package (`packages/op-codex`) owns the Codex plugin packaging (`@zseven-w/codex-openpencil`
plus six platform-selective `@zseven-w/codex-openpencil-*` npm runtime packages), local MCP
bridge, CLI entrypoint (`codex-openpencil`), and design workflows inside the `ZSeven-W/openpencil`
monorepo (alongside `packages/op-dsh`). The Rust editor and MCP tool implementations belong
to the root `crates/` workspace.

- Keep the runtime dependency-free (outside the six platform `optionalDependencies`) and compatible with Node.js 22+.
- Run `npm run check`, `npm test`, `npm run verify:packages`, and `npm run test:release-publish` before committing.
- Use `npm run release:sync -- <version>` to synchronize the root manifest, six `npm/*/package.json` manifests, `plugin.json`, `canvas-controller.mjs`, and `ui/app.html`; verify with `npm run release:check -- <version>`.
- Tag releases as `codex-v<version>` (handled by `.github/workflows/codex-release.yml` via npm OIDC trusted publishing, publishing the six native packages first and root last).
- Use `npm run stage:editor-runtime` to stage a locally built `op-host-web-server` + `web-bundle` into `npm/<platform>/`, or `npm run smoke -- --binary PATH` for an isolated file-backed host; never edit the user's live canvas.
- Do not bundle machine-local paths, ports, credentials, or generated user designs.
- Never retry a failed mutation automatically: a timeout may follow an applied edit.
- Keep individual source files under 800 lines. Use English source comments.
- Conventional commits; new feature branches use `codex/`.
