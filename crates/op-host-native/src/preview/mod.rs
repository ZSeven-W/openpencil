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
//! handles). The editor state only carries the `preview_mode` flag +
//! warning list (`EditorUiState::{enter,exit}_preview`).
//!
//! ## Render
//!
//! Each preview frame calls `collect_draws_with_widgets` and translates
//! the returned `DrawOp`s onto the OP `RenderBackend` via
//! [`draw_bridge`]. The editor's normal selection/handles do NOT paint
//! in preview.

mod draw_bridge;

#[cfg(test)]
mod tests;

use jian_core::gesture::pointer::Modifiers;
use jian_core::render::scene::{collect_draws_with_widgets, WidgetRenderCtx};
use jian_core::render::widget_style::WidgetTheme;
use jian_core::Runtime;
use jian_ops_schema::compat::{load_str_with, LoadOptions};
use jian_ops_schema::error::LoadWarning;

use op_editor_ui::{Rect, RenderBackend};

/// A live preview runtime built from a snapshot of the editor document.
///
/// Constructed by [`PreviewSession::enter`] from a JSON serialization of
/// the document; dropping it tears down the runtime. The session is
/// `!Send` (it owns an `Rc`-bound `Runtime`), so it is held only on the
/// UI-thread-local host.
pub struct PreviewSession {
    runtime: Runtime,
    theme: WidgetTheme,
    /// Logical canvas size the layout was last computed against. Re-laid
    /// out on resize via [`PreviewSession::resize`].
    canvas_size: (f32, f32),
    /// Non-fatal load warnings (e.g. legacy role promotions), formatted
    /// for display in the editor's `preview_warnings`.
    warnings: Vec<String>,
}

impl PreviewSession {
    /// Build a preview runtime from the document's JSON. `promote=true`
    /// turns legacy role-frames into first-class widget nodes in-memory
    /// (the source doc is untouched). `canvas_size` is the logical
    /// canvas region the runtime lays out into.
    ///
    /// Returns `Err(message)` if serialization, parsing, runtime build,
    /// or layout fails — the host then declines to enter preview and
    /// surfaces the message.
    pub fn enter(
        doc: &jian_ops_schema::PenDocument,
        canvas_size: (f32, f32),
    ) -> Result<Self, String> {
        let src = serde_json::to_string(doc).map_err(|e| format!("serialize document: {e}"))?;
        let loaded = load_str_with(
            &src,
            LoadOptions {
                promote_legacy_widgets: true,
            },
        )
        .map_err(|e| format!("parse document for preview: {e}"))?;

        let warnings = loaded
            .warnings
            .iter()
            .filter_map(format_warning)
            .collect::<Vec<_>>();

        let mut runtime =
            Runtime::new_from_document(loaded.value).map_err(|e| format!("build runtime: {e}"))?;
        let size = (canvas_size.0.max(1.0), canvas_size.1.max(1.0));
        runtime
            .build_layout(size)
            .map_err(|e| format!("compute layout: {e}"))?;
        runtime.rebuild_spatial();

        Ok(Self {
            runtime,
            theme: WidgetTheme::default(),
            canvas_size: size,
            warnings,
        })
    }

