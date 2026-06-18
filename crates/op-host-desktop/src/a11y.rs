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
//! creation). Subsequent frames are pushed via [`DesktopA11y::push`]:
//! it caches the latest tree (so a late activation gets a fresh one) and
//! calls the adapter's `update_if_active`, raising any queued platform
//! events. Incoming [`accesskit::ActionRequest`]s (Focus / Click) land
//! in a thread-safe queue the runner drains each frame and routes back
//! into host state via `WidgetHostNative::apply_a11y_action`.

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, TreeUpdate};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Latest assembled tree, shared with the platform adapter's activation
/// handler (which may run on a platform thread).
type SharedTree = Arc<Mutex<Option<TreeUpdate>>>;

/// Queue of action requests from assistive tech, drained by the runner.
type ActionQueue = Arc<Mutex<VecDeque<ActionRequest>>>;

/// Returns the cached full tree to the platform adapter on activation.
struct CachedTreeActivation {
    tree: SharedTree,
}

impl ActivationHandler for CachedTreeActivation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
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
    /// still runs.
    pub fn new(window: &winit::window::Window) -> Self {
        let tree: SharedTree = Arc::new(Mutex::new(None));
        let queue: ActionQueue = Arc::new(Mutex::new(VecDeque::new()));
        let adapter = Self::build_adapter(window, tree.clone(), queue.clone());
        Self {
            adapter,
            tree,
            queue,
        }
    }

    /// Cache `update` and hand it to the platform adapter, raising any
    /// queued platform events. Cheap when assistive tech isn't active
    /// (`update_if_active` skips building/sending in that case — but we
    /// still cache so a later activation gets the current tree).
    pub fn push(&mut self, update: TreeUpdate) {
        if let Ok(mut cached) = self.tree.lock() {
            *cached = Some(update.clone());
        }
        self.raise(update);
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
    ) -> Option<accesskit_macos::SubclassingAdapter> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let raw = window.window_handle().ok()?.as_raw();
        let RawWindowHandle::AppKit(handle) = raw else {
            return None;
        };
        let ns_view = handle.ns_view.as_ptr();
        let activation = CachedTreeActivation { tree };
        let action = QueueingActionHandler { queue };
        // SAFETY: `ns_view` is the live NSView of `window`, which outlives
        // this adapter (the adapter is dropped with the host, before the
        // window). The handle came straight from `window.window_handle()`.
        let adapter =
            unsafe { accesskit_macos::SubclassingAdapter::new(ns_view, activation, action) };
        Some(adapter)
    }

    #[cfg(target_os = "macos")]
    fn raise(&mut self, update: TreeUpdate) {
        if let Some(adapter) = self.adapter.as_mut() {
            if let Some(events) = adapter.update_if_active(|| update) {
                events.raise();
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn build_adapter(
        window: &winit::window::Window,
        tree: SharedTree,
        queue: ActionQueue,
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
        let activation = CachedTreeActivation { tree };
        let action = QueueingActionHandler { queue };
        Some(accesskit_windows::SubclassingAdapter::new(
            hwnd, activation, action,
        ))
    }

    #[cfg(target_os = "windows")]
    fn raise(&mut self, update: TreeUpdate) {
        if let Some(adapter) = self.adapter.as_mut() {
            if let Some(events) = adapter.update_if_active(|| update) {
                events.raise();
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn build_adapter(
        _window: &winit::window::Window,
        tree: SharedTree,
        queue: ActionQueue,
    ) -> Option<accesskit_unix::Adapter> {
        // The Unix (AT-SPI / D-Bus) adapter is windowless. It needs a
        // deactivation handler in addition to activation + action; a
        // no-op suffices — the cache survives, so a re-activation just
        // re-publishes the current tree.
        struct NoopDeactivation;
        impl accesskit::DeactivationHandler for NoopDeactivation {
            fn deactivate_accessibility(&mut self) {}
        }
        let activation = CachedTreeActivation { tree };
        let action = QueueingActionHandler { queue };
        Some(accesskit_unix::Adapter::new(
            activation,
            action,
            NoopDeactivation,
        ))
    }

    #[cfg(target_os = "linux")]
    fn raise(&mut self, update: TreeUpdate) {
        if let Some(adapter) = self.adapter.as_mut() {
            // The Unix adapter's `update_if_active` returns `()`.
            adapter.update_if_active(|| update);
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    fn build_adapter(
        _window: &winit::window::Window,
        _tree: SharedTree,
        _queue: ActionQueue,
    ) -> Option<()> {
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    fn raise(&mut self, _update: TreeUpdate) {}
}
