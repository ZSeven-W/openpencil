use crate::session::PreviewSession;
#[cfg(any(all(test, not(target_os = "windows")), feature = "testing"))]
use op_editor_ui::layout_scene::LayoutScene;

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
}
