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
//! Binding overlay limits (Spec-2 slice): only `content` bindings are
//! re-resolved; the scene is NOT re-laid-out, so text that grows past
//! its authored box paints per the design painter's normal overflow
//! behavior. `visible` / fill / geometry bindings are collected but not
//! yet applied.
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
//! To honor the 800-line-per-file cap, [`AppMode`] + the per-root
//! `solve_roots` + the app-mode query methods live in `app_mode.rs`,
//! keyboard/focus/pointer dispatch + the scene→runtime coordinate
//! mapping live in `input.rs`, and the leaf formatter helpers
//! (`apply_widget_state` / `display_string` / `format_warning`) live in
//! `scene_helpers.rs`. `RootFrame` + `PreviewSession` stay here
//! (shared), with the fields those sibling modules touch scoped
//! `pub(in crate::preview)` (not `pub(super)`, which resolves to
//! `pub(crate)` at this top-level module and would trip
//! `private_interfaces` on the `AppMode` type).

mod app_mode;
#[cfg(feature = "gl-host")]
mod auto_wire;
#[cfg(not(feature = "gl-host"))]
mod auto_wire_stub;
mod binding_sites;
mod error;
mod input;
mod mode_transition;
mod present;
mod scene_helpers;
mod transition;
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
#[cfg(all(test, not(target_os = "windows")))]
mod tests_caret;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_device_frame;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_geometry_parity;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_tabs;
#[cfg(all(test, not(target_os = "windows")))]
mod tests_transition;

use app_mode::AppMode;
use binding_sites::{collect_binding_sites, BindingSite};
pub use error::{PreviewEnterError, PreviewLayoutError};
use scene_helpers::{apply_widget_state, display_string, format_warning};

#[allow(unused_imports)]
pub(crate) use mode_transition::{lerp_color, ModeTransition, ModeTransitionKind};
#[allow(unused_imports)]
pub(crate) use present::PinnedPaint;

use jian_core::action::services::Router;
use jian_core::render::widget_style::{resolve_authored_widget_visual, with_visual_opacity};
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
    /// The design `LayoutScene` preview paints: paint tree from the
    /// prepared + PROMOTED document (so a generated/legacy `role=input`
    /// field renders as an interactive `text_input` widget, not a
    /// frame), GEOMETRY from the unpromoted `layout_doc` laid out
    /// exactly as the design canvas lays it out — so node positions
    /// match design mode by construction. Live widget values are
    /// overlaid onto a clone of this each frame in `paint_scene`.
    scene: LayoutScene,
    /// The prepared (ref/token-resolved, page-projected / screen-
    /// normalized) but UNPROMOTED document — the geometry source the
    /// design canvas would lay out. Kept so app-mode screen switches
    /// rebuild the scene against the same geometry.
    layout_doc: jian_ops_schema::PenDocument,
    /// Whether the editor document carries authored (Figma Preserve)
    /// geometry: the design canvas skips the flex solver for these, so
    /// preview must too or every element shifts.
    preserve_authored_geometry: bool,
    /// Non-fatal load warnings (e.g. legacy role promotions), formatted
    /// for display in the editor's `preview.warnings`.
    warnings: Vec<String>,
    /// Compiled non-`bind:value` bindings from the promoted document,
    /// re-evaluated against the live state graph each overlay pass (see
    /// `apply_binding_sites`) so `set $app.*` writes become visible.
    binding_sites: Vec<BindingSite>,
    /// APP MODE state (routed multi-screen doc), or `None` for the
    /// classic single-page workbench preview. `pub(in crate::preview)`
    /// so `app_mode`'s `is_app_mode` can read it. See [`AppMode`].
    pub(in crate::preview) app: Option<AppMode>,
    /// The (scene rect, runtime rect) pair the current pointer gesture
    /// anchored on at `Down` — held `Move`s and the `Up` map through
    /// it (pointer capture), so a drag that leaves the node's scene
    /// bounds doesn't remap through a neighbour. `None` between
    /// gestures or when the `Down` hit no mapped node.
    gesture_mapping: Option<(Rect, Rect)>,
    /// Track C-3: the in-flight screen-transition animation, set by
    /// `app_mode::reconcile` on every screen switch. `None` when idle
    /// (including the entire classic workbench-mode session, which never
    /// switches screens).
    transition: Option<transition::ScreenTransition>,
    /// The last value passed to [`Self::set_now_ms`] — `transition`'s
    /// idle input dispatch guard (`input.rs`) needs "now" but the
    /// dispatch methods don't take a clock param of their own.
    last_now_ms: u64,
}

