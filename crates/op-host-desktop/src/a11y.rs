//! Desktop accessibility adapter (#67).
//!
//! Bridges the host's assembled `accesskit::TreeUpdate` (built by
//! `op_host_native::WidgetHostNative::accessibility_tree_update`) to the
//! OS accessibility API so VoiceOver / Narrator / Orca can read the
//! editor, which otherwise looks like one opaque canvas.
//!
//! ## Why the raw-window-handle adapters, not `accesskit_winit`
//!
//! The desktop windowing crate is `casement` (a winit fork), imported as
//! `winit` via `package = "casement"`. `accesskit_winit` hard-depends on
//! the *upstream* `winit` crate, which would pull a second, incompatible
//! winit into the build — the same reason `glutin-winit` is rejected
//! (see the Cargo.toml comments). Instead this uses the platform
//! subclassing adapters keyed off the raw window handle, which depend
//! only on `accesskit` + `raw-window-handle`:
//!
//! - macOS:   `accesskit_macos::SubclassingAdapter` (NSView pointer)
//! - Windows: `accesskit_windows::SubclassingAdapter` (HWND)
//! - Linux:   `accesskit_unix::Adapter` (AT-SPI, windowless)
//!
//! ## Lifecycle
//!
//! The platform adapter pulls the *initial* tree lazily through an
//! [`ActivationHandler`] (assistive tech may not be running at window
//! creation). Subsequent frames are pushed via [`DesktopA11y::push`],
//! which takes a `build: FnOnce() -> TreeUpdate` closure rather than a
//! built [`TreeUpdate`]: the closure is handed straight to the platform
//! adapter's `update_if_active`, which only calls it when assistive tech
//! is actually attached. This is what makes the push cheap on every
//! ordinary painted frame — the caller (`app_handler.rs`) wraps the
//! expensive `WidgetHostNative::accessibility_tree_update` (including its
//! O(nodes) `LayerPanel::from_editor` walk) in the closure instead of
//! calling it eagerly, so the walk only happens while a screen reader is
//! actually listening.
//!
//! When the closure DOES run (adapter active), it also refreshes the
//! shared cache that [`CachedTreeActivation::request_initial_tree`]
//! reads from. That handler runs on a platform-owned thread that cannot
//! safely reach back into `&mut WidgetHostNative` (it isn't `Send` /
//! main-thread-only), so it can't build a tree itself; it serves the
//! most recent tree seen while the adapter was active instead. Per
//! accesskit's own `ActivationHandler::request_initial_tree` contract,
//! returning a still-`None` or *previous-period* cache at the instant of
//! (re)activation is acceptable — the platform shows a placeholder until
//! the real tree lands "no later than the next display refresh".
//!
//! ## Guaranteeing that "next display refresh" actually happens
//!
//! An idle editor parks the winit event loop in `ControlFlow::Wait` —
//! nothing schedules a redraw just because assistive tech attached, so
//! without an explicit nudge the cached (possibly `None` / stale) tree
//! could be all the AT ever sees. `request_initial_tree` therefore also
//! calls a [`WakeFn`] (`CachedTreeActivation::wake`, threaded in through
//! [`DesktopA11y::new`]) that the caller wires to the same
//! `EventLoopProxy::send_event` wake-up mechanism `DesktopApp` already
//! uses for live-MCP requests arriving off the render thread (see
//! `mcp_runtime.rs::mcp_wake_callback`). That queues a `DesktopEvent`
//! which `app_handler.rs::user_event` turns into `request_redraw(true)`,
//! guaranteeing a real `RedrawRequested` lands even if the app was
//! otherwise fully idle. That painted frame runs the normal per-frame
//! `push` path, which re-enters `update_if_active` — now active — and
//! builds + caches a current, full (non-incremental) tree, satisfying the
//! "must contain a full tree" requirement the docs call out for the
//! post-`None` case. Incoming [`accesskit::ActionRequest`]s (Focus /
//! Click) land in a thread-safe queue the runner drains each frame and
//! routes back into host state via `WidgetHostNative::apply_a11y_action`.

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, TreeUpdate};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Latest assembled tree, shared with the platform adapter's activation
/// handler (which may run on a platform thread).
type SharedTree = Arc<Mutex<Option<TreeUpdate>>>;

