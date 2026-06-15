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
//! ## Hit-testing across two coordinate spaces
//!
//! The design scene offsets every page-root by its authored
//! `(base.x, base.y)`, but the jian runtime lays each root at its own
//! `(0, 0)`. So a tap arrives in SCENE space (it inverts the scene paint
//! transform) and must be translated back into the runtime's
//! root-relative space — subtract the containing root's authored origin
//! — before [`Runtime::dispatch_pointer`]. See [`PreviewSession::dispatch_tap`].

#[cfg(test)]
mod tests;

use jian_core::gesture::pointer::Modifiers;
use jian_core::widget_state::WidgetState;
use jian_core::Runtime;
use jian_ops_schema::compat::{load_str_with, LoadOptions};
use jian_ops_schema::error::LoadWarning;

use op_editor_ui::layout_scene::{LayoutScene, SceneNode, SceneWidget};
use op_editor_ui::widgets::{paint_scene_page, PaintCx};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend};

/// One page-root's mapping between the design scene's coordinate space
/// (root offset baked in) and the jian runtime's root-relative
/// hit-test space. Used to translate a scene-space tap back into the
/// space `Runtime::dispatch_pointer` expects.
struct RootFrame {
    /// The root's bounds in SCENE space (authored origin + size).
    scene_rect: Rect,
    /// The root's authored `(base.x, base.y)` — the delta between scene
    /// space and the runtime's root-relative space.
    offset: (f32, f32),
}

/// A live preview runtime built from a snapshot of the editor document.
///
/// Constructed by [`PreviewSession::enter`] from a JSON serialization of
/// the document; dropping it tears down the runtime. The session is
/// `!Send` (it owns an `Rc`-bound `Runtime`), so it is held only on the
/// UI-thread-local host.
pub struct PreviewSession {
    runtime: Runtime,
    /// The available size the runtime's PRIMARY (first) root was laid
    /// out against (the root's authored size). Read only by the
    /// layout-parity test; retained as the record of what the runtime
    /// solved against.
    #[cfg_attr(not(test), allow(dead_code))]
    available: (f32, f32),
    /// Per-root scene↔runtime coordinate mapping for tap translation.
    root_frames: Vec<RootFrame>,
    /// The design `LayoutScene` preview paints, built from the SAME
    /// prepared + PROMOTED document the runtime was seeded from (so a
    /// generated/legacy `role=input` field renders as an interactive
    /// `text_input` widget, not a frame). Live widget values are
    /// overlaid onto a clone of this each frame in `paint_scene`.
    scene: LayoutScene,
    /// Non-fatal load warnings (e.g. legacy role promotions), formatted
    /// for display in the editor's `preview_warnings`.
    warnings: Vec<String>,
}

