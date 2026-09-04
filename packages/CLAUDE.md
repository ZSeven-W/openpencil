# Packages

The web SDK packages that remain after the TypeScript retirement. This directory is the **npm/Bun workspace root** (`packages/package.json`); the repository root is a **pure Cargo workspace** with no `package.json`.

> The `pen-*` packages (pen-types, pen-core, pen-engine, pen-renderer, pen-figma, pen-mcp, pen-ai-skills, pen-sdk, pen-react, pen-acp) and pen-codegen were **retired** along with `apps/*`. Their functionality now lives in the Rust `crates/` (see `crates/CLAUDE.md`). Nothing here depends on them. The `agent-native` Zig runtime was also **removed** — the built-in agent runtime is now the Rust `agent` crate (`vendor/agent`, shared with Zode).

## Workspace tooling

Run these from `packages/`:

- **Lint / format the SDK:** `bun run lint` (oxlint, plus the extension's extractor drift check) / `bun run format` (oxfmt).
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

## op-chrome-extension (`op-chrome-extension/`)

Manifest V3 Chrome extension that captures the **rendered** active tab and
imports it into a running OpenPencil. No dependencies, not a Bun workspace
member — load it unpacked from `chrome://extensions`.

- **The logic is Rust.** `crates/op-chrome-extension-core` (endpoint rules,
  chunked-transfer integrity, `/mcp` envelope + reply classification, download-name
  sanitisation) compiles to wasm; `packages/op-chrome-extension/scripts/build-wasm.sh`
  runs `cargo build --target wasm32-unknown-unknown` + `wasm-bindgen --target web`
  and installs the shim + module into `op-chrome-extension/wasm/`, which is
  **gitignored** like every other wasm-bindgen output in the repo. **Build it
  before "Load unpacked"** — the popup reports an actionable error otherwise.
  Unit-test the logic natively with `cargo test -p op-chrome-extension-core`.
- The JS that remains is glue: `chrome.*` calls, `fetch`, popup DOM, and the
  functions injected into the captured tab (which must be JS — they run in the
  page's process). The manifest declares `'wasm-unsafe-eval'` in
  `content_security_policy.extension_pages`, which is what MV3 requires to
  instantiate a bundled `.wasm`; no code is fetched from outside the package.
- The capture engine is `vendor/snapshot-extractor.js`, a **byte-identical copy**
  of `crates/op-html/assets/snapshot-extractor.js` (the contract
  `op_html::import_snapshot` parses). `bun run lint` runs
  `op-chrome-extension/scripts/check-extractor-sync.sh`, which fails on drift;
  `--fix` re-copies the canonical asset. Never edit the copy.
- The live MCP endpoint exposes two extension-scoped REST capabilities. Snapshot
  import uses `POST /api/import/web-snapshot` (insert-only and open to a
  well-formed `chrome-extension://` origin unless explicitly pinned), with a
  plain `/mcp` `tools/call import_web_snapshot` fallback for unmanaged
  `--serve-web`. Intelligent `design.md` extraction uses a stricter asynchronous
  `POST /api/generate/design-md` + GET/DELETE job flow: it accepts only bounded
  style evidence and queues model work only for an extension id explicitly
  paired through `OPENPENCIL_EXTENSION_ALLOWED_IDS` and a cancellable,
  evidence-only built-in API provider; tool-capable CLI/ACP adapters are not
  eligible. The extension generates a deterministic local guide when pairing
  or compatible model generation is unavailable.
  See `snapshot_ingest.rs` and `design_md_route.rs` under
  `crates/op-host-services/src/mcp_live/`. The extension's offline path is
  **Download .op**, which converts the capture locally into a ready-to-open
  document.
- **Account (optional).** The popup header can sign in to OP Hub through the
  hub's own BFF: a tab on `GET /api/v1/auth/login?return_to=/account`, then
  `GET /api/v1/session` with `credentials: 'include'`. The extension is a
  public client — no SSO secret, no token, no `cookies` permission; the hub's
  `HttpOnly` session cookie stays in the browser's jar. Regions `cn` /
  `global` map to `https://op.zseven.cn` / `https://op.zseven.tech`, both in
  `host_permissions`, and `account.rs` asserts the manifest agrees.
- **Account delivery (optional, signed in).** `POST <hub>/api/v1/snapshots`
  with `credentials: 'include'` + `X-CSRF-Token`, i.e. op-hub's per-user
  snapshot inbox. The envelope, the page-title-derived name, the 32 MiB
  ceiling and the reply classification are Rust (`hub.rs` / `hub_reply.rs`);
  `delivery.rs` remains the single rule that decides local vs account, and an
  expired session collapses to local. The service worker's element-pick flow
  runs its own session probe (`account.js` is in the worker's static graph and
  stays dynamic-import-free). Contract history:
  `op-chrome-extension/docs/hub-inbox-api-proposal.md`.
- **Store packaging.** `bun run package-extension` (or
  `op-chrome-extension/scripts/package-extension.sh`) builds the wasm, runs
  the tests and all four guards, stages only the runtime files, copies `pt` to
  `pt_BR`/`pt_PT` for Chrome's manifest-locale lookup, and writes
  `op-chrome-extension/dist/op-chrome-extension-<version>.zip` (gitignored).
  The listing's privacy policy is `op-chrome-extension/docs/privacy-policy.md`.