/// Queue of action requests from assistive tech, drained by the runner.
type ActionQueue = Arc<Mutex<VecDeque<ActionRequest>>>;

/// Cross-thread callback that wakes the UI event loop and asks for a
/// repaint. Fired from [`CachedTreeActivation::request_initial_tree`] so an
/// idle app (no dirty frame, `ControlFlow::Wait` parked) still converges on
/// a current tree promptly after activation instead of leaving the cached
/// (possibly `None` / stale) tree in place indefinitely. Boxed as `Arc<dyn
/// Fn>` — same shape as `DesktopApp::mcp_wake_callback` in
/// `mcp_runtime.rs`, which wakes the loop the same way for live-MCP
/// requests arriving off the render thread.
type WakeFn = Arc<dyn Fn() + Send + Sync>;

/// Returns the cached full tree to the platform adapter on activation, and
/// asks the host to wake the UI loop so a fresh tree lands promptly (see
/// [`WakeFn`]).
struct CachedTreeActivation {
    tree: SharedTree,
    wake: WakeFn,
}

impl ActivationHandler for CachedTreeActivation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        // Always ask for a repaint on activation, even when a cached tree
        // is returned below: the cache may be stale (built while the
        // adapter was last active, possibly several document edits ago —
        // it is never refreshed while inactive, see the module doc). The
        // repaint that follows re-enters `update_if_active` on the very
        // next painted frame and republishes a current tree.
        (self.wake)();
        self.tree.lock().ok().and_then(|t| t.clone())
    }
}

/// Pushes incoming action requests onto the shared queue.
struct QueueingActionHandler {
    queue: ActionQueue,
}

impl ActionHandler for QueueingActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        if let Ok(mut q) = self.queue.lock() {
            q.push_back(request);
        }
    }
}

/// A drained, host-level accessibility action: which editor region the
/// assistive tech targeted, and whether it was a focus (vs activation)
/// request. The runner feeds these to `WidgetHostNative::apply_a11y_action`.
pub struct A11yAction {
    /// Raw `accesskit::NodeId.0` == `WidgetId.0` of the target region.
    pub target: u64,
    /// `true` for `Action::Focus`; `false` for `Click` / `Default`.
    pub is_focus: bool,
}

/// Desktop accessibility adapter. One per window; created once the
/// window's raw handle is available, then fed a fresh tree each dirty
/// frame.
pub struct DesktopA11y {
    /// Platform adapter. macOS / Windows subclass the view / HWND; Linux
    /// is windowless. `None` if the window handle could not be resolved
    /// (the editor still runs, just without the a11y bridge).
    #[cfg(target_os = "macos")]
    adapter: Option<accesskit_macos::SubclassingAdapter>,
    #[cfg(target_os = "windows")]
    adapter: Option<accesskit_windows::SubclassingAdapter>,
    #[cfg(target_os = "linux")]
    adapter: Option<accesskit_unix::Adapter>,
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    adapter: Option<()>,

    tree: SharedTree,
    queue: ActionQueue,
}

impl DesktopA11y {
    /// Build the adapter for `window`. The platform adapter is created
    /// from the window's raw handle; if that can't be resolved the
    /// adapter is left inert (every method becomes a no-op) so the editor
    /// still runs. `wake` is called from the platform's activation
    /// handler — possibly off the main/render thread — to ask the host to
    /// schedule a repaint (see [`WakeFn`]); the caller wires this to the
    /// same `EventLoopProxy` wake-up mechanism used for live-MCP requests.
    pub fn new(window: &winit::window::Window, wake: impl Fn() + Send + Sync + 'static) -> Self {
        let tree: SharedTree = Arc::new(Mutex::new(None));
        let queue: ActionQueue = Arc::new(Mutex::new(VecDeque::new()));
        let wake: WakeFn = Arc::new(wake);
        let adapter = Self::build_adapter(window, tree.clone(), queue.clone(), wake);
        Self {
            adapter,
            tree,
            queue,
        }
    }

