# Packages

The JS/Zig packages that remain after the TypeScript retirement.

> The `pen-*` packages (pen-types, pen-core, pen-engine, pen-renderer, pen-figma, pen-mcp, pen-ai-skills, pen-sdk, pen-react, pen-acp) and pen-codegen were **retired** along with `apps/*`. Their functionality now lives in the Rust `crates/` (see `crates/CLAUDE.md`). Nothing here depends on them.

## agent-native (`agent-native/`)

Native AI agent runtime written in **Zig**, exposed to JS via a NAPI addon (`napi/agent_napi.node`). Multi-provider, supports concurrent Agent Teams. Cross-product runtime (also consumed by Zode).

- Build: `bun run agent:build` (`zig build napi -Doptimize=ReleaseFast`), then `bun run agent-native:bundle` to copy the `.node` into `napi/`.
- See `agent-native/CLAUDE.md` for the full runtime docs.

## op-web-sdk (`op-web-sdk/`)

Read-only OpenPencil `.op` **viewer** SDK for the web, wasm-backed. Wraps the `op-host-web` CanvasKit wasm bundle behind a small JS/TS embedding API (mount / load `.op` / viewport control / zoom-to-fit). Replaces the public role of the retired `pen-react` (viewing only — editing is not a goal of the public SDK).

- Zero runtime dependencies; ships its own wasm under `wasm/`.
- Build: `tsup` (`bun run build` inside the package). Tests: `vitest`.

## op-web-sdk-react (`op-web-sdk-react/`)

React 19 adapter for `op-web-sdk` (component + hooks wrapper). Depends only on `@zseven-w/op-web-sdk` (+ peer `react` / `react-dom`).

## op-web-sdk-vue (`op-web-sdk-vue/`)

Vue 3 adapter for `op-web-sdk`. Depends only on `@zseven-w/op-web-sdk` (+ peer `vue`).
