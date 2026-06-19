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

// Migrated headless modules (Phases 2-5), kept alphabetical.
pub mod acp_agent_probe_host;
pub mod ai_proxy;
pub mod chat_agent_loop;
pub mod chat_attachment;
pub mod chat_builtin_http;
pub mod chat_canvas_tools;
pub mod chat_claude;
pub mod chat_copilot;
pub mod chat_http_server;
pub mod chat_intent;
pub mod chat_provider_llm;
pub mod chat_runtime;
pub mod chat_spawn;
pub mod chat_subprocess;
pub mod chat_subprocess_quirks;
pub mod chat_system_prompt;
pub mod design_session;
pub mod doc_io;
pub mod export;
pub mod export_pdf;
pub mod mcp_live;
pub mod mcp_serve;
pub mod model_discovery;
pub mod pre_validator;
pub mod provider_probe;
pub mod provider_probe_host;
pub mod provider_probe_models;
pub mod settings_io;
pub mod web_canvas_server;
pub mod web_chat_standard;
pub mod web_static;