    /// Lazily build and push a tree to the platform adapter, raising any
    /// queued platform events. `build` is only invoked when assistive
    /// tech is actually attached — `update_if_active` skips calling it
    /// entirely otherwise, so the (potentially expensive) tree assembly
    /// stays off the hot path when nobody is listening. When it does
    /// run, the freshly built tree is also cached for
    /// [`CachedTreeActivation::request_initial_tree`] (see the module
    /// doc for why that handler can't build the tree itself).
    pub fn push(&mut self, build: impl FnOnce() -> TreeUpdate) {
        self.raise(build);
    }

    /// Drain pending action requests into host-level actions. The runner
    /// applies each via `WidgetHostNative::apply_a11y_action`.
    pub fn drain_actions(&mut self) -> Vec<A11yAction> {
        let mut out = Vec::new();
        if let Ok(mut q) = self.queue.lock() {
            while let Some(req) = q.pop_front() {
                let is_focus = matches!(req.action, accesskit::Action::Focus);
                // Only Focus / Click / Default map to host state today.
                if matches!(
                    req.action,
                    accesskit::Action::Focus | accesskit::Action::Click
                ) {
                    out.push(A11yAction {
                        target: req.target_node.0,
                        is_focus,
                    });
                }
            }
        }
        out
    }

    // --- platform-specific glue ------------------------------------

