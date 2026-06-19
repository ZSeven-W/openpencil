//! OpenPencil headless web-canvas daemon.
//!
//! The Rust analog of the TS web app's Nitro routes: it owns the canonical
//! `.op` document in memory and serves it (whole-document REST sync + SSE) to
//! the browser WASM shell (`op-host-web`) and to external MCP/CLI clients, plus
//! server-side PNG/screenshot export via skia raster. It links NO winit /
//! glutin / muda / accesskit / skia-GL — the desktop GUI stack stays in
//! `op-host-desktop`.
//!
//! Modules are migrated here from `op-host-desktop` over Phases 2-5 of the
//! op-web-daemon extraction (see
//! `openpencil-docs/superpowers/plans/2026-06-19-op-web-daemon-extraction.md`).
//! Both `op-host-desktop` (for its `--serve-web` mode) and a thin
//! `op-host-web-server` binary depend on this crate.

// Phase 2 — host-free leaf modules.
pub mod web_static;
