//! APP MODE state + per-root layout solve for [`super::PreviewSession`].
//!
//! Split out of `preview/mod.rs` to keep it under the repo's
//! 800-line-per-file cap. Holds:
//! - [`AppMode`] — the routed multi-screen state a marked document
//!   enters (entry screen mounted + `ScreenRouter` installed).
//! - [`solve_roots`] — the per-root layout + scene↔runtime coordinate
//!   capture, extracted from `enter` so Task 9's screen-switch reconcile
//!   can re-solve the newly-mounted root(s) the same way.
//! - the [`super::PreviewSession`] app-mode query + mutation methods
//!   (`is_app_mode` / `current_screen_scene_rect` / `reconcile` +
//!   test-only accessors). Rust allows inherent `impl` blocks in a
//!   child module; the fields they read are scoped `pub(in
//!   crate::preview)` on the struct in `mod.rs` (a plain `pub(super)`
//!   would resolve to `pub(crate)` here and leak the preview-only
//!   `AppMode` type — the `private_interfaces` lint). `reconcile`
//!   additionally reads/writes several plain-private `PreviewSession`
//!   fields (`runtime` / `scene` / `warnings` / `binding_sites` /
//!   `root_frames` / `available`); those need no extra visibility
//!   annotation because Rust's default (unmarked) privacy is already
//!   visible to descendant modules, and `app_mode` is one.

use super::{PreviewSession, RootFrame};
use jian_core::Runtime;
use op_editor_ui::{Point2D, Rect};

/// APP MODE state: present only when [`super::PreviewSession::enter`]
/// found at least one explicitly `screen`-marked top-level frame
/// (`jian_ops_schema::screen_projection::project_screens`). The entry
/// screen is mounted first; switching screens (Task 9's per-frame
/// reconcile) drives `router`, compares it against `current_path`, and
/// swaps the mounted document via `table.doc_for_path`.
///
/// Every field is read by Task 9's reconcile loop, not yet by Task 8's
/// enter-only slice — `allow(dead_code)` at the struct level covers the
/// gap without littering per-field attributes.
#[allow(dead_code)]
pub(in crate::preview) struct AppMode {
    /// The full normalized multi-screen document (all synthetic pages,
    /// entry first) — refs/tokens resolved, legacy widgets promoted.
    /// Retained so Task 9 can re-derive scenes for any screen without
    /// re-running projection.
    pub(in crate::preview) promoted_doc: jian_ops_schema::PenDocument,
    /// Route table derived from the projected document (path → synthetic
    /// page).
    pub(in crate::preview) table: jian_core::screens::ScreenTable,
    /// Installed on `runtime.nav`; also the drainer of rejected navs.
    pub(in crate::preview) router: std::rc::Rc<jian_core::screens::ScreenRouter>,
    /// The screen path currently mounted in the runtime — compared
    /// against `router.current()` each reconcile pass.
    pub(in crate::preview) current_path: String,
    /// Index of `current_path`'s synthetic page inside `promoted_doc`.
    pub(in crate::preview) page_idx: usize,
    /// The active theme `enter` was called with — reused so a screen
    /// switch re-derives the scene against the same theme.
    pub(in crate::preview) theme: std::collections::BTreeMap<String, String>,
}

impl PreviewSession {
    /// Whether this session is running a routed multi-screen APP MODE
    /// document (vs. the classic single-page workbench preview).
    pub fn is_app_mode(&self) -> bool {
        self.app.is_some()
    }

    /// The scene-space bounds of the currently-mounted screen's first
    /// root, or `None`. Used by the host to center the viewport on the
    /// entry screen / a switched screen (Task 9).
    ///
    /// Gated on APP MODE: returns `None` for a classic workbench-mode
    /// session even though `root_frames` is populated there, so neither
    /// call site (`enter_preview` centering, `reconcile` re-centering)
    /// ever recenters an ordinary unmarked document — preserving the
    /// "no behavior change for unmarked docs" invariant.
    pub fn current_screen_scene_rect(&self) -> Option<Rect> {
        self.app.as_ref()?;
        self.root_frames.first().map(|f| f.scene_rect)
    }

