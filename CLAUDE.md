# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **The TypeScript OpenPencil has been retired.** `apps/web`, `apps/desktop`, `apps/cli`, and the `pen-*` packages (pen-types/core/engine/renderer/figma/mcp/ai-skills/sdk/react/acp) are gone. The product is now implemented in **Rust** (`crates/`) with a thin **wasm-backed web SDK** (`packages/op-web-sdk*`). Historical `// ported from pen-*` comments in the Rust sources name the retired TS as their origin — that code no longer exists in-tree; consult git history (last TS tag `v0.7.5`) if you need it.

Detailed module docs load automatically in subdirectories:

- **`crates/CLAUDE.md`** — Rust shell: crate layout, editor core, widgets, hosts (native/web/desktop), MCP, AI, orchestrator, codegen. **The canonical architecture doc.**
- **`packages/CLAUDE.md`** — Remaining packages (op-web-sdk web viewer SDK family).

## Commands

Tooling is **Cargo** (Rust — the product). The root has **no `package.json`**; the JS/**Bun** tooling for the web SDK lives under `packages/` — run SDK/JS scripts from there.

- **Web dev server (Rust):** `bash scripts/start-web-rust.sh`
- **Build (Rust):** `cargo build --workspace --release`
- **Run all tests (Rust):** `cargo test --workspace`; single crate: `cargo test -p <crate>`
- **Type check:** `cargo check --workspace`; wasm: `cargo check --target wasm32-unknown-unknown -p op-host-web --no-default-features --features web`
- **Lint / format (Rust):** `cargo clippy --workspace --all-targets -- -D warnings` / `cargo fmt --all`
- **Lint / format (remaining TS SDK):** from `packages/`: `bun run lint` (oxlint) / `bun run format` (oxfmt)
- **Desktop app:** `cargo build -p op-host-desktop` → binary `openpencil-desktop` (live MCP on `127.0.0.1:<port>/mcp`)
- **CLI:** `cargo build -p op-cli` → binary `op`
- **MCP server:** built into the desktop/web host (`--mcp <path>`); crate `op-mcp`
- **Iconify catalog (Rust assets):** from `packages/`: `bun run generate-iconify-catalog`
- **Bump SDK versions:** from `packages/`: `bun run bump <version>` (syncs the SDK `package.json`s; Rust versions live in Cargo.toml)

## Architecture

OpenPencil is an open-source, AI-native vector design tool (Design-as-Code). The editor — canvas engine, chrome, stores, MCP, AI — is Rust, built on the vendored **jian** skia/widget/render/event toolkit (`vendor/jian`) and **casement** winit fork (`vendor/casement`). See **`crates/CLAUDE.md`** for the authoritative crate map; the essentials:

```text
crates/
├── op-editor-core/       Canonical `.op` (PenDocument) editor state + EditorCommand + design-variable resolution
├── op-editor-ui/         Platform-free widgets + RenderBackend facade (wasm32-clean)
├── op-editor-host-core/  Transport-free host state machines shared by all hosts
├── op-host-native/       Native host lib (winit + skia-safe GL) — desktop + mobile
├── op-host-web/          Browser bundle: wasm32 cdylib, CanvasKit renderer
├── op-host-desktop/      Desktop binary `openpencil-desktop`; also the `--serve-web` daemon
├── op-host-services/     Headless serve-web / MCP daemon lib (shared by desktop + web-server)
├── op-host-web-server/   Thin GL-free web-server binary
├── op-cli/               `op` command-line tool
└── op-mcp / op-ai / op-ai-skills / op-codegen / op-orchestrator / op-figma /
   op-git / op-opmerge / op-pen-loader / op-design-lint / op-i18n / op-smoke / …

packages/
├── op-web-sdk/           Read-only `.op` web viewer SDK (wraps the op-host-web wasm bundle)
├── op-web-sdk-react/     React 19 adapter for op-web-sdk
└── op-web-sdk-vue/       Vue 3 adapter for op-web-sdk
```

Data flow, canvas engine, Document/EditorCommand model, MCP layered-design workflow, design variables, and the run/debug recipe are all documented in **`crates/CLAUDE.md`**.

## Code Style

- Single files must not exceed **800 lines** — split into smaller modules when they grow beyond this.
- One component/widget per file, single responsibility.
- `.rs` filenames use snake_case; `.ts`/`.tsx` (SDK) use kebab-case.
- Source comments (`.rs`/`.ts`/`.toml`) in **English** (spec/plan markdown + test CJK fixtures may keep Chinese).
- Rust widgets paint against jian's `Painter`; `draw_text` is **baseline-relative** (label components center via `centered_text_baseline_y`, not `(h-fs)/2`).

## Git Commit Convention

Use [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <subject>`

**Types:** `feat`, `fix`, `refactor`, `perf`, `style`, `docs`, `test`, `chore`

**Scopes:** `editor`, `canvas`, `panels`, `history`, `ai`, `codegen`, `store`, `types`, `variables`, `figma`, `mcp`, `desktop`, `web`, `renderer`, `sdk`, `cli`, `agent`, `i18n`

**Rules:** Subject in English, lowercase start, no period, imperative mood. Body optional; explain **why** not what. One commit per change.

## License

MIT License. See [LICENSE](./LICENSE) for details.
