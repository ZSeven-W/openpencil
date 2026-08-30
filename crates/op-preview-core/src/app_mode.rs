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
use crate::invalidation::InvalidationKind;
use jian_core::action::services::Router;
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
pub(crate) struct AppMode {
    /// The full normalized multi-screen document (all synthetic pages,
    /// entry first) — refs/tokens resolved, legacy widgets promoted.
    /// Retained so Task 9 can re-derive scenes for any screen without
    /// re-running projection.
    pub(crate) promoted_doc: jian_ops_schema::PenDocument,
    /// Route table derived from the projected document (path → synthetic
    /// page).
    pub(crate) table: jian_core::screens::ScreenTable,
    /// Installed on `runtime.nav`; also the drainer of rejected navs.
    pub(crate) router: std::rc::Rc<jian_core::screens::ScreenRouter>,
    /// The screen path currently mounted in the runtime — compared
    /// against `router.current()` each reconcile pass.
    pub(crate) current_path: String,
    /// The route stack (`router.current().stack`) as of the LAST
    /// reconcile pass — Track C-3's reference point for classifying a
    /// NEW switch as push/pop/replace. The live router's stack has
    /// already advanced by the time `reconcile` runs (the tap mutated it
    /// synchronously), so this recorded copy is the only way to see the
    /// depth BEFORE the switch being reconciled right now.
    pub(crate) mounted_stack: Vec<String>,
    /// Index of `current_path`'s synthetic page inside `promoted_doc`.
    pub(crate) page_idx: usize,
    /// The active theme `enter` was called with — reused so a screen
    /// switch re-derives the scene against the same theme.
    pub(crate) theme: std::collections::BTreeMap<String, String>,
}

/// Result of the app-mode per-frame reconcile. `repaint` preserves the
/// old boolean meaning (anything repaint-relevant changed); `switched`
/// is true ONLY when the mounted screen actually swapped — the host
/// recenters / resets device-frame scroll on `switched`, never on a
/// warning-only pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReconcileOutcome {
    pub repaint: bool,
    pub switched: bool,
}

impl ReconcileOutcome {
    pub fn invalidation(self) -> InvalidationKind {
        if self.switched {
            InvalidationKind::Navigation
        } else if self.repaint {
            InvalidationKind::PaintOnly
        } else {
            InvalidationKind::None
        }
    }
}

impl PreviewSession {
    /// Whether this session is running a routed multi-screen APP MODE
    /// document (vs. the classic single-page workbench preview).
    pub fn is_app_mode(&self) -> bool {
        self.app.is_some()
    }

