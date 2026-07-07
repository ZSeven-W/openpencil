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
//! Binding overlay limits (Spec-2 slice): only `content` bindings are
//! re-resolved; the scene is NOT re-laid-out, so text that grows past
//! its authored box paints per the design painter's normal overflow
//! behavior. `visible` / fill / geometry bindings are collected but not
//! yet applied.
//!
//! ## Hit-testing across two coordinate spaces
//!
//! The design scene offsets every page-root by its authored
//! `(base.x, base.y)`, but the jian runtime lays each root at its own
//! `(0, 0)`. So a tap arrives in SCENE space (it inverts the scene paint
//! transform) and must be translated back into the runtime's
//! root-relative space — subtract the containing root's authored origin
//! — before [`Runtime::dispatch_pointer`]. See [`PreviewSession::dispatch_tap`].
//!
//! ## Module split
//!
//! To honor the 800-line-per-file cap, [`AppMode`] + the per-root
//! `solve_roots` + the app-mode query methods live in `app_mode.rs`,
//! and the leaf formatter helpers (`apply_widget_state` /
//! `display_string` / `format_warning`) live in `scene_helpers.rs`.
//! `RootFrame` + `PreviewSession` stay here (shared), with the fields
//! those sibling modules touch scoped `pub(in crate::preview)` (not
//! `pub(super)`, which resolves to `pub(crate)` at this top-level
//! module and would trip `private_interfaces` on the `AppMode` type).

mod app_mode;
mod binding_sites;
mod scene_helpers;
// Gated off Windows: preview tests exercise runtime layout through
// `jian_skia::SkiaMeasure`, which hits DirectWrite in Windows CI and aborts
// with STATUS_ACCESS_VIOLATION before Rust can report a normal failure.
// macOS + Linux keep the full preview coverage.
#[cfg(all(test, not(target_os = "windows")))]
mod tests;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_app_mode;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_bindings;

use app_mode::AppMode;
use binding_sites::{collect_binding_sites, BindingSite};
use scene_helpers::{apply_widget_state, display_string, format_warning};

use jian_core::gesture::pointer::{Modifiers, PointerPhase};
use jian_core::widget_state::WidgetState;
use jian_core::Runtime;
use jian_ops_schema::compat::{load_str_with, LoadOptions};

use op_editor_ui::layout_scene::{LayoutScene, SceneNode};
use op_editor_ui::widgets::{paint_scene_page, PaintCx};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend};

/// One page-root's mapping between the design scene's coordinate space
/// (root offset baked in) and the jian runtime's root-relative
/// hit-test space. Used to translate a scene-space tap back into the
/// space `Runtime::dispatch_pointer` expects.
///
/// Fields are `pub(in crate::preview)` so `app_mode::solve_roots` (which
/// constructs these) can reach them from the child module.
pub(in crate::preview) struct RootFrame {
    /// The root's bounds in SCENE space (authored origin + size).
    pub(in crate::preview) scene_rect: Rect,
    /// The root's authored `(base.x, base.y)` — the delta between scene
    /// space and the runtime's root-relative space.
    pub(in crate::preview) offset: (f32, f32),
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
    /// `pub(in crate::preview)` so `app_mode`'s
    /// `current_screen_scene_rect` can read the first frame's scene rect.
    pub(in crate::preview) root_frames: Vec<RootFrame>,
    /// The design `LayoutScene` preview paints, built from the SAME
    /// prepared + PROMOTED document the runtime was seeded from (so a
    /// generated/legacy `role=input` field renders as an interactive
    /// `text_input` widget, not a frame). Live widget values are
    /// overlaid onto a clone of this each frame in `paint_scene`.
    scene: LayoutScene,
    /// Non-fatal load warnings (e.g. legacy role promotions), formatted
    /// for display in the editor's `preview_warnings`.
    warnings: Vec<String>,
    /// Compiled non-`bind:value` bindings from the promoted document,
    /// re-evaluated against the live state graph each overlay pass (see
    /// `apply_binding_sites`) so `set $app.*` writes become visible.
    binding_sites: Vec<BindingSite>,
    /// APP MODE state (routed multi-screen doc), or `None` for the
    /// classic single-page workbench preview. `pub(in crate::preview)`
    /// so `app_mode`'s `is_app_mode` can read it. See [`AppMode`].
    pub(in crate::preview) app: Option<AppMode>,
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
    /// ## App mode
    ///
    /// If the document carries at least one explicitly `screen`-marked
    /// top-level frame (`jian_ops_schema::screen_projection::
    /// project_screens`), `enter` projects it into a synthetic
    /// multi-page document (entry screen at `pages[0]`) and installs a
    /// [`jian_core::screens::ScreenRouter`] on `runtime.nav`; only the
    /// entry screen is mounted. Docs with no screen markers keep today's
    /// exact active-page workbench behavior, unchanged.
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