fn widget_field_foreground(node: &SceneNode) -> Color {
    let visual = resolve_authored_widget_visual(
        node.fill.map(Color::to_jian),
        node.stroke.map(|stroke| stroke.color.to_jian()),
    );
    // Scene fill/stroke already carry direct-paint opacity; the contrast-derived
    // caret does not, so fold it exactly once through the shared widget policy.
    let color = with_visual_opacity(visual.foreground, node.opacity);
    Color::rgba_u8(
        color.r(),
        color.g(),
        color.b(),
        f32::from(color.a()) / 255.0,
    )
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
    /// ## Presenting a deck
    ///
    /// `presenting` says the host is about to run this document as a
    /// slideshow (`op_editor_core::preview_slideshow`). A deck's boards are
    /// slides, not screens: routing them would present them in route order
    /// instead of the authored order, and would leave every board but the
    /// mounted one out of the scene the presentation has to draw. So both
    /// the screen projection and its auto-wire fallback are skipped, and
    /// the session keeps the plain workbench scene with every board in it.
    ///
    /// Returns [`PreviewEnterError`] if serialization, parsing, runtime
    /// build, or layout fails — the host then declines to enter preview and
    /// surfaces the rendered message.
    pub fn enter(
        doc: &jian_ops_schema::PenDocument,
        canvas_size: (f32, f32),
        active_theme: &std::collections::BTreeMap<String, String>,
        active_page_index: usize,
        preserve_authored_geometry: bool,
        presenting: bool,
    ) -> Result<Self, PreviewEnterError> {
        let _ = canvas_size; // layout is root-derived, not canvas-derived.

        // Track C-1: if the document has no authored `screen` marker at
        // all, auto-wire a preview-only clone with Track A's deterministic
        // screen/nav pass before anything else runs, so a hand-drawn or
        // pre-Track-A multi-screen document still enters App Mode. `doc`
        // is a local binding for the rest of `enter` — it either points at
        // the caller's document (untouched) or at `auto_wired` (owned
        // here); either way the CALLER's document is never mutated. See
        // `auto_wire`'s module doc for the "any marker → skip entirely"
        // rationale.
        #[cfg(feature = "gl-host")]
        let auto_wired = (!presenting)
            .then(|| auto_wire::auto_wire_for_preview(doc, active_page_index))
            .flatten();
        // Mobile: the orchestrator-backed pass is not linked; authored
        // `screen` markers still drive App Mode.
        #[cfg(not(feature = "gl-host"))]
        let auto_wired: Option<jian_ops_schema::PenDocument> = None;
        let doc: &jian_ops_schema::PenDocument = auto_wired.as_ref().unwrap_or(doc);

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
        // `project_screens` now always returns its warnings alongside the
        // projection outcome (previously nested inside the `Some`, so a
        // failed projection silently dropped them) — the projected doc
        // still carries a `ScreenVariantTable` for responsive breakpoint
        // variants, which this preview path doesn't consume yet (Phase 3
        // scope: breakpoint-aware preview UI).
        if !presenting {
            let (projected, ws) = jian_ops_schema::screen_projection::project_screens(&prepared);
            projection_warnings.extend(ws.iter().map(|w| format!("preview: {w}")));
            if let Some((normalized, _variants)) = projected {
                prepared = std::borrow::Cow::Owned(normalized);
                app_projected = true;
            }
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

        // Own the prepared (unpromoted) tree: it is BOTH the runtime's
        // serialization source and the preview scene's geometry source
        // (the design canvas lays out this exact tree, so taking rects
        // from it keeps preview positions design-identical).
        let layout_doc = prepared.into_owned();
        let src = serde_json::to_string(&layout_doc)
            .map_err(|e| PreviewEnterError::Serialize(e.to_string()))?;

        let loaded = load_str_with(
            &src,
            LoadOptions {
                promote_legacy_widgets: true,
            },
        )
        .map_err(|e| PreviewEnterError::Parse(e.to_string()))?;

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
                .ok_or(PreviewEnterError::LostRoutes)?;
            let router = std::rc::Rc::new(jian_core::screens::ScreenRouter::new(
                table.entry_path(),
                table.paths(),
            ));
            let mounted_stack = router.current().stack;
            app = Some(AppMode {
                current_path: table.entry_path().to_owned(),
                mounted_stack,
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

        let mut runtime = Runtime::new_from_document(loaded.value)
            .map_err(|e| PreviewEnterError::BuildRuntime(e.to_string()))?;
        if let Some(a) = &app {
            runtime.nav = a.router.clone();
        }

        let (root_frames, primary_available) = app_mode::solve_roots(&mut runtime)?;

        // Build the preview scene: paint tree from the promoted
        // document, geometry from the unpromoted `layout_doc` — the
        // design canvas's exact layout (or, for Figma Preserve imports,
        // its authored rects), so preview positions match design mode
        // by construction. The active page was projected to the top
        // level in `enter`, so it is page index 0. APP MODE: page 0 is
        // the entry screen (the `project_screens` convention) either way.
        let scene = op_pen_loader::pen_document_to_layout_scene_for_preview(
            &promoted_doc,
            &layout_doc,
            preserve_authored_geometry,
            active_theme,
            0,
        );

        Ok(Self {
            runtime,
            available: primary_available,
            root_frames,
            scene,
            layout_doc,
            preserve_authored_geometry,
            warnings,
            binding_sites,
            app,
            gesture_mapping: None,
            transition: None,
            last_now_ms: 0,
        })
    }

    /// The formatted load warnings collected on `enter` (for the
    /// editor's `preview.warnings` diagnostics surface).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Push the host clock so the runtime can drive caret blink etc.
    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.runtime.set_now_ms(now_ms);
        self.last_now_ms = now_ms;
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

        // Match the field's value foreground. The shared resolver derives a
        // readable caret from the authored surface/stroke, so a dark input no
        // longer receives the old hard-coded near-black caret.
        let color = widget_field_foreground(node);
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

    /// Test-only: the absolute layout rect `(x, y, w, h)` the runtime
    /// resolved for the node with schema `id`, or `None` if unknown.
    /// In the runtime's root-relative space (no scene offset).
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn node_rect(&self, id: &str) -> Option<(f32, f32, f32, f32)> {
        let r = self.runtime_rect(id)?;
        Some((r.origin.x, r.origin.y, r.size.x, r.size.y))
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
