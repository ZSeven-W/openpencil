//! The platform's reduced-motion preference for the editor chrome.
//!
//! `OPENPENCIL_REDUCED_MOTION=1` / `=0` forces it either way (the preview
//! runtime already honoured that variable); otherwise macOS answers from the
//! system Accessibility ▸ Display ▸ Reduce Motion setting. Other desktops
//! report no preference.

pub(crate) fn system_reduced_motion() -> bool {
    match std::env::var("OPENPENCIL_REDUCED_MOTION").ok().as_deref() {
        Some("1") => true,
        Some("0") => false,
        _ => platform_reduced_motion(),
    }
}

#[cfg(target_os = "macos")]
fn platform_reduced_motion() -> bool {
    // SAFETY: `sharedWorkspace` returns the process-wide NSWorkspace and the
    // getter only reads a system accessibility flag.
    unsafe {
        objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_reduced_motion() -> bool {
    false
}

impl crate::DesktopApp {
    /// Re-read the preference (launch, and whenever the window regains
    /// focus — the user may have flipped the setting meanwhile).
    pub(crate) fn refresh_reduced_motion(&mut self) {
        let reduced = system_reduced_motion();
        let ui = &mut self.host.editor_state_mut().editor_ui;
        if ui.reduced_motion != reduced {
            ui.reduced_motion = reduced;
            self.host.mark_editor_state_dirty();
        }
    }
}