        // Screen projection: marked multi-screen docs enter APP MODE
        // (entry screen mounted, ScreenRouter installed); unmarked docs
        // fall through to the classic active-page workbench path.
        let mut projection_warnings: Vec<String> = Vec::new();
        let mut app_projected = false;
        if let Some((normalized, ws)) =
            jian_ops_schema::screen_projection::project_screens(&prepared)
        {
            projection_warnings.extend(ws.iter().map(|w| format!("preview: {w}")));
            prepared = std::borrow::Cow::Owned(normalized);
            app_projected = true;
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
        // Skipped in APP MODE: the normalized doc's `pages[0]` is already
        // the entry screen and must keep ALL its synthetic pages (Task 9
        // switches among them via the router).
        if !app_projected
            && prepared
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

        let mut warnings = loaded
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

        // APP MODE: derive the route table + router from the normalized
        // doc. In workbench mode `app` stays `None`.
        let mut app = None;
        if app_projected {
            let table = jian_core::screens::ScreenTable::from_document(promoted_doc.clone())
                .ok_or_else(|| "projected document lost its routes".to_string())?;
            let router = std::rc::Rc::new(jian_core::screens::ScreenRouter::new(
                table.entry_path(),
                table.paths(),
            ));
            app = Some(AppMode {
                current_path: table.entry_path().to_owned(),
                page_idx: 0,
                theme: active_theme.clone(),
                promoted_doc: promoted_doc.clone(),
                table,
                router,
            });
        }

        // Compile every non-`bind:value` binding on the promoted tree so
        // Task C2's overlay pass can re-evaluate them against the live
        // state graph each frame. Compile failures surface as warnings,
        // never as `enter` errors — a bad binding just doesn't animate.
        // APP MODE compiles bindings off the ENTRY page's children (the
        // mounted screen), not the top-level `children` — the normalized
        // doc keeps everything under `pages`.
        let mut binding_sites = Vec::new();
        let site_children: &[jian_ops_schema::node::PenNode] = if app_projected {
            &promoted_doc.pages.as_ref().unwrap()[0].children
        } else {
            &promoted_doc.children
        };
        collect_binding_sites(site_children, &mut binding_sites, &mut warnings);
        warnings.extend(projection_warnings);

        let mut runtime =
            Runtime::new_from_document(loaded.value).map_err(|e| format!("build runtime: {e}"))?;
        if let Some(a) = &app {
            runtime.nav = a.router.clone();
        }

        let (root_frames, primary_available) = app_mode::solve_roots(&mut runtime)?;

        // Build the design scene from the promoted document. The active
        // page was projected to the top-level `children` in `enter`, so
        // it is page index 0. Refs/tokens are already resolved in
        // `promoted_doc`, so the builder's detector walks early-out.
        // APP MODE: page 0 of `promoted_doc` is the entry screen (the
        // same convention `project_screens` guarantees), so this stays
        // page-index 0 either way.
        let scene = op_pen_loader::pen_document_to_layout_scene(&promoted_doc, active_theme, 0);

        Ok(Self {
            runtime,
            available: primary_available,
            root_frames,
            scene,
            warnings,
            binding_sites,
            app,
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
        let scene: &LayoutScene = if self.runtime.widget_states.iter().next().is_none()
            && self.binding_sites.is_empty()
        {
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

    /// Recursively overlay runtime widget state + live binding values
    /// onto a scene subtree.
    fn overlay_node(&self, node: &mut SceneNode) {
        if let Some(widget) = node.widget.as_mut() {
            if let Some(state) = self.runtime.widget_states.get(&node.id) {
                apply_widget_state(widget, state);
            }
        }
        self.apply_binding_sites(node);
        for child in node.children.iter_mut() {
            self.overlay_node(child);
        }
    }

    /// Re-evaluate this node's compiled bindings against the live state
    /// graph. Only `content` (scene text) lands today; other props are
    /// skipped until the preview painter learns them. Linear scan over
    /// the sites is fine at preview scale (a handful of bindings per
    /// document); index by node id if profiles ever say otherwise.
    fn apply_binding_sites(&self, node: &mut SceneNode) {
        for site in self.binding_sites.iter().filter(|s| s.node_id == node.id) {
            if site.prop != "content" {
                continue;
            }
            let (value, _warnings) = site.expr.eval(&self.runtime.state, None, Some(&node.id));
            node.text = Some(display_string(&value));
            // Bound text is dynamic single-style content — styled runs
            // resolved from the authored literal no longer apply.
            node.text_runs.clear();
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
        let down = self.dispatch_pointer_phase(scene_x, scene_y, PointerPhase::Down);
        let up = self.dispatch_pointer_phase(scene_x, scene_y, PointerPhase::Up);
        down || up
    }

    /// Dispatch one pointer phase at a SCENE-space point. `Down`/`Up`/
    /// `Move` carry mouse-left button semantics (a held drag — slider
    /// knobs); `Hover` is an unpressed move (fires `onHoverEnter` /
    /// `onHoverLeave` actions). Returns `true` when the runtime emitted
    /// any semantic event.
    pub fn dispatch_pointer_phase(
        &mut self,
        scene_x: f32,
        scene_y: f32,
        phase: PointerPhase,
    ) -> bool {
        use jian_core::geometry::point;
        use jian_core::gesture::pointer::{MouseButtons, PointerEvent, PointerKind};
        let (rt_x, rt_y) = self.scene_to_runtime(scene_x, scene_y);
        let mut ev = PointerEvent::simple(1, phase, point(rt_x, rt_y));
        ev.kind = PointerKind::Mouse;
        if matches!(phase, PointerPhase::Hover) {
            ev.buttons = MouseButtons::empty();
            ev.pressure = 0.0;
        }
        !self.runtime.dispatch_pointer(ev).is_empty()
    }

    /// Route a wheel at a SCENE-space point into the runtime. Returns
    /// `true` only when a node carrying `events.onScroll` consumed it —
    /// the host falls back to canvas pan/zoom otherwise. `dx`/`dy` are
    /// screen-pixel deltas (same magnitude the design canvas pans by).
    pub fn dispatch_wheel(&mut self, scene_x: f32, scene_y: f32, dx: f32, dy: f32) -> bool {
        use jian_core::geometry::point;
        use jian_core::gesture::pointer::WheelEvent;
        let (rt_x, rt_y) = self.scene_to_runtime(scene_x, scene_y);
        let ev = WheelEvent::simple(point(rt_x, rt_y), point(dx, dy));
        !self.runtime.dispatch_wheel(ev).is_empty()
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
            self.runtime
                .widget_states
                .get_or_init(&schema, &self.runtime.state);
        }
    }

    /// Test-only read access to the live runtime so the host test can
    /// assert injected text reached the widget state graph.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Test-only: the session's own scene with live runtime widget
    /// values overlaid — what `paint_scene` walks — so render tests can
    /// assert widget values without a backend.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn preview_scene_for_test(&self) -> LayoutScene {
        self.overlay_runtime_state(&self.scene)
    }

    /// Test-only: translate a scene-space point into the runtime's
    /// root-relative space (exercises the tap coordinate fix).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn scene_to_runtime_for_test(&self, x: f32, y: f32) -> (f32, f32) {
        self.scene_to_runtime(x, y)
    }

    /// Test-only: the absolute layout rect `(x, y, w, h)` the runtime
    /// resolved for the node with schema `id`, or `None` if unknown.
    /// In the runtime's root-relative space (no scene offset).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn node_rect(&self, id: &str) -> Option<(f32, f32, f32, f32)> {
        let doc = self.runtime.document.as_ref()?;
        let key = doc.tree.by_id.get(id).copied()?;
        let r = self.runtime.layout.node_rect(key)?;
        Some((r.origin.x, r.origin.y, r.size.width, r.size.height))
    }

    /// Test-only: the available size the runtime's primary root was laid
    /// out against (the root's authored size).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn available(&self) -> (f32, f32) {
        self.available
    }

