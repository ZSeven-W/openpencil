//! OpenPencil composition root.
//!
//! Phase 7.3 strangler reorg — `op-app` is the thin crate that names
//! the editor application's composition. It exists so the workspace
//! has one obvious "this is the app" entry point rather than two
//! free-standing host crates.
//!
//! ## What composes the app
//!
//! The editor-UI composition — which widgets exist, how they lay out,
//! the theme, the layout-resolved render scene — already lives in the
//! [`op_editor_ui`] crate. The canonical editor state lives in
//! `op-editor-core`; the `.op` loader in `op-pen-loader`. The two
//! platform hosts wire that shared composition to a concrete backend:
//!
//! - **`op-host-native`** — winit + skia-safe + accesskit GL host
//!   (desktop + mobile). The desktop binary is `op-host-desktop`.
//! - **`op-host-web`** — the wasm32-unknown-unknown browser bundle
//!   (`WebShell` + `mount()`).
//!
//! Because that composition is already factored into `op-editor-ui`,
//! there is no shared host *bootstrap* code left to extract — each
//! host owns only platform-specific backend wiring. So `op-app` is
//! deliberately thin (YAGNI): it re-exports the per-platform host
//! entry point behind its target `cfg` and serves as the documented
//! composition root. If genuine cross-host wiring later emerges, it
//! lands here.

/// The platform-agnostic editor-UI composition (widgets, theme,
/// layout scene, render-backend facade, gesture types).
pub use op_editor_ui as editor_ui;

/// The native host — winit + skia-safe + accesskit (desktop + mobile).
///
/// Re-exported on every non-wasm target. The shipped desktop
/// executable lives in the `op-host-desktop` crate, which drives
/// [`host_native::WidgetHostNative`] from a winit event loop.
#[cfg(not(target_arch = "wasm32"))]
pub use op_host_native as host_native;

/// The web host — the wasm32-unknown-unknown browser bundle entry.
///
/// Re-exported only on the wasm32 target, where `op-host-web`'s
/// `cdylib` / wasm-bindgen setup applies. `host_web::mount` is the
/// JS-facing entry point.
#[cfg(target_arch = "wasm32")]
pub use op_host_web as host_web;
