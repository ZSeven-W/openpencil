//! OpenPencil headless host services — the GUI-free host backend.
//!
//! Everything the host does that doesn't need a window: the in-memory
//! `.op` document daemon (whole-document REST sync + SSE for the browser
//! `op-host-web` shell + external MCP/CLI clients), the stdio + HTTP MCP
//! servers, the AI chat providers + design orchestration, server-side
//! PNG/SVG/PDF export (skia raster), document/settings persistence, and
//! model/provider discovery. It links NO winit / glutin / muda /
//! accesskit adapters / skia-GL — the desktop GUI stack stays in
//! `op-host-native` / `op-host-desktop`.
//!
//! Both hosts depend on this crate: `op-host-desktop` embeds it for its
//! own `--serve-web` / MCP / chat / export, and the thin
//! `op-host-web-server` binary is just an argv dispatcher over it.
//!
//! Modules were migrated here out of `op-host-desktop` over Phases 2–5 of
//! the extraction (plan:
//! `openpencil-docs/superpowers/plans/2026-06-19-op-web-daemon-extraction.md`).

// Migrated headless modules (Phases 2-5), kept alphabetical.
pub mod acp_agent_probe_host;
pub mod ai_proxy;
mod chat_agent_context;
pub mod chat_agent_loop;
pub mod chat_attachment;
#[cfg(test)]
mod chat_avatar_modify_tests;
pub mod chat_builtin_http;
pub mod chat_canvas_tools;
pub mod chat_claude;
pub mod chat_copilot;
pub mod chat_grok_stream;
pub mod chat_http_server;
pub mod chat_intent;
mod chat_modify_sanitize;
pub mod chat_provider_llm;
pub mod chat_runtime;
pub mod chat_spawn;
pub mod chat_subprocess;
mod chat_subprocess_parse;
pub mod chat_subprocess_quirks;
mod chat_subprocess_safety;
pub mod chat_system_prompt;
pub mod cli_model_discovery;
pub mod cli_provider_probe;
mod cli_resolver_windows;
mod design_agent_diagnostics;
#[cfg(test)]
mod design_agent_reflow_tests;
pub mod design_agent_tools;
pub mod design_context;
pub mod design_md_llm;
pub mod design_session;
pub mod doc_io;
pub mod export;
pub mod export_pdf;
pub mod mcp_live;
pub mod mcp_serve;
pub mod model_discovery;
mod model_probe;
pub mod pre_validator;
pub mod provider_probe;
pub mod provider_probe_host;
pub mod provider_probe_models;
pub mod settings_io;
pub mod validation_providers;
pub mod web_canvas_server;
pub mod web_chat_standard;
pub mod web_credential_policy;
pub mod web_credentials;
pub mod web_static;
pub mod zode_import;
