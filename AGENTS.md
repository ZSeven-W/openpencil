# AGENTS.md

This file provides guidance to Codex when working with code in this repository.

> **The TypeScript OpenPencil has been retired.** `apps/web`, `apps/desktop`, `apps/cli`, and the `pen-*` packages are gone. The product is **Rust** (`crates/`) + a **Zig** agent runtime (`packages/agent-native`) + a **wasm-backed web SDK** (`packages/op-web-sdk*`). See git history (last TS tag `v0.7.5`) for the retired code.

For full guidance see **`CLAUDE.md`** (this directory). Authoritative Rust architecture lives in **`crates/CLAUDE.md`**; remaining packages in **`packages/CLAUDE.md`**.

## Commands

Tooling is **Cargo** (Rust — the product) plus **Bun** (remaining JS/Zig glue).

- **Web dev server (Rust):** `bun run dev` (= `bash scripts/start-web-rust.sh`)
- **Build (Rust):** `bun run build` (= `cargo build --workspace --release`)
- **Tests (Rust):** `bun run test` (= `cargo test --workspace`); single crate: `cargo test -p <crate>`
- **Type check:** `cargo check --workspace`; wasm: `bun run cargo:wasm-check`
- **Lint / format (Rust):** `cargo clippy --workspace --all-targets -- -D warnings` / `cargo fmt --all`
- **Lint / format (TS SDK):** `bun run lint` (oxlint) / `bun run format` (oxfmt)
- **Desktop app:** `cargo build -p op-host-desktop` → binary `openpencil-desktop`
- **CLI:** `cargo build -p op-cli` → binary `op`
- **Agent runtime (Zig):** `bun run agent:build` then `bun run agent-native:bundle`

## Conventions

- Single files ≤ **800 lines**; one component/widget per file.
- `.rs` snake_case, `.ts`/`.tsx` kebab-case; source comments in English.
- Conventional Commits: `<type>(<scope>): <subject>` — scopes: `editor`, `canvas`, `panels`, `ai`, `codegen`, `variables`, `figma`, `mcp`, `desktop`, `web`, `renderer`, `sdk`, `cli`, `agent`, `i18n`.
