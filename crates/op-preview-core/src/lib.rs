//! Canvas Preview (Play) mode — runtime owner (Phase D5).
//!
//! When the editor enters Preview mode, the current document is run
//! through the jian `Runtime` so widget nodes become live + interactive
//! (typing into inputs, toggling switches, caret blink, focus chain).
//! The runtime is built from the document's serialized form, so the
//! saved `PenDocument` is NEVER mutated: enter clones via JSON, exit
//! drops the runtime, and the editor's `doc` is byte-identical.
//!
//! ## Why this lives host-side, not in `op-editor-core`
//!
//! `jian_core::Runtime` holds `Rc<...>` (scheduler / state graph), so it
//! is `!Send`. `op-editor-core` must stay wasm32-clean + does not hold
//! the runtime. The session therefore lives on the native host
//! (`WidgetHostNative`), which is already UI-thread-local (it owns skia
//! handles). The editor state only carries the `preview.mode` flag +
//! warning list (`EditorUiState::{enter,exit}_preview`).
//!
//! ## Render — reuse the design-canvas renderer
//!
//! Preview does NOT paint through jian's `collect_draws_with_widgets`
//! MVP scene walker (which scatters multi-root docs to the origin,
//! greys out every image, and re-implements text metrics). Instead it
//! renders the live document through the SAME mature painter the design
//! canvas uses: the host hands in the design `LayoutScene`, we overlay
//! each interactive widget's LIVE runtime value (typed text / toggle /
//! slider / select) onto the scene's `SceneWidget`s, and paint it with
//! [`op_editor_ui::widgets::paint_scene_page`] — pixel-identical to the
//! design surface (root offsets, images, gradients, shadows, real glyph
//! metrics), plus a focus caret drawn on top. The editor's normal
//! selection / handles / grid do NOT paint in preview.
//!
//! R6's typed binding overlay resolves content, widget values, visibility,
//! paint, transforms, geometry, and structural state without mutating the
//! authored document. Jian's canonical invalidation classifier orders the
//! resulting work as PaintOnly, HitTest, Relayout, or Navigation. The
//! read-only `$scroll` namespace drives PaintOnly bindings and sticky/pinned
//! child geometry through the same overlay.
//!
//! ## Hit-testing across two coordinate spaces
//!
//! The scene paints DESIGN-canvas geometry while the runtime hit-tests
//! its own (promoted, re-solved) layout, so a tap arriving in SCENE
//! space maps through the rect pair of the deepest painted node it hit,
//! with a per-gesture anchor (pointer capture). The whole pipeline
//! lives in `input.rs` — see its module docs.
//!
//! ## Module split
//!
//! To honor the 800-line-per-file cap, [`PreviewSession`] is a spine in
//! `session.rs` (struct + retained fields + `enter` + accessors) with
//! the scene overlay/paint impls in the sibling `session_paint.rs`;
//! [`AppMode`] + the per-root `solve_roots` + the app-mode query methods
//! live in `app_mode.rs`, keyboard/focus/pointer dispatch + the
//! scene→runtime coordinate mapping live in `input.rs`, and the leaf
//! formatter helpers (`apply_widget_state` / `display_string` /
//! `format_warning`) live in `scene_helpers.rs`. The crate root keeps
//! only module declarations and stable re-exports.

mod app_mode;
#[cfg(feature = "gl-host")]
mod auto_wire;
#[cfg(not(feature = "gl-host"))]
mod auto_wire_stub;
mod binding_overlay;
mod binding_sites;
pub mod device_frame;
#[cfg(all(test, not(target_os = "windows")))]
mod device_frame_tests;
mod effects;
mod error;
mod input;
mod input_event;
mod interaction_state;
mod invalidation;
mod mode_transition;
mod present;
mod scene_helpers;
mod session;
mod session_paint;
mod transition;
mod ui_actions;

/// R3 effect queue — the bounded FIFO between the engine's effect sink
/// and the host.
pub use effects::PreviewEffectQueue;
/// R4 Canonical PreviewInput — the unified input envelope, dispatch
/// outcome, and lifecycle phases behind
/// [`PreviewSession::dispatch_input`].
pub use input_event::{
    AppLifecyclePhase, BackSource, PageLifecyclePhase, PreviewDispatchOutcome, PreviewInput,
    PreviewInputEnvelope, PreviewLifecycle, ScrollPhase,
};
/// R4 interaction state — per-pointer pressed + hover node tracking.
pub use interaction_state::InteractionState;
pub use invalidation::InvalidationKind;
/// Frozen Preview contract DTOs, re-exported so consumers of this crate
/// (UI, FFI, hosts) need not take the leaf dependency directly.
pub use op_preview_contracts::{
    EffectSource, HapticStyle, PreviewCapability, PreviewEffect, PreviewEffectFailure,
    PreviewEffectFailureCode, PreviewEffectResult, PreviewHostCapabilities, SharePayload,
    UserActivationId,
};

/// Typed failure domains for entering / re-solving a preview session
/// (`PreviewSession::enter` + `app_mode::solve_roots`).
pub use error::{PreviewEnterError, PreviewLayoutError};
/// Screen transition animation state and helpers — used by the host integration layer.
#[allow(unused_imports)]
pub use mode_transition::{lerp_color, ModeTransition, ModeTransitionKind};
/// Pinned paint tracking for caret animation — used by the host integration layer.
#[allow(unused_imports)]
pub use present::PinnedPaint;
/// The live preview runtime session — constructed by
/// [`PreviewSession::enter`] from a snapshot of the editor document.
pub use session::PreviewSession;

/// Internal re-export: `input` overlays live widget state through the
/// shared scene formatter helper.
pub(crate) use scene_helpers::apply_widget_state;
/// Internal re-export: `app_mode`'s per-root layout solve constructs
/// [`crate::session::RootFrame`]s and reads the first frame's scene rect.
pub(crate) use session::RootFrame;

/// Test-only layout backend + font-registry lock, shared by every
/// preview test module.
#[cfg(all(test, not(target_os = "windows")))]
pub(crate) use session::{font_registry_test_support, test_measure};

// Gated off Windows: preview tests exercise runtime layout through
// `jian_skia::SkiaMeasure`, which hits DirectWrite in Windows CI and aborts
// with STATUS_ACCESS_VIOLATION before Rust can report a normal failure.
// macOS + Linux keep the full preview coverage.
#[cfg(all(test, not(target_os = "windows")))]
mod tests;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_app_mode;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_binding_overlay;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_bindings;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_caret;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_clock_gate;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_device_frame;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_effects;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_geometry_parity;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_input_trace;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_interaction;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_interaction_state;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_multi_pointer;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_swipe;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_tabs;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_transition;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_ui_actions;
// Same platform gate as `tests_app_mode` / `tests_transition`, whose
// fixture and helpers this file reuses: those are excluded on Windows,
// so a Windows build cannot see `test_measure`, `TWO_SCREEN_DOC_JSON`
// or `node_rect` either.
#[cfg(all(test, not(target_os = "windows")))]
mod tests_transition_input;
