# AGENTS.md

This file provides guidance to Codex when working with code in this repository.

> **The TypeScript OpenPencil has been retired.** `apps/web`, `apps/desktop`, `apps/cli`, and the `pen-*` packages are gone. The product is **Rust** (`crates/`) + a **wasm-backed web SDK** (`packages/op-web-sdk*`). See git history (last TS tag `v0.7.5`) for the retired code.

For full guidance see **`CLAUDE.md`** (this directory). Authoritative Rust architecture lives in **`crates/CLAUDE.md`**; remaining packages in **`packages/CLAUDE.md`**.

## Commands

Tooling is **Cargo** (Rust — the product). The root has **no `package.json`**; the JS/**Bun** tooling for the web SDK lives under `packages/` — run SDK/JS scripts from there.

- **Web dev server (Rust):** `bash scripts/start-web-rust.sh`
- **Build (Rust):** `cargo build --workspace --release`
- **Tests (Rust):** `cargo test --workspace`; single crate: `cargo test -p <crate>`
- **Type check:** `cargo check --workspace`; wasm: `cargo check --target wasm32-unknown-unknown -p op-host-web --no-default-features --features web`
- **Lint / format (Rust):** `cargo clippy --workspace --all-targets -- -D warnings` / `cargo fmt --all`
- **Lint / format (TS SDK):** from `packages/`: `bun run lint` (oxlint) / `bun run format` (oxfmt)
- **Desktop app:** `cargo build -p op-host-desktop` → binary `openpencil-desktop`
- **CLI:** `cargo build -p op-cli` → binary `op`
- **Iconify catalog (Rust assets):** from `packages/`: `bun run generate-iconify-catalog`

## Conventions

- Single files ≤ **800 lines**; one component/widget per file.
- `.rs` snake_case, `.ts`/`.tsx` kebab-case; source comments in English.
- Conventional Commits: `<type>(<scope>): <subject>` — scopes: `editor`, `canvas`, `panels`, `ai`, `codegen`, `variables`, `figma`, `mcp`, `desktop`, `web`, `renderer`, `sdk`, `cli`, `agent`, `i18n`.