    /// App-mode per-frame reconcile: drain rejected navigations into
    /// the warnings list and, when the route tip diverged from the
    /// mounted screen, swap the runtime document and rebuild the
    /// paint-side projections (layout, root frames, scene, binding
    /// sites) the same way `enter` built them for the entry screen.
    /// Called by the host once per frame BEFORE `paint_scene`. A no-op
    /// (`false`) outside APP MODE. Returns `true` when anything
    /// repaint-relevant changed (a screen switch or a newly appended
    /// warning), so the host knows to refresh `preview_warnings` and
    /// re-center the viewport on the newly-mounted screen.
    pub fn reconcile(&mut self) -> bool {
        let Some(app) = self.app.as_mut() else {
            return false;
        };
        let outcome = match jian_core::screens::reconcile_screens(
            &mut self.runtime,
            &app.router,
            &app.table,
            &mut app.current_path,
        ) {
            Ok(o) => o,
            Err(e) => {
                self.warnings
                    .push(format!("preview: screen switch failed: {e}"));
                return true;
            }
        };

        let mut changed = false;
        for r in outcome.rejections {
            self.warnings.push(format!(
                "preview: unknown route `{}` ({}) ignored",
                r.path, r.verb
            ));
            changed = true;
        }
        if outcome.switched.is_none() {
            return changed;
        }

        // The route tip diverged from the mounted screen: re-derive the
        // paint-side projections the same way `enter` built them for
        // the entry screen, so the newly-mounted root paints + hit-tests
        // exactly as it would have if the session had started there.
        //
        // `reconcile_screens` has ALREADY advanced `current_path` +
        // swapped the runtime document, so this switch is committed and
        // will never be retried (the next pass sees the path unchanged).
        // The scene + binding-sites rebuild (both infallible) therefore
        // runs UNCONDITIONALLY and BEFORE the fallible `solve_roots` —
        // otherwise a `solve_roots` error would leave paint permanently
        // stuck rendering the old screen. Rebuilding scene first means a
        // `solve_roots` failure leaves the visible scene matching the
        // new screen; only `root_frames` (the hit-test mapping) stays
        // stale — strictly better than nothing rebuilt.
        app.page_idx = app.table.page_index(&app.current_path).unwrap_or(0);
        self.scene = op_pen_loader::pen_document_to_layout_scene(
            &app.promoted_doc,
            &app.theme,
            app.page_idx,
        );
        self.binding_sites.clear();
        let children: &[jian_ops_schema::node::PenNode] = app
            .promoted_doc
            .pages
            .as_ref()
            .and_then(|ps| ps.get(app.page_idx))
            .map(|p| p.children.as_slice())
            .unwrap_or(&[]);
        super::binding_sites::collect_binding_sites(
            children,
            &mut self.binding_sites,
            &mut self.warnings,
        );
        match solve_roots(&mut self.runtime) {
            Ok((frames, available)) => {
                self.root_frames = frames;
                self.available = available;
            }
            Err(e) => {
                self.warnings.push(format!("preview: relayout failed: {e}"));
            }
        }

        // Seed every widget on the newly-mounted screen so the paint-time
        // overlay (`overlay_node` reads `widget_states.get()`
        // non-mutating) shows persisted app-scope values on the FIRST
        // frame after the switch — not just after the next tap/focus.
        // `replace_document` (inside `reconcile_screens`) pruned the
        // store to the new tree's ids via `retain_ids`, so a bound input
        // re-mounted here has no live entry; `get_or_init` re-seeds it
        // from the preserved `$state.*` value (Task 5 read-back). Runs
        // AFTER the scene/layout rebuild so it seeds against the screen
        // that will actually paint.
        self.seed_all_widget_states();
        true
    }

    /// Seed EVERY widget on the currently-mounted document so the
    /// paint-time overlay ([`super::PreviewSession::overlay_node`], which
    /// reads `widget_states.get()` non-mutating) surfaces each widget's
    /// value immediately — without waiting for the next tap/focus to
    /// lazily seed it. `WidgetStateStore::get_or_init` is idempotent and
    /// reads bound app-scope values via the loader's `bind:value`
    /// read-back (`jian_core::widget_state::bound_app_value`): an unbound
    /// widget seeds its authored prop, and a widget bound to `$state.*`
    /// carrying a persisted value (e.g. a text input typed on a prior
    /// screen) seeds THAT value.
    ///
    /// Whole-tree generalization of
    /// `PreviewSession::seed_focused_widget_state`; called only from
    /// `reconcile`'s screen-switch branch (the entry screen at `enter`
    /// starts with an empty store where authored == app-scope, so `enter`
    /// needs no equivalent). Follows the same clone-then-mutate borrow
    /// discipline: collect the schema clones FIRST (releasing the
    /// `runtime.document` borrow) before taking `widget_states` mutably.
    fn seed_all_widget_states(&mut self) {
        let schemas: Vec<jian_ops_schema::node::PenNode> = match self.runtime.document.as_ref() {
            Some(doc) => doc
                .tree
                .nodes
                .iter()
                .map(|(_, node_data)| node_data.schema.clone())
                .collect(),
            None => return,
        };
        for schema in &schemas {
            self.runtime
                .widget_states
                .get_or_init(schema, &self.runtime.state);
        }
    }

    /// Test-only: the APP MODE screen path currently mounted
    /// (`app.current_path`), or `""` outside APP MODE.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn current_path_for_test(&self) -> &str {
        self.app
            .as_ref()
            .map(|a| a.current_path.as_str())
            .unwrap_or("")
    }

    /// Test-only: the installed `ScreenRouter`, so a test can drive
    /// navigations directly (`router.push(...)`) without a tap. Panics
    /// outside APP MODE.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn router_for_test(&self) -> &std::rc::Rc<jian_core::screens::ScreenRouter> {
        &self.app.as_ref().expect("app mode").router
    }
}

/// Solve layout for every mounted page-root against its OWN authored
/// available size and capture the scene↔runtime coordinate mapping.
/// Extracted from `enter` so Task 9's screen-switch reconcile can
/// re-solve the newly-mounted root(s) the same way, without duplicating
/// this logic.
///
/// Mirrors `op_pen_loader::compute_layout`: installs the real skia
/// paragraph shaper (so `fit_content` text frames hit-test against the
/// glyph advances paint draws), then `compute`s EACH root against its
/// OWN authored available size. `Runtime::build_layout` would lay every
/// root against a single size, diverging from the design canvas. The
/// returned taffy NodeIds are positional with `doc.tree.roots` (see
/// `LayoutEngine::build`), so they are zipped to pair each root with the
/// id `compute` needs.
pub(in crate::preview) fn solve_roots(
    runtime: &mut Runtime,
) -> Result<(Vec<RootFrame>, (f32, f32)), String> {
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
    // translation. The design scene offsets every root by its authored
    // `(base.x, base.y)`; the runtime lays each at its own origin.
    // `runtime.document` + `runtime.layout` are disjoint fields, so the
    // two immutable borrows below co-exist.
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

    Ok((root_frames, primary_available))
}