impl PreviewSession {
    /// Build a preview runtime from the document's JSON. `promote=true`
    /// turns legacy role-frames into first-class widget nodes in-memory
    /// (the source doc is untouched). `canvas_size` is the editor canvas
    /// region; it is retained for API compatibility but does NOT drive
    /// the runtime layout (layout is per-root from the document).
    ///
    /// ## Layout
    ///
    /// The runtime layout is solved per page-root against that root's
    /// OWN authored size (`root_available_size`) with real skia text
    /// metrics (`jian_skia::SkiaMeasure`) — the SAME backend the design
    /// canvas uses. The runtime layout now serves only hit-testing (the
    /// visible scene is painted by the host through the design
    /// `LayoutScene`); keeping it bit-identical to the design canvas
    /// keeps taps landing where widgets paint.
    ///
    /// ## Active theme
    ///
    /// `$token` refs are resolved against the editor's currently-active
    /// theme (`active_theme`) before the runtime is built, so the live
    /// state graph (e.g. seeded input values) reflects the same theme
    /// the design canvas paints.
    ///
    /// Returns `Err(message)` if serialization, parsing, runtime build,
    /// or layout fails — the host then declines to enter preview and
    /// surfaces the message.
    pub fn enter(
        doc: &jian_ops_schema::PenDocument,
        canvas_size: (f32, f32),
        active_theme: &std::collections::BTreeMap<String, String>,
        active_page_index: usize,
    ) -> Result<Self, String> {
        let _ = canvas_size; // layout is root-derived, not canvas-derived.

        // Prepare the document EXACTLY as the design canvas does before
        // it lays out + paints (`op_pen_loader::layout_scene::
        // editor_state_to_layout_scene`): expand component (`RefNode`)
        // instances FIRST so instance subtrees also resolve, then land
        // every `$token` against the editor's ACTIVE theme. Both passes
        // early-out via cheap detector walks, so a ref-free / token-free
        // document pays no clone. Keeping this identical to the design
        // canvas means the runtime tree (ids, seeded widget values) and
        // the painted scene agree node-for-node.
        let mut prepared = std::borrow::Cow::Borrowed(doc);
        if op_editor_core::ref_resolve::document_has_refs(&prepared) {
            prepared = std::borrow::Cow::Owned(
                op_editor_core::ref_resolve::resolve_refs_for_canvas(&prepared),
            );
        }
        if op_editor_core::variables_resolve::document_has_tokens(&prepared) {
            prepared = std::borrow::Cow::Owned(
                op_editor_core::variables_resolve::resolve_document_for_canvas(
                    &prepared,
                    active_theme,
                ),
            );
        }

        // Project the editor's ACTIVE page so the runtime's roots match
        // the page the design scene paints. jian's loader always takes
        // `pages[0]` as roots (vendor/jian `document/loader.rs`), so a
        // multi-page doc previewed on page N>0 would otherwise hit-test
        // / seed widget state against page 0 while the scene paints page
        // N. Slicing the active page's children to the top level (and
        // clearing `pages`) makes the loader use them. This fixes
        // ENTERING preview on any page; switching pages WHILE in preview
        // needs the host to re-enter (the runtime is built once here).
        if prepared
            .pages
            .as_ref()
            .is_some_and(|pages| !pages.is_empty())
        {
            let mut owned = prepared.into_owned();
            if let Some(pages) = owned.pages.take() {
                let idx = active_page_index.min(pages.len().saturating_sub(1));
                if let Some(page) = pages.into_iter().nth(idx) {
                    owned.children = page.children;
                }
            }
            prepared = std::borrow::Cow::Owned(owned);
        }

        let src =
            serde_json::to_string(&*prepared).map_err(|e| format!("serialize document: {e}"))?;

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

        // Clone the prepared + promoted document BEFORE the runtime
        // consumes it: this is the exact tree (refs/tokens resolved,
        // active page projected, legacy role-frames promoted) we render
        // the design scene from, so the painted scene and the runtime's
        // hit-test/state graph agree node-for-node.
        let promoted_doc = loaded.value.clone();
        let mut runtime =
            Runtime::new_from_document(loaded.value).map_err(|e| format!("build runtime: {e}"))?;

        // Per-root layout, mirroring `op_pen_loader::compute_layout`:
        // install the real skia paragraph shaper (so `fit_content` text
        // frames hit-test against the glyph advances paint draws), then
        // `compute` EACH root against its OWN authored available size.
        // `Runtime::build_layout` would lay every root against a single
        // size, diverging from the design canvas. The returned taffy
        // NodeIds are positional with `doc.tree.roots` (see
        // `LayoutEngine::build`), so we zip them to pair each root with
        // the id `compute` needs.
        runtime
            .layout
            .set_backend(std::rc::Rc::new(jian_skia::SkiaMeasure::new()));
        let primary_available = {
            let Some(rt_doc) = runtime.document.as_ref() else {
                return Err("preview runtime has no document".to_string());
            };
            let root_keys = rt_doc.tree.roots.clone();
            let taffy_roots = runtime
                .layout
                .build(&rt_doc.tree)
                .map_err(|e| format!("build layout tree: {e}"))?;
            // `build` never clears `runtime.document`; surface an error
            // rather than panic to keep the no-panic contract.
            let Some(rt_doc) = runtime.document.as_ref() else {
                return Err("preview runtime document vanished after layout build".to_string());
            };
            let mut primary: Option<(f32, f32)> = None;
            for (root_key, taffy_root) in root_keys.iter().zip(taffy_roots.iter()) {
                let per_root = rt_doc
                    .tree
                    .nodes
                    .get(*root_key)
                    .map(|node_data| op_pen_loader::root_available_size(&node_data.schema))
                    .unwrap_or((1440.0, 900.0));
                if primary.is_none() {
                    primary = Some(per_root);
                }
                runtime
                    .layout
                    .compute(*taffy_root, per_root)
                    .map_err(|e| format!("compute layout: {e}"))?;
            }
            primary.unwrap_or((1440.0, 900.0))
        };
        runtime.rebuild_spatial();

        // Capture each root's scene↔runtime coordinate mapping for tap
        // translation. The design scene offsets every root by its
        // authored `(base.x, base.y)`; the runtime lays each at its own
        // origin. `runtime.document` + `runtime.layout` are disjoint
        // fields, so the two immutable borrows below co-exist.
        let root_frames = {
            let mut frames = Vec::new();
            if let Some(rt_doc) = runtime.document.as_ref() {
                for root_key in rt_doc.tree.roots.iter() {
                    let Some(node_data) = rt_doc.tree.nodes.get(*root_key) else {
                        continue;
                    };
                    let offset = op_pen_loader::root_authored_origin(&node_data.schema);
                    let rrect = runtime.layout.node_rect(*root_key);
                    let (rx, ry, rw, rh) = rrect
                        .map(|r| (r.origin.x, r.origin.y, r.size.width, r.size.height))
                        .unwrap_or((0.0, 0.0, 0.0, 0.0));
                    frames.push(RootFrame {
                        scene_rect: Rect {
                            origin: Point2D::new(offset.0 + rx, offset.1 + ry),
                            size: Point2D::new(rw, rh),
                        },
                        offset,
                    });
                }
            }
            frames
        };

        // Build the design scene from the promoted document. The active
        // page was projected to the top-level `children` in `enter`, so
        // it is page index 0. Refs/tokens are already resolved in
        // `promoted_doc`, so the builder's detector walks early-out.
        let scene = op_pen_loader::pen_document_to_layout_scene(&promoted_doc, active_theme, 0);

        Ok(Self {
            runtime,
            available: primary_available,
            root_frames,
            scene,
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

    /// Resize hook for the host's `Resized` handler. Layout is derived
    /// per-root from the document, independent of the editor canvas
    /// region, so this is a no-op; the parameter is kept for API
    /// compatibility with the host call site.
    pub fn resize(&mut self, canvas_size: (f32, f32)) {
        let _ = canvas_size;
    }

    // --- Render ----------------------------------------------------

    /// Paint the live preview by rendering the session's own design
    /// `LayoutScene` (built from the promoted document) with the current
    /// widget runtime state overlaid, then a focus caret on top.
    /// `canvas_region` is the screen-space canvas rect (clip + transform
    /// origin); `pan` / `zoom` come from the editor viewport. The scene
    /// paints through the SAME painter the design canvas uses, so preview
    /// is pixel-identical plus live.
    pub fn paint_scene(
        &self,
        backend: &mut dyn RenderBackend,
        canvas_region: Rect,
        pan: (f32, f32),
        zoom: f32,
        now_ms: u64,
    ) {
        // Avoid cloning the (potentially large) design scene until the
        // user has actually interacted: with no live widget state the
        // overlay is a no-op, so paint the scene directly. Once any
        // widget carries runtime state we clone + overlay it.
        let overlaid;
        let scene: &LayoutScene = if self.runtime.widget_states.iter().next().is_none() {
            &self.scene
        } else {
            overlaid = self.overlay_runtime_state(&self.scene);
            &overlaid
        };
        let Some(page) = scene.active_page() else {
            return;
        };
        let viewport_origin = Point2D::new(
            canvas_region.origin.x + pan.0,
            canvas_region.origin.y + pan.1,
        );
        const CULL_MARGIN: f32 = 64.0;
        let cull = Rect {
            origin: Point2D::new(
                canvas_region.origin.x - CULL_MARGIN,
                canvas_region.origin.y - CULL_MARGIN,
            ),
            size: Point2D::new(
                canvas_region.size.x + CULL_MARGIN * 2.0,
                canvas_region.size.y + CULL_MARGIN * 2.0,
            ),
        };
        backend.save();
        backend.clip_rect(canvas_region);
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            paint_scene_page(&mut cx, page, viewport_origin, zoom, cull);
        }
        self.paint_focus_caret(backend, scene, viewport_origin, zoom, now_ms);
        backend.restore();
    }

    /// Clone the design scene and overlay each interactive widget's LIVE
    /// runtime value so the preview reflects typed text / toggles /
    /// slider drags / selection. Pure (no paint), so it is unit-tested
    /// without a backend. Geometry is untouched — only `SceneWidget`
    /// value fields change — so the overlay can never scramble layout.
    fn overlay_runtime_state(&self, base: &LayoutScene) -> LayoutScene {
        let mut scene = base.clone();
        let idx = scene.active_page_index;
        if let Some(page) = scene.pages.get_mut(idx) {
            for node in page.children.iter_mut() {
                self.overlay_node(node);
            }
        }
        scene
    }

    /// Recursively overlay runtime widget state onto a scene subtree.
    fn overlay_node(&self, node: &mut SceneNode) {
        if let Some(widget) = node.widget.as_mut() {
            if let Some(state) = self.runtime.widget_states.get(&node.id) {
                apply_widget_state(widget, state);
            }
        }
        for child in node.children.iter_mut() {
            self.overlay_node(child);
        }
    }

    /// Draw a 1px caret for the focused text widget, aligned to the
    /// glyph advance of the value up to the caret. Metrics mirror
    /// `op_editor_ui`'s `paint_text_field` (14px font; left inset via the
    /// shared `widget_text_inset_left`, so the caret clears a leading
    /// icon). No-op unless a text widget is focused with a blink-visible
    /// caret.
    fn paint_focus_caret(
        &self,
        backend: &mut dyn RenderBackend,
        scene: &LayoutScene,
        viewport_origin: Point2D,
        zoom: f32,
        now_ms: u64,
    ) {
        let Some(id) = self.focused_schema_id() else {
            return;
        };
        let Some(WidgetState::TextInput(st)) = self.runtime.widget_states.get(&id) else {
            return;
        };
        if !st.caret_visible(now_ms) {
            return;
        }
        let Some(node) = scene.active_page().and_then(|p| p.find(&id)) else {
            return;
        };
        let Some(widget) = node.widget.as_ref() else {
            return;
        };
        if !matches!(
            widget.kind.as_str(),
            "text_input" | "text_area" | "number_input"
        ) {
            return;
        }

        // Mirror `paint_text_field`: fixed 14px label, left inset via the
        // shared helper (8px pad, +icon box when a leading icon is set),
        // single-line vertically centred (text_area top-aligned).
        const FONT: f32 = 14.0;
        let inset = op_editor_ui::widgets::widget_text_inset_left(widget);
        let world_x = viewport_origin.x + node.bounds.origin.x * zoom;
        let world_y = viewport_origin.y + node.bounds.origin.y * zoom;
        let world_h = node.bounds.size.y * zoom;
        let fs_world = FONT * zoom;
        let text_x = world_x + inset * zoom;
        let top_y = if widget.kind == "text_area" {
            world_y + 8.0 * zoom
        } else {
            world_y + (world_h - fs_world) / 2.0
        };

        // Caret byte offset, clamped to a UTF-8 boundary so slicing a
        // multi-byte value never panics.
        let text = st.text();
        let mut caret_byte = st.caret().min(text.len());
        while caret_byte > 0 && !text.is_char_boundary(caret_byte) {
            caret_byte -= 1;
        }
        let advance = backend.measure_text_weighted(&text[..caret_byte], fs_world, 400);
        let caret_x = text_x + advance;

        // Near-black caret (matches the field's value text colour) at a
        // crisp ≥1px width regardless of zoom.
        let color = Color {
            r: 0.067,
            g: 0.067,
            b: 0.067,
            a: 1.0,
        };
        backend.stroke_line(
            Point2D::new(caret_x, top_y),
            Point2D::new(caret_x, top_y + fs_world),
            color,
            zoom.max(1.0),
        );
    }

    /// Schema id of the currently focused node, mapped from the runtime
    /// focus chain.
    fn focused_schema_id(&self) -> Option<String> {
        let key = self.runtime.focus.current()?;
        let doc = self.runtime.document.as_ref()?;
        let node = doc.tree.nodes.get(key)?;
        Some(jian_core::document::tree::node_schema_id(&node.schema).to_owned())
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

    /// Dispatch a tap (Down then Up) at a SCENE-space point into the
    /// runtime so clicks land on switches / buttons / and place caret /
    /// focus in text inputs. The host converts the screen press to scene
    /// (document) space via the editor viewport; here we translate it
    /// into the runtime's root-relative space (subtract the containing
    /// root's authored origin) so the hit-test matches where the widget
    /// paints. Returns `true` when the runtime emitted any semantic
    /// event.
    pub fn dispatch_tap(&mut self, scene_x: f32, scene_y: f32) -> bool {
        use jian_core::geometry::point;
        use jian_core::gesture::pointer::{PointerEvent, PointerPhase};
        let (rt_x, rt_y) = self.scene_to_runtime(scene_x, scene_y);
        let down = PointerEvent::simple(1, PointerPhase::Down, point(rt_x, rt_y));
        let mut emitted = self.runtime.dispatch_pointer(down);
        let up = PointerEvent::simple(1, PointerPhase::Up, point(rt_x, rt_y));
        emitted.extend(self.runtime.dispatch_pointer(up));
        !emitted.is_empty()
    }

    /// Translate a scene-space point into the runtime's root-relative
    /// hit-test space: find the page-root whose scene bounds contain the
    /// point and subtract its authored origin. Falls through unchanged
    /// when the point is outside every root (nothing to hit there). For
    /// a single root authored at the origin this is the identity.
    fn scene_to_runtime(&self, x: f32, y: f32) -> (f32, f32) {
        for frame in &self.root_frames {
            let r = frame.scene_rect;
            if x >= r.origin.x
                && x <= r.origin.x + r.size.x
                && y >= r.origin.y
                && y <= r.origin.y + r.size.y
            {
                return (x - frame.offset.0, y - frame.offset.1);
            }
        }
        (x, y)
    }

    /// Advance focus to the next focusable widget (Tab).
    pub fn focus_next(&mut self) {
        self.runtime.focus_next();
        self.seed_focused_widget_state();
    }

    /// Advance focus to the previous focusable widget (Shift+Tab).
    pub fn focus_previous(&mut self) {
        self.runtime.focus_previous();
        self.seed_focused_widget_state();
    }

    /// Lazily seed the focused widget's runtime state so a freshly
    /// Tab-focused (but not-yet-typed) text input shows its caret right
    /// away — `Runtime::focus_next` only moves the focus pointer; it
    /// does not touch the widget-state store. A no-op for non-widget
    /// (or already-seeded) focus targets.
    fn seed_focused_widget_state(&mut self) {
        let Some(key) = self.runtime.focus.current() else {
            return;
        };
        // Clone the focused node's schema so the `&PenNode` borrow of
        // `runtime.document` is released before `get_or_init` takes
        // `runtime.widget_states` mutably (focus changes are rare, so
        // the clone is cheap relative to the interaction it serves).
        let schema = self
            .runtime
            .document
            .as_ref()
            .and_then(|d| d.tree.nodes.get(key))
            .map(|n| n.schema.clone());
        if let Some(schema) = schema {
            self.runtime.widget_states.get_or_init(&schema);
        }
    }

    /// Test-only read access to the live runtime so the host test can
    /// assert injected text reached the widget state graph.
    #[cfg(test)]
    pub(crate) fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Test-only: the session's own scene with live runtime widget
    /// values overlaid — what `paint_scene` walks — so render tests can
    /// assert widget values without a backend.
    #[cfg(test)]
    pub(crate) fn preview_scene_for_test(&self) -> LayoutScene {
        self.overlay_runtime_state(&self.scene)
    }

    /// Test-only: translate a scene-space point into the runtime's
    /// root-relative space (exercises the tap coordinate fix).
    #[cfg(test)]
    pub(crate) fn scene_to_runtime_for_test(&self, x: f32, y: f32) -> (f32, f32) {
        self.scene_to_runtime(x, y)
    }

    /// Test-only: the absolute layout rect `(x, y, w, h)` the runtime
    /// resolved for the node with schema `id`, or `None` if unknown.
    /// In the runtime's root-relative space (no scene offset).
    #[cfg(test)]
    pub(crate) fn node_rect(&self, id: &str) -> Option<(f32, f32, f32, f32)> {
        let doc = self.runtime.document.as_ref()?;
        let key = doc.tree.by_id.get(id).copied()?;
        let r = self.runtime.layout.node_rect(key)?;
        Some((r.origin.x, r.origin.y, r.size.width, r.size.height))
    }

    /// Test-only: the available size the runtime's primary root was laid
    /// out against (the root's authored size).
    #[cfg(test)]
    pub(crate) fn available(&self) -> (f32, f32) {
        self.available
    }
}

/// Overlay one widget's live runtime value onto its scene widget. Only
/// value fields change — geometry / options / labels stay as the design
/// scene resolved them. The static design value is the fallback (no
/// runtime state exists until the user interacts).
fn apply_widget_state(widget: &mut SceneWidget, state: &WidgetState) {
    match state {
        // text_input / text_area / number_input. `Some("")` falls back
        // to the placeholder in `text_field_display_text`, so an empty
        // edited field shows its placeholder again.
        WidgetState::TextInput(st) => {
            widget.value_str = Some(st.text().to_owned());
        }
        // switch / checkbox.
        WidgetState::Toggle { on } => {
            widget.checked = Some(*on);
        }
        WidgetState::Slider { value, .. } => {
            widget.value_num = Some(*value as f32);
        }
        WidgetState::Select { value, .. } => {
            widget.value_str = value.clone();
        }
        WidgetState::Radio { value, .. } => {
            widget.value_str = value.clone();
        }
        WidgetState::Tabs { active, .. } => {
            widget.value_str = active.clone();
        }
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