    #[cfg(target_os = "macos")]
    fn build_adapter(
        window: &winit::window::Window,
        tree: SharedTree,
        queue: ActionQueue,
        wake: WakeFn,
    ) -> Option<accesskit_macos::SubclassingAdapter> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let raw = window.window_handle().ok()?.as_raw();
        let RawWindowHandle::AppKit(handle) = raw else {
            return None;
        };
        let ns_view = handle.ns_view.as_ptr();
        let activation = CachedTreeActivation { tree, wake };
        let action = QueueingActionHandler { queue };
        // SAFETY: `ns_view` is the live NSView of `window`, which outlives
        // this adapter (the adapter is dropped with the host, before the
        // window). The handle came straight from `window.window_handle()`.
        let adapter =
            unsafe { accesskit_macos::SubclassingAdapter::new(ns_view, activation, action) };
        Some(adapter)
    }

    #[cfg(target_os = "macos")]
    fn raise(&mut self, build: impl FnOnce() -> TreeUpdate) {
        if let Some(adapter) = self.adapter.as_mut() {
            let tree = self.tree.clone();
            if let Some(events) = adapter.update_if_active(move || cache_and_build(build, &tree)) {
                events.raise();
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn build_adapter(
        window: &winit::window::Window,
        tree: SharedTree,
        queue: ActionQueue,
        wake: WakeFn,
    ) -> Option<accesskit_windows::SubclassingAdapter> {
        use accesskit_windows::HWND;
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let raw = window.window_handle().ok()?.as_raw();
        let RawWindowHandle::Win32(handle) = raw else {
            return None;
        };
        // raw-window-handle 0.6 exposes the HWND as a `NonZeroIsize`;
        // accesskit_windows' re-exported `HWND` wraps a `*mut c_void`.
        let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);
        let activation = CachedTreeActivation { tree, wake };
        let action = QueueingActionHandler { queue };
        Some(accesskit_windows::SubclassingAdapter::new(
            hwnd, activation, action,
        ))
    }

    #[cfg(target_os = "windows")]
    fn raise(&mut self, build: impl FnOnce() -> TreeUpdate) {
        if let Some(adapter) = self.adapter.as_mut() {
            let tree = self.tree.clone();
            if let Some(events) = adapter.update_if_active(move || cache_and_build(build, &tree)) {
                events.raise();
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn build_adapter(
        _window: &winit::window::Window,
        tree: SharedTree,
        queue: ActionQueue,
        wake: WakeFn,
    ) -> Option<accesskit_unix::Adapter> {
        // The Unix (AT-SPI / D-Bus) adapter is windowless. It needs a
        // deactivation handler in addition to activation + action; a
        // no-op suffices — the cache survives, so a re-activation just
        // re-publishes the current tree.
        struct NoopDeactivation;
        impl accesskit::DeactivationHandler for NoopDeactivation {
            fn deactivate_accessibility(&mut self) {}
        }
        let activation = CachedTreeActivation { tree, wake };
        let action = QueueingActionHandler { queue };
        Some(accesskit_unix::Adapter::new(
            activation,
            action,
            NoopDeactivation,
        ))
    }

    #[cfg(target_os = "linux")]
    fn raise(&mut self, build: impl FnOnce() -> TreeUpdate) {
        if let Some(adapter) = self.adapter.as_mut() {
            let tree = self.tree.clone();
            // The Unix adapter's `update_if_active` returns `()`.
            adapter.update_if_active(move || cache_and_build(build, &tree));
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    fn build_adapter(
        _window: &winit::window::Window,
        _tree: SharedTree,
        _queue: ActionQueue,
        _wake: WakeFn,
    ) -> Option<()> {
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    fn raise(&mut self, _build: impl FnOnce() -> TreeUpdate) {}
}

/// Run `build`, cache the result into `tree` (for a later activation's
/// [`CachedTreeActivation::request_initial_tree`]), and return it to the
/// platform adapter. Only ever invoked from inside `update_if_active`,
/// i.e. only while the adapter is active — the shared helper keeps that
/// invariant (build + cache together) identical across all three
/// platform `raise` impls.
fn cache_and_build(build: impl FnOnce() -> TreeUpdate, tree: &SharedTree) -> TreeUpdate {
    let update = build();
    if let Ok(mut cached) = tree.lock() {
        *cached = Some(update.clone());
    }
    update
}

#[cfg(test)]
impl DesktopA11y {
    /// Headless test double: no real platform adapter, so `raise` hits
    /// the same `self.adapter.as_mut() == None` short-circuit every
    /// platform's `raise` impl already has. `push`/`raise` can't be
    /// unit-tested end-to-end through a real accesskit adapter (that
    /// needs a live NSView / HWND / AT-SPI session and its "active" flag
    /// is platform-owned), so this exercises the one gate `DesktopA11y`
    /// itself controls: an absent adapter must never invoke `build`.
    fn new_for_test() -> Self {
        Self {
            adapter: None,
            tree: Arc::new(Mutex::new(None)),
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::{Node, NodeId, Role, Tree, TreeId};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn push_does_not_build_when_adapter_absent() {
        let mut a11y = DesktopA11y::new_for_test();
        // The adapter is `None` (as it always is off-macOS/Windows/Linux,
        // and as it is here by construction), so `raise` must return
        // without ever calling `build`. Panicking inside the closure
        // turns "build ran" into a hard test failure instead of a silent
        // observation.
        a11y.push(|| panic!("build must not run when no adapter is attached"));
    }

    #[test]
    fn drain_actions_is_empty_with_no_platform_adapter() {
        let mut a11y = DesktopA11y::new_for_test();
        assert!(a11y.drain_actions().is_empty());
    }

    /// Minimal valid `TreeUpdate` for equality assertions — shape mirrors
    /// `op_editor_ui::accessibility::assemble_tree_update`'s output
    /// (single root node, `tree` + `focus` set), just with a distinct
    /// `NodeId` per call so tests can tell trees apart.
    fn test_tree_update(id: u64) -> TreeUpdate {
        let root = NodeId(id);
        TreeUpdate {
            nodes: vec![(root, Node::new(Role::Window))],
            tree: Some(Tree::new(root)),
            tree_id: TreeId::ROOT,
            focus: root,
        }
    }

    // --- CachedTreeActivation: the real activation seam ------------
    //
    // `DesktopA11y::push`/`raise` can't be driven through a real
    // accesskit platform adapter in a unit test (needs a live NSView /
    // HWND / AT-SPI session, and "active" is platform-owned state), but
    // `CachedTreeActivation` — the `ActivationHandler` impl that runs on
    // activation and is the exact object the BLOCK finding is about — is
    // a plain struct this module owns outright. These tests exercise it
    // directly: does activation (a) always fire the wake callback, and
    // (b) still serve whatever's in the cache while the wake-triggered
    // repaint is in flight.

    #[test]
    fn activation_wakes_and_returns_none_when_cache_empty() {
        let wake_calls = Arc::new(AtomicUsize::new(0));
        let wake_calls_handle = wake_calls.clone();
        let mut activation = CachedTreeActivation {
            tree: Arc::new(Mutex::new(None)),
            wake: Arc::new(move || {
                wake_calls_handle.fetch_add(1, Ordering::SeqCst);
            }),
        };

        let result = activation.request_initial_tree();

        assert!(
            result.is_none(),
            "no tree has been cached yet, so activation must not fabricate one"
        );
        assert_eq!(
            wake_calls.load(Ordering::SeqCst),
            1,
            "activation with an empty cache must still request a repaint so a \
             real tree lands promptly instead of leaving the AT with nothing \
             indefinitely"
        );
    }

    #[test]
    fn activation_wakes_and_returns_cached_tree_when_present() {
        let wake_calls = Arc::new(AtomicUsize::new(0));
        let wake_calls_handle = wake_calls.clone();
        let cached = test_tree_update(42);
        let mut activation = CachedTreeActivation {
            tree: Arc::new(Mutex::new(Some(cached.clone()))),
            wake: Arc::new(move || {
                wake_calls_handle.fetch_add(1, Ordering::SeqCst);
            }),
        };

        let result = activation.request_initial_tree();

        // Stale-but-something beats a placeholder: a previously-cached
        // tree (built the last time the adapter was active) is served
        // immediately, even though...
        assert_eq!(
            result,
            Some(cached),
            "a cached tree must be served immediately on activation"
        );
        // ...the wake still fires, because the cache may be stale (built
        // several document edits ago) — the repaint that follows
        // republishes a CURRENT tree via the normal per-frame push path.
        assert_eq!(
            wake_calls.load(Ordering::SeqCst),
            1,
            "activation must request a repaint even when a cached tree exists, \
             since that cache is not guaranteed to be current"
        );
    }

    #[test]
    fn cache_and_build_refreshes_cache_then_activation_serves_it() {
        // End-to-end of the two units together: `cache_and_build` (what
        // every platform's `raise` calls from inside `update_if_active`,
        // i.e. only while the adapter is active) writes into the same
        // `SharedTree` a subsequent `CachedTreeActivation` reads from —
        // this is the mechanism that lets a *later* activation serve a
        // tree built during a previous active period.
        let tree: SharedTree = Arc::new(Mutex::new(None));
        let built = cache_and_build(|| test_tree_update(7), &tree);

        let wake_calls = Arc::new(AtomicUsize::new(0));
        let wake_calls_handle = wake_calls.clone();
        let mut activation = CachedTreeActivation {
            tree: tree.clone(),
            wake: Arc::new(move || {
                wake_calls_handle.fetch_add(1, Ordering::SeqCst);
            }),
        };

        assert_eq!(activation.request_initial_tree(), Some(built));
        assert_eq!(wake_calls.load(Ordering::SeqCst), 1);
    }
}
