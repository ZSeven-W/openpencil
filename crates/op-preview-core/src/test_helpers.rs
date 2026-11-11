//! Test-only surface of [`PreviewSession`]: narrow read-outs and live-runtime
//! access for the crate's own tests (and, behind the `testing` feature, for
//! op-host-native's), plus the layout backend and font-registry lock every
//! preview test module shares. None of it is compiled into a release build.

use crate::session::PreviewSession;
#[cfg(all(test, not(target_os = "windows")))]
use jian_core::layout::measure::MeasureBackend;
#[cfg(all(test, not(target_os = "windows")))]
use jian_core::Runtime;
#[cfg(any(all(test, not(target_os = "windows")), feature = "testing"))]
use op_editor_ui::layout_scene::LayoutScene;
#[cfg(all(test, not(target_os = "windows")))]
use std::rc::Rc;

impl PreviewSession {
    /// Narrow test-only clock readout: the session's current monotonic time.
    #[cfg(any(all(test, not(target_os = "windows")), feature = "testing"))]
    pub fn now_ms_for_test(&self) -> u64 {
        self.last_now_ms
    }

    /// Test-only: the session's scene with live runtime values overlaid.
    #[doc(hidden)]
    #[cfg(any(all(test, not(target_os = "windows")), feature = "testing"))]
    pub fn preview_scene_for_test(&self) -> LayoutScene {
        self.overlay_runtime_state(&self.scene)
    }

    /// Test-only: how many times the runtime overlay cloned the scene.
    #[cfg(test)]
    pub fn overlay_builds_for_test(&self) -> u64 {
        self.overlay_builds_for_test.get()
    }

    /// Test-only read access to the live runtime so the crate's own
    /// tests can assert injected text reached the widget state graph.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Test-only mutable access (e.g. `focused_editable_snapshot` for
    /// the R4 IME trace assertions). Same `!Send` caveat as `runtime`.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    /// Narrow test-only snapshot of one `$app` state value, CLONED out
    /// of the runtime. `Runtime.state` is interior-mutable, so it must
    /// never be handed out in production — cross-crate tests read this
    /// cloned seam instead (op-host-native's test builds enable the
    /// `testing` feature; see its Cargo.toml dev-dependency).
    #[cfg(any(all(test, not(target_os = "windows")), feature = "testing"))]
    pub fn app_state_value_for_test(&self, key: &str) -> Option<jian_core::value::RuntimeValue> {
        self.runtime.state.app_get(key)
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
            Some(jian_core::widget_state::WidgetState::TextInput(st)) => st.text().to_string(),
            _ => String::new(),
        }
    }
}

/// Share the canvas backend so parity tests compare the same metrics, including
/// the Windows CI estimator. Font-specific tests inject SkiaMeasure explicitly.
#[cfg(all(test, not(target_os = "windows")))]
pub(crate) fn test_measure() -> Rc<dyn MeasureBackend> {
    op_pen_loader::layout_measure_backend()
}

#[cfg(all(test, not(target_os = "windows")))]
pub(crate) mod font_registry_test_support {
    use std::sync::{LazyLock, Mutex, MutexGuard};

    static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    pub(crate) fn lock() -> MutexGuard<'static, ()> {
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
}
