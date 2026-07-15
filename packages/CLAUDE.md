# Packages

The web SDK packages that remain after the TypeScript retirement. This directory is the **npm/Bun workspace root** (`packages/package.json`); the repository root is a **pure Cargo workspace** with no `package.json`.

> The `pen-*` packages (pen-types, pen-core, pen-engine, pen-renderer, pen-figma, pen-mcp, pen-ai-skills, pen-sdk, pen-react, pen-acp) and pen-codegen were **retired** along with `apps/*`. Their functionality now lives in the Rust `crates/` (see `crates/CLAUDE.md`). Nothing here depends on them. The `agent-native` Zig runtime was also **removed** — the built-in agent runtime is now the Rust `agent` crate (`vendor/agent`, shared with Zode).

## Workspace tooling

Run these from `packages/`:

- **Lint / format the SDK:** `bun run lint` (oxlint) / `bun run format` (oxfmt).
- **Iconify catalog (Rust assets):** `bun run generate-iconify-catalog` — `scripts/generate-iconify-catalog.mjs` reads `@iconify-json/*` and writes `crates/op-editor-ui/assets/iconify-catalog-{core,brands}.json` (the icon catalog embedded in / served by the Rust web target).
- **Sync SDK versions:** `bun run sync-version` reads the canonical version from root `Cargo.toml` and updates all SDK consumers; verify with `bun run sync-version:check`.

## op-web-sdk (`op-web-sdk/`)

Read-only OpenPencil `.op` **viewer** SDK for the web, wasm-backed. Wraps the `op-host-web` CanvasKit wasm bundle behind a small JS/TS embedding API (mount / load `.op` / viewport control / zoom-to-fit). Replaces the public role of the retired `pen-react` (viewing only — editing is not a goal of the public SDK).

- Zero runtime dependencies; ships its own wasm under `wasm/`.
- Build: `tsup` (`bun run build` inside the package). Tests: `vitest`.

## op-web-sdk-react (`op-web-sdk-react/`)

React 19 adapter for `op-web-sdk` (component + hooks wrapper). Depends only on `@zseven-w/op-web-sdk` (+ peer `react` / `react-dom`).

## op-web-sdk-vue (`op-web-sdk-vue/`)

Vue 3 adapter for `op-web-sdk`. Depends only on `@zseven-w/op-web-sdk` (+ peer `vue`).