    /// Track C-2's screen-switcher pill row: `(path, display label)` pairs
    /// in a STABLE order (paths sorted ascending — not read straight off
    /// `ScreenTable`'s internal collection so the pill order can't shuffle
    /// on an unrelated jian refactor). The label is the routed page's own
    /// `name` (`screen_projection` sets it from the source frame's name),
    /// falling back to the path's slug when a screen was never named.
    /// Empty outside APP MODE.
    pub fn screen_switcher_entries(&self) -> Vec<(String, String)> {
        let Some(app) = &self.app else {
            return Vec::new();
        };
        sorted_screen_paths(app)
            .into_iter()
            .map(|path| {
                let label = app
                    .table
                    .page_index(&path)
                    .and_then(|idx| app.promoted_doc.pages.as_ref()?.get(idx))
                    .map(|page| page.name.clone())
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| path.trim_start_matches('/').to_string());
                (path, label)
            })
            .collect()
    }

    /// Index of the currently-mounted screen into
    /// [`screen_switcher_entries`](Self::screen_switcher_entries)'s order,
    /// or `None` outside APP MODE.
    pub fn current_screen_index(&self) -> Option<usize> {
        let app = self.app.as_ref()?;
        sorted_screen_paths(app)
            .iter()
            .position(|p| p == &app.current_path)
    }

    /// Navigate to `path` via the installed router — the screen-switcher
    /// pill row's click handler (Track C-2), and the escape hatch for any
    /// screen whose nav tab Track A never bound to `on_tap`. A no-op
    /// outside APP MODE. `path` need not be valid: the router itself
    /// validates and records an unknown-path rejection the next
    /// `reconcile` drains into `preview.warnings`, exactly like a runtime
    /// tap on an unwired nav button would.
    pub fn navigate_to_screen(&self, path: &str) -> InvalidationKind {
        if let Some(app) = &self.app {
            app.router.replace(path);
            InvalidationKind::Navigation
        } else {
            InvalidationKind::None
        }
    }

    /// Track C-4 (edge-swipe): whether a `pop` right now would actually
    /// move anywhere — the router's stack depth is `1` at the entry
    /// screen (or in a fresh multi-screen session that was never pushed
    /// into), and `Router::pop` on a depth-1 stack is a documented no-op
    /// (`ScreenRouter::pop`). `false` outside APP MODE.
    pub fn can_pop(&self) -> bool {
        self.app
            .as_ref()
            .is_some_and(|app| app.router.current().stack.len() > 1)
    }

    /// Navigate back via the installed router — the edge-swipe gesture's
    /// action, mirroring the on-screen back button's `{"pop": null}`
    /// path. A no-op outside APP MODE or at the entry screen.
    pub fn pop_screen(&self) {
        if let Some(app) = &self.app {
            app.router.pop();
        }
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
    /// outside APP MODE. The outcome separates repaint-relevant changes
    /// from actual screen switches so warning-only passes never recenter.
    /// `now_ms` seeds a new Track C-3 transition's clock on a switch —
    /// see `crate::preview::transition`.
    pub fn reconcile(&mut self, now_ms: u64) -> ReconcileOutcome {
        let (pending_path, old_path, old_page_idx) = {
            let Some(app) = self.app.as_ref() else {
                return ReconcileOutcome::default();
            };
            (
                app.router.current().path.clone(),
                app.current_path.clone(),
                app.page_idx,
            )
        };
        // R4 lifecycle: peek whether a route switch is pending BEFORE
        // reconcile_screens commits it, and pre-resolve the OUTGOING
        // side's `onLeave` / `onUnmount` ActionLists against the still-
        // mounted tree. They spawn (in deterministic leave → unmount →
        // mount → enter order) only AFTER the swap: spawned earlier they
        // would be cancelled by the swap's task teardown, and executed
        // synchronously they could deadlock on authored `delay` actions.
        let switching = pending_path != old_path;
        let mut outgoing_lifecycle: Vec<(
            &'static str,
            serde_json::Value,
            Option<String>,
            serde_json::Value,
        )> = Vec::new();
        if switching {
            if let Some(page) = self
                .app
                .as_ref()
                .and_then(|app| app.promoted_doc.pages.as_ref())
                .and_then(|pages| pages.get(old_page_idx))
            {
                if let Some(list) = enabled_page_hook(page.lifecycle.as_ref(), "onLeave") {
                    outgoing_lifecycle.push((
                        "onLeave",
                        serde_json::to_value(list).unwrap_or(serde_json::Value::Null),
                        None,
                        serde_json::json!({
                            "reason": "leave",
                            "page": page.id,
                            "next": pending_path,
                        }),
                    ));
                }
            }
            for key in self.runtime.nodes_with_lifecycle_hook("onUnmount") {
                if let Some((list, node_id)) = self.resolved_node_hook(key, "onUnmount") {
                    outgoing_lifecycle.push((
                        "onUnmount",
                        list,
                        Some(node_id.clone()),
                        serde_json::json!({
                            "reason": "unmount",
                            "nodeId": node_id,
                            "previous": old_path,
                        }),
                    ));
                }
            }
        }
        let Some(app) = self.app.as_mut() else {
            return ReconcileOutcome::default();
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
                return ReconcileOutcome {
                    repaint: true,
                    switched: false,
                };
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
            return ReconcileOutcome {
                repaint: changed,
                switched: false,
            };
        }

        // Track C-3: classify the switch from the stack-depth delta
        // against what was mounted as of the LAST reconcile (see
        // `mounted_stack`'s doc — the live router already advanced), then
        // snapshot the pre-switch scene as the outgoing transition layer
        // BEFORE it is overwritten below. A brand-new transition always
        // REPLACES whatever was still playing outright (`transition`
        // module doc): no queuing, so a rapid second nav never leaves a
        // half-finished slide lingering.
        let new_stack = app.router.current().stack;
        let prev_stack = std::mem::replace(&mut app.mounted_stack, new_stack.clone());
        let transition_kind =
            super::transition::classify_transition(prev_stack.len(), new_stack.len());
        // `app` (borrowed from `self.app` above) is not read again after
        // the two lines above in THIS binding's scope, so its borrow ends
        // here (NLL) — letting `overlay_runtime_state` below borrow the
        // whole `&self` (it needs `self.runtime` + `self.binding_sites`,
        // opaque to the borrow checker at the call site) without
        // conflicting with the outstanding `&mut self.app`. Re-borrowed
        // immediately after for the rebuild that follows.
        let outgoing_page = self
            .overlay_runtime_state(&self.scene)
            .active_page()
            .cloned();
        let app = self
            .app
            .as_mut()
            .expect("checked Some at function entry; nothing here can clear it mid-reconcile");

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
        // A screen switch invalidates every in-flight pointer anchor —
        // their rects belong to the unmounted screen's scene (per-pointer
        // anchors since R4, so concurrent fingers cannot leak a stale
        // mapping across the switch).
        self.gesture_mappings.clear();
        app.page_idx = app.table.page_index(&app.current_path).unwrap_or(0);
        self.scene = op_pen_loader::pen_document_to_layout_scene_for_preview(
            &app.promoted_doc,
            &self.layout_doc,
            self.preserve_authored_geometry,
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
        self.binding_overlay.replace_document(&app.promoted_doc);
        self.binding_overlay.set_runtime_document(
            self.runtime
                .document
                .as_ref()
                .map(|document| document.schema.clone()),
        );
        match solve_roots(&mut self.runtime, &self.measure) {
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

        // R4 lifecycle: resolve the INCOMING side's `onMount` /
        // `onEnter` against the freshly-mounted tree, then spawn all
        // four hook groups in the deterministic order leave → unmount →
        // mount → enter. Everything here spawns through the task queue
        // (`spawn_lifecycle`), so authored actions schedule exactly like
        // event actions; the outgoing lists were resolved pre-swap
        // (above) because their scopes die with the old tree.
        let new_page = self.app.as_ref().and_then(|app| {
            let pages = app.promoted_doc.pages.as_ref()?;
            pages.get(app.page_idx).cloned()
        });
        let mut incoming_lifecycle: Vec<(
            &'static str,
            serde_json::Value,
            Option<String>,
            serde_json::Value,
        )> = Vec::new();
        for key in self.runtime.nodes_with_lifecycle_hook("onMount") {
            if let Some((list, node_id)) = self.resolved_node_hook(key, "onMount") {
                incoming_lifecycle.push((
                    "onMount",
                    list,
                    Some(node_id.clone()),
                    serde_json::json!({
                        "reason": "mount",
                        "nodeId": node_id,
                        "previous": old_path,
                    }),
                ));
            }
        }
        if let Some(page) = new_page {
            if let Some(list) = enabled_page_hook(page.lifecycle.as_ref(), "onEnter") {
                incoming_lifecycle.push((
                    "onEnter",
                    serde_json::to_value(list).unwrap_or(serde_json::Value::Null),
                    None,
                    serde_json::json!({
                        "reason": "enter",
                        "page": page.id,
                        "previous": old_path,
                    }),
                ));
            }
        }
        // `app`'s mutable borrow ended above (NLL) — field borrows below
        // are disjoint.
        for (hook, list, node_id, payload) in outgoing_lifecycle {
            self.runtime.spawn_lifecycle(hook, list, node_id, payload);
        }
        for (hook, list, node_id, payload) in incoming_lifecycle {
            self.runtime.spawn_lifecycle(hook, list, node_id, payload);
        }

        // Track C-3: start the transition against the freshly-settled
        // scene now that the switch has fully landed. `outgoing_page` is
        // `None` only for an empty pre-switch page (nothing to animate
        // from) — `paint_framed_animated` falls back to a plain static
        // paint of the entering layer in that case.
        if let Some(page) = outgoing_page {
            self.transition = Some(super::transition::ScreenTransition::start(
                transition_kind,
                page,
                now_ms,
            ));
        }

        // R8: the route changed, so anything a user deferred against the
        // PREVIOUS screen is now aimed at a screen that is gone. Bumping
        // the counter is what makes `replay_deferred_input` drop it.
        self.route_generation = self.route_generation.saturating_add(1);
        self.transition_tap = None;
        self.animation.clear();

        ReconcileOutcome {
            repaint: true,
            switched: true,
        }
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

    /// The (serialized ActionList, node schema id) for an ENABLED node
    /// lifecycle hook on `key` in the CURRENT runtime tree, or `None`
    /// when the node has no such authored hook or lists it in its own
    /// `disabledEvents`. See [`resolved_node_hook_impl`].
    fn resolved_node_hook(
        &self,
        key: jian_core::document::tree::NodeKey,
        hook: &str,
    ) -> Option<(serde_json::Value, String)> {
        resolved_node_hook_impl(&self.runtime, key, hook)
    }

    /// Test-only: the APP MODE screen path currently mounted
    /// (`app.current_path`), or `""` outside APP MODE.
    #[cfg(any(all(test, not(target_os = "windows")), feature = "testing"))]
    pub fn current_path_for_test(&self) -> &str {
        self.app
            .as_ref()
            .map(|a| a.current_path.as_str())
            .unwrap_or("")
    }

    /// Test-only: the installed `ScreenRouter`, so a test can drive
    /// navigations directly (`router.push(...)`) without a tap. Panics
    /// outside APP MODE.
    #[cfg(any(all(test, not(target_os = "windows")), feature = "testing"))]
    pub fn router_for_test(&self) -> &std::rc::Rc<jian_core::screens::ScreenRouter> {
        &self.app.as_ref().expect("app mode").router
    }
}

/// The (serialized ActionList, node schema id) for an ENABLED node
/// lifecycle hook on `key` in the CURRENT runtime tree, or `None` when
/// the node has no such authored hook or lists it in its own
/// `disabledEvents`. Works for both the outgoing tree (pre-swap
/// resolution, R4) and the incoming tree (post-swap).
fn resolved_node_hook_impl(
    runtime: &jian_core::Runtime,
    key: jian_core::document::tree::NodeKey,
    hook: &str,
) -> Option<(serde_json::Value, String)> {
    let node = runtime.document.as_ref()?.tree.nodes.get(key)?;
    let hooks = node.schema.lifecycle()?;
    if hooks
        .disabled_events
        .as_deref()
        .is_some_and(|list| list.iter().any(|name| name == hook))
    {
        return None;
    }
    let list = match hook {
        "onMount" => hooks.on_mount.as_ref()?,
        "onUnmount" => hooks.on_unmount.as_ref()?,
        _ => return None,
    };
    let node_id = jian_core::document::tree::node_schema_id(&node.schema).to_owned();
    Some((serde_json::to_value(list).ok()?, node_id))
}

/// The ENABLED page-level hook list on `hooks`, or `None` when absent,
/// disabled, or unknown to the page scope.
fn enabled_page_hook<'a>(
    hooks: Option<&'a jian_ops_schema::lifecycle::PageLifecycleHooks>,
    hook: &str,
) -> Option<&'a jian_ops_schema::events::ActionList> {
    let hooks = hooks?;
    if hooks
        .disabled_events
        .as_deref()
        .is_some_and(|list| list.iter().any(|name| name == hook))
    {
        return None;
    }
    match hook {
        "onEnter" => hooks.on_enter.as_ref(),
        "onLeave" => hooks.on_leave.as_ref(),
        "onForeground" => hooks.on_foreground.as_ref(),
        "onBackground" => hooks.on_background.as_ref(),
        _ => None,
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
/// `AppMode`'s known screen paths, sorted ascending — the SAME stable
/// order [`PreviewSession::screen_switcher_entries`] /
/// [`PreviewSession::current_screen_index`] both build on, computed
/// independently in each so a caller only needing the count/index doesn't
/// pay for label lookups.
fn sorted_screen_paths(app: &AppMode) -> Vec<String> {
    // Present screens in the DOCUMENT's page order, not alphabetical route
    // order: the designer arranged the screens (and any in-design nav that
    // mirrors them) in page order, so an alphabetized switcher reads as
    // shuffled next to the design's own bottom nav. Path is only the
    // tiebreak for screens that share a page slot.
    let mut paths = app.table.paths();
    paths.sort_by(|a, b| {
        let key = |p: &String| (app.table.page_index(p).unwrap_or(usize::MAX), p.clone());
        key(a).cmp(&key(b))
    });
    paths
}

pub(crate) fn solve_roots(
    runtime: &mut Runtime,
    measure: &std::rc::Rc<dyn jian_core::layout::measure::MeasureBackend>,
) -> Result<(Vec<RootFrame>, (f32, f32)), super::PreviewLayoutError> {
    use super::PreviewLayoutError;

    runtime.layout.set_backend(std::rc::Rc::clone(measure));
    let primary_available = {
        let Some(rt_doc) = runtime.document.as_ref() else {
            return Err(PreviewLayoutError::NoDocument);
        };
        let root_keys = rt_doc.tree.roots.clone();
        let taffy_roots = runtime
            .layout
            .build(&rt_doc.tree)
            .map_err(|e| PreviewLayoutError::BuildTree(e.to_string()))?;
        // `build` never clears `runtime.document`; surface an error
        // rather than panic to keep the no-panic contract.
        let Some(rt_doc) = runtime.document.as_ref() else {
            return Err(PreviewLayoutError::DocumentVanished);
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
                .map_err(|e| PreviewLayoutError::Compute(e.to_string()))?;
        }
        primary.unwrap_or((1440.0, 900.0))
    };
    runtime.rebuild_spatial();

    // Capture each root's scene↔runtime coordinate mapping for tap
    // translation. The design scene offsets every root by its authored
    // `(base.x, base.y)`; the runtime lays each at its own origin.
    // `runtime.document` + `runtime.layout` are disjoint fields, so the
    // two immutable borrows below co-exist.
    //
    // Merge note (responsive-m1a into main): `LayoutEngine::node_rect`
    // now bakes a non-origin-normalized root's own authored offset into
    // its returned rect (see `op-pen-loader`'s `compute_layout` for the
    // full explanation) — `solve_roots` above never calls
    // `override_root_for_viewport`, so `rrect` here already carries
    // `offset`. Adding `offset` again doubled `scene_rect`'s origin,
    // making every scene-space tap fall outside every root's bounds
    // (matched nothing, fell through to the "outside all roots"
    // passthrough). `frame.offset` itself (used unchanged by
    // `input.rs`'s scene→runtime subtraction) is still the standalone
    // authored origin, independent of this rect-construction bug.
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
                        origin: Point2D::new(rx, ry),
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
