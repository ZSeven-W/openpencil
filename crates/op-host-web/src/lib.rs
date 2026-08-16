//! OpenPencil web host — web bundle entry.
//!
//! Two build configurations:
//! * `canvaskit` — the production web shell: renders the full editor through
//!   the official CanvasKit skia WASM (GPU) via `mount_ck` (`canvaskit.rs`).
//!   All widget / chat / codegen / file-IO / live-sync logic is shared Rust.
//! * `web` (default) — a wasm32-unknown-unknown-clean stub (no skia, no
//!   CanvasKit) that only compile-checks the public surface for CI; `mount`
//!   below returns a fields-less `WebShell` and never paints.
//!
//! (The from-scratch skia raster WebBackend + its `skia` / `codegen` /
//! `live-sync` features + the WebGL2 Ganesh backend were retired 2026-06-17 —
//! CanvasKit is the only web renderer now, and `skia-safe` reverted to upstream
//! crates.io, which has no wasm32-unknown-unknown target.)

// Hidden ARIA DOM mirror (#57). Pure accesskit + web_sys — no skia, no
// `op-editor-core` — so it compile-checks under BOTH the production
// `canvaskit` build (where the mount wires it) and the wasm32-clean `web`
// stub baseline (compile coverage only).
mod a11y_dom;
#[cfg(feature = "canvaskit")]
pub mod canvaskit;
pub mod event;
// Hidden IME-capture input (#54). Pure web_sys — compiles under BOTH the
// production `canvaskit` build (where the mount wires composition→apply_ime)
// and the `web` stub baseline (compile coverage only).
#[cfg(any(feature = "canvaskit", test))]
mod image_decode_queue;
mod ime_input;
#[cfg(feature = "canvaskit")]
mod listener;
#[cfg(feature = "canvaskit")]
mod widget_host;
// Backend-agnostic seam: lets the chat / live-sync / codegen / file-IO modules
// drive the CanvasKit `CkInner` through one trait.
#[cfg(feature = "canvaskit")]
mod repaint_ctx;
// Pure web_sys IO (no native Skia/C toolchain) — compiled always so the `web`
// stub still compile-checks on wasm32.
mod live_sync;
// Daemon → browser agent-indicator relay (poll + local mirror + rAF pump).
#[cfg(feature = "canvaskit")]
mod agent_indicator_sync;
// Top-bar hover-tooltip dwell → rAF repaint (the web host has no
// animation-deadline scheduler of its own).
#[cfg(feature = "canvaskit")]
mod tooltip_pump;
// Daemon device-login relay (action drain + login-status poll + popup).
#[cfg(feature = "canvaskit")]
mod live_sync_glue;
// The recovery stash behind an online auto-accept.
#[cfg(feature = "canvaskit")]
mod live_sync_recovery;
#[cfg(feature = "canvaskit")]
mod web_auth_sync;
// Opens the hub portal's per-account MCP-token page in a new tab from the
// signed-in account dropdown (online/hub-served web only). Only the
// canvaskit widget host calls it, so it shares that gate.
#[cfg(feature = "canvaskit")]
mod web_mcp_tokens;
// Daemon collaboration relay (action drain + projection pull + presence).
#[cfg(feature = "canvaskit")]
mod collab_sync;
// Shared daemon base-URL resolution (page origin when served by the daemon,
// localhost fallback for the dev smoke page).
#[cfg(feature = "canvaskit")]
mod codegen_bundle;
#[cfg(feature = "canvaskit")]
mod codegen_web;
mod daemon_base;
#[cfg(feature = "canvaskit")]
mod document_json;
#[cfg(feature = "canvaskit")]
mod dom_io;
#[cfg(feature = "canvaskit")]
mod file_actions;
// Which account this tab belongs to. Lives with the daemon-facing modules
// because it is driven by the device-login status poll.
#[cfg(feature = "canvaskit")]
pub mod identity_epoch;
#[cfg(all(test, feature = "canvaskit"))]
mod prompt_center_file_actions_tests;
// Short-lived Worker-side Figma converter. The exported class is instantiated
// only in a module Worker by `figma_temp_worker.js`; the main editor never
// retains it.
#[cfg(feature = "canvaskit")]
mod figma_temp_writer;
#[cfg(feature = "canvaskit")]
pub use figma_temp_writer::FigmaTempWriter;
#[cfg(feature = "canvaskit")]
mod figma_temp_bridge;
#[cfg(feature = "canvaskit")]
mod raf_pump;
#[cfg(feature = "canvaskit")]
mod repaint_coalescer;
#[cfg(feature = "canvaskit")]
mod theme_preset_io;
#[cfg(feature = "canvaskit")]
mod web_ai_credentials;
#[cfg(feature = "canvaskit")]
mod web_ai_transport;
#[cfg(feature = "canvaskit")]
mod web_builtin_model_discovery;
// Web Iconify bridge — drains the icon picker's remote-search request directly
// against api.iconify.design (CORS-open, same as TS).
#[cfg(feature = "canvaskit")]
mod iconify_web;
// Runtime fetch for the product assets the wasm bundle omits.
#[cfg(feature = "canvaskit")]
mod web_asset_fetch;
// Runtime fetch for collaboration-peer avatars via the daemon proxy.
#[cfg(feature = "canvaskit")]
mod collab_avatar_fetch;
// Web chat session — drains `chat.pending_send` / Stop / New Chat and streams
// real standard-mode turns through the daemon's `/api/ai/standard` route.
#[cfg(feature = "canvaskit")]
mod web_chat;
#[cfg(feature = "canvaskit")]
mod web_credential_sync;
#[cfg(feature = "canvaskit")]
mod web_design_md;
// Web image-panel drain — Search / Generate popover network via the daemon's
// `/api/ai/image/*` routes (desktop `image_panel_host` counterpart).
#[cfg(feature = "canvaskit")]
mod web_image_panel;
#[cfg(feature = "canvaskit")]
mod web_model_catalog;
#[cfg(feature = "canvaskit")]
mod web_settings;
// The Styles tab's DESIGN.md file pick — the browser half of the import the
// paste box shares (desktop `style_import_host` counterpart).
#[cfg(feature = "canvaskit")]
mod web_style_import;
// The daemon ACP route intentionally remains unavailable in production.
// Compile the dormant browser bridge in CanvasKit tests so its request
// generation contract cannot silently drift from the shared core state.
#[cfg(all(test, feature = "canvaskit"))]
#[allow(dead_code)]
mod web_acp_connect;
// postMessage bridge to the VS Code extension host (token bootstrap, document
// open, snapshot, save-committed, conflict resolution). DOM wiring only — the
// wire codec lives in `op_editor_core::bridge_protocol`.
#[cfg(feature = "canvaskit")]
mod vscode_bridge;
#[cfg(feature = "canvaskit")]
mod web_storage;
// Pure web_sys clipboard/download — Ctrl+C/X in inputs + Figma/file paste.
#[cfg(feature = "canvaskit")]
mod web_clipboard;
#[cfg(feature = "canvaskit")]
mod web_fonts;
// IndexedDB persistence for user-imported fonts (Phase 4).
#[cfg(feature = "canvaskit")]
mod font_store_idb;
// Pure-Rust font-family extraction for imported fonts (the vendored CanvasKit
// build has no family-name introspection).
#[cfg(feature = "canvaskit")]
mod font_meta;

#[cfg(not(feature = "canvaskit"))]
use wasm_bindgen::prelude::*;

/// Fields-less stub shell for the `web` compile-guard build. The production
/// CanvasKit build uses `canvaskit::mount_ck` instead and never constructs it.
#[cfg(not(feature = "canvaskit"))]
#[wasm_bindgen]
pub struct WebShell {}

/// Stub mount used by the kickoff §1.2 wasm32-clean compile guard CI.
/// Returns a fields-less `WebShell` after verifying the host has a canvas with
/// the given id; never paints. The production renderer is `mount_ck`
/// (`canvaskit` feature, `canvaskit.rs`).
#[cfg(not(feature = "canvaskit"))]
#[wasm_bindgen]
pub fn mount(canvas_id: &str) -> Result<WebShell, JsValue> {
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;

    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("mount: window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("mount: document unavailable"))?;
    let element = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("mount: canvas '{canvas_id}' not found")))?;
    let _canvas = element
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("mount: target element is not <canvas>"))?;

    Ok(WebShell {})
}
