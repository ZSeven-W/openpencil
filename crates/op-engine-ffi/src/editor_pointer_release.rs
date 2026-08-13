//! Shared pointer-up seam for editor-mode ABI entry points.
//!
//! Both the dedicated mobile editor ABI and the generic pointer ABI report an
//! authoritative Up coordinate. Keep their host release behavior identical so
//! a quick slideshow flick works even when the OS emits no final Move.

use crate::error::FfiResult;
use crate::lifecycle::Session;

/// Apply an editor release at an already safe-area-local point.
pub(crate) fn release(session: &mut Session, x: f32, y: f32) -> FfiResult<bool> {
    let (viewport_w, viewport_h) = session.editor_viewport();
    let host = session.editor_mut()?;
    // This is deliberately narrower than `apply_cursor_move`: only an armed
    // slideshow gesture observes the Up endpoint. Ordinary drags retain their
    // existing last-Move and release semantics.
    host.preview_slideshow_release_point(x, y, viewport_w, viewport_h);
    Ok(host.apply_release_with_viewport(viewport_w, viewport_h))
}

#[cfg(all(test, not(target_os = "windows")))]
#[path = "editor_pointer_release_tests.rs"]
mod tests;
