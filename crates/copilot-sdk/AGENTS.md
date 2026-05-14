# Repository Guidelines

## Project Structure

- `src/`: library implementation (`client`, `session`, `transport`, `jsonrpc`, `types`).
- `examples/`: runnable samples (e.g. `cargo run --example basic_chat`).
- `tests/`: integration tests:
  - `e2e_tests.rs` (feature-gated; requires a real Copilot CLI)
  - `snapshot_conformance.rs` (feature-gated; optional upstream snapshot suite)
- `.github/workflows/ci.yml`: CI for fmt/clippy/test/doc/package.

## Build, Test, and Development Commands

- `cargo build`: compile the crate.
- `cargo test`: run unit + integration tests (feature-gated suites are skipped by default).
- `cargo fmt --all`: format code (`cargo fmt --all -- --check` in CI).
- `cargo clippy --all-targets --all-features -- -D warnings`: lint (treat warnings as errors).
- `cargo doc --no-deps`: build docs locally.

Feature-gated tests:

- `cargo test --features e2e -- --test-threads=1`: E2E tests against the installed/authenticated Copilot CLI.
- `cargo test --features snapshots --test snapshot_conformance`: snapshot conformance (set `COPILOT_SDK_RUST_SNAPSHOT_DIR` or `UPSTREAM_SNAPSHOTS` if auto-detect fails).

## Coding Style & Naming

- Rust `1.85.0` is pinned in `rust-toolchain.toml` (Edition 2024).
- Format with `rustfmt`; lint with `clippy`.
- Keep `unsafe` out: the crate uses `#![forbid(unsafe_code)]`.
- Maintain attribution headers at the top of Rust files (`tests/attribution_headers.rs` enforces this).

## Commit & Pull Request Guidelines

- Commit convention is not established in this repo’s current history; use Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`) and keep commits focused.
- PRs should include: what/why, how to test (commands), and docs/example updates for API or behavior changes.

## Security & Configuration Tips

- The SDK interacts with the Copilot CLI runtime; avoid logging secrets, prompts, or tokens.
- Ensure `copilot` is on `PATH`, or set `COPILOT_CLI_PATH` to the CLI executable/script.