    /// Test-only: number of compiled binding sites.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn binding_sites_len_for_test(&self) -> usize {
        self.binding_sites.len()
    }

    /// Test-only: number of currently-mounted page-roots (1 in APP MODE
    /// — the entry screen only; N for an unmarked doc's top-level
    /// frames).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn root_frames_len_for_test(&self) -> usize {
        self.root_frames.len()
    }

    /// Test-only: focus a node by schema `id` directly (skips the
    /// Tab-ring walk `focus_next`/`focus_previous` use), then seed its
    /// widget runtime state the same way those two do. Returns `true`
    /// when the id resolved to a live node AND that node is in the
    /// focus chain (`FocusManager::request` rejects ids outside it).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn focus_node_for_test(&mut self, id: &str) -> bool {
        let Some(key) = self
            .runtime
            .document
            .as_ref()
            .and_then(|d| d.tree.by_id.get(id).copied())
        else {
            return false;
        };
        self.runtime.focus_request(key);
        self.seed_focused_widget_state();
        self.runtime.focus.current() == Some(key)
    }

    /// Test-only: the current text value of the text-input-family
    /// widget with schema `id`, forcing the same lazy
    /// `WidgetStateStore::get_or_init` seed a live interaction would
    /// (mirrors `seed_focused_widget_state`'s borrow pattern) — so a
    /// bound input re-mounted after a screen switch reads back its
    /// persisted `$state.*` value even without an intervening focus
    /// call. Empty string for any other widget kind or an unknown id.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn widget_text_for_test(&mut self, id: &str) -> String {
        let schema = self.runtime.document.as_ref().and_then(|d| {
            let key = d.tree.by_id.get(id).copied()?;
            d.tree.nodes.get(key).map(|n| n.schema.clone())
        });
        let Some(schema) = schema else {
            return String::new();
        };
        match self
            .runtime
            .widget_states
            .get_or_init(&schema, &self.runtime.state)
        {
            Some(WidgetState::TextInput(st)) => st.text().to_string(),
            _ => String::new(),
        }
    }
}