    /// The formatted load warnings collected on `enter` (for the
    /// editor's `preview_warnings` diagnostics surface).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Push the host clock so the runtime can drive caret blink etc.
    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.runtime.set_now_ms(now_ms);
    }

    /// Re-lay-out the runtime against a new logical canvas size. Cheap
    /// no-op when the size is unchanged (sub-pixel).
    pub fn resize(&mut self, canvas_size: (f32, f32)) {
        let size = (canvas_size.0.max(1.0), canvas_size.1.max(1.0));
        if (size.0 - self.canvas_size.0).abs() < 0.5 && (size.1 - self.canvas_size.1).abs() < 0.5 {
            return;
        }
        self.canvas_size = size;
        // A layout failure here leaves the previous rects in place — the
        // preview keeps painting the last good frame rather than panicking.
        let _ = self.runtime.build_layout(size);
        self.runtime.rebuild_spatial();
    }

    /// Schema id of the currently focused node (for the render ctx's
    /// focus ring + caret), mapped from the runtime focus chain.
    fn focused_schema_id(&self) -> Option<String> {
        let key = self.runtime.focus.current()?;
        let doc = self.runtime.document.as_ref()?;
        let node = doc.tree.nodes.get(key)?;
        Some(jian_core::document::tree::node_schema_id(&node.schema).to_owned())
    }

    /// Collect this frame's draw ops from the live runtime.
    fn draw_ops(&self, now_ms: u64) -> Vec<jian_core::render::DrawOp> {
        let Some(doc) = self.runtime.document.as_ref() else {
            return Vec::new();
        };
        let focused = self.focused_schema_id();
        let ctx = WidgetRenderCtx {
            states: &self.runtime.widget_states,
            theme: &self.theme,
            focused_id: focused.as_deref(),
            now_ms,
        };
        collect_draws_with_widgets(doc, &self.runtime.layout, &self.runtime.state, &ctx)
    }

    /// Paint the live preview scene into `backend`, transformed by the
    /// editor viewport so it lands in the same place the design surface
    /// drew (canvas origin + pan, scaled by zoom). `canvas_region` is
    /// the screen-space rect of the canvas (clip target); `pan` / `zoom`
    /// come from the editor viewport.
    pub fn paint(
        &self,
        backend: &mut dyn RenderBackend,
        canvas_region: Rect,
        pan: (f32, f32),
        zoom: f32,
        now_ms: u64,
    ) {
        let ops = self.draw_ops(now_ms);
        backend.save();
        backend.clip_rect(canvas_region);
        // screen = canvas_origin + pan + doc * zoom
        backend.translate(op_editor_ui::Point2D::new(
            canvas_region.origin.x + pan.0,
            canvas_region.origin.y + pan.1,
        ));
        backend.scale(
            op_editor_ui::Point2D::new(zoom, zoom),
            op_editor_ui::Point2D::new(0.0, 0.0),
        );
        for op in &ops {
            draw_bridge::paint_draw_op(backend, op);
        }
        backend.restore();
    }

    // --- Input dispatch -------------------------------------------

    /// Route a printable character into the focused widget. Returns
    /// `true` when the runtime consumed it (a focused editable widget
    /// accepted the text).
    pub fn dispatch_text(&mut self, text: &str) -> bool {
        self.runtime.dispatch_text_input(text)
    }

    /// Route a named key (e.g. `"Backspace"`, `"ArrowLeft"`, `"Enter"`,
    /// `"Tab"`) into the runtime with the given modifier set. Returns
    /// `true` when the dispatch emitted any semantic event.
    pub fn dispatch_key(&mut self, key: &str, modifiers: Modifiers) -> bool {
        !self
            .runtime
            .dispatch_keyboard(key.to_string(), modifiers)
            .is_empty()
    }

    /// Dispatch a tap (Down then Up) at a DOCUMENT-space point into the
    /// runtime so clicks land on switches / buttons / and place caret /
    /// focus in text inputs. The host converts the screen press to doc
    /// space (via the editor viewport) before calling this. Returns
    /// `true` when the runtime emitted any semantic event.
    pub fn dispatch_tap(&mut self, doc_x: f32, doc_y: f32) -> bool {
        use jian_core::geometry::point;
        use jian_core::gesture::pointer::{PointerEvent, PointerPhase};
        let down = PointerEvent::simple(1, PointerPhase::Down, point(doc_x, doc_y));
        let mut emitted = self.runtime.dispatch_pointer(down);
        let up = PointerEvent::simple(1, PointerPhase::Up, point(doc_x, doc_y));
        emitted.extend(self.runtime.dispatch_pointer(up));
        !emitted.is_empty()
    }

    /// Advance focus to the next focusable widget (Tab).
    pub fn focus_next(&mut self) {
        self.runtime.focus_next();
    }

    /// Advance focus to the previous focusable widget (Shift+Tab).
    pub fn focus_previous(&mut self) {
        self.runtime.focus_previous();
    }

    /// Test-only read access to the live state graph + widget state so
    /// the host test can assert injected text reached the runtime.
    #[cfg(test)]
    pub(crate) fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}

/// Format a load warning for the editor's diagnostics surface. Only
/// surfaces the actionable ones (legacy promotions, future versions,
/// skipped logic) — generic unknown-field noise is dropped.
fn format_warning(w: &LoadWarning) -> Option<String> {
    match w {
        LoadWarning::LegacyRolePromoted {
            path,
            from_role,
            to,
        } => Some(format!(
            "LegacyRolePromoted: '{path}' role '{from_role}' → {to}"
        )),
        LoadWarning::FutureFormatVersion {
            found,
            supported_max,
        } => Some(format!(
            "FutureFormatVersion: {found} (supported ≤ {supported_max})"
        )),
        LoadWarning::LogicModulesSkipped { reason } => {
            Some(format!("LogicModulesSkipped: {reason}"))
        }
        LoadWarning::InvalidExpression { path, reason, .. } => {
            Some(format!("InvalidExpression: '{path}': {reason}"))
        }
        LoadWarning::UnknownField { .. } => None,
    }
}
