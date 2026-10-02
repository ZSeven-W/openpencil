//! System Back navigation for native mobile shells.

use crate::error::FfiError;
use crate::lifecycle::call_session;
use crate::OpStatus;
use op_editor_core::EntrySurface;

/// Dismiss one editor layer, or return a work reader to the Works list.
/// `consumed` is false at the root so the shell keeps its normal exit policy.
/// This never stops an active generation or discards its document.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread; `consumed` must
/// point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn op_editor_back(
    engine: *mut crate::OpEngine,
    consumed: *mut bool,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if consumed.is_null() {
                return Err(FfiError::invalid("back output pointer is null"));
            }
            consumed.write(false);
            let handled = session.with_collab_local_edit(|host| {
                if host.apply_escape() {
                    return true;
                }
                if !host.works_reader_visible() {
                    return false;
                }
                let ui = &mut host.editor_state_mut().editor_ui;
                ui.home.visible = true;
                ui.home.works_open = true;
                ui.home.composer_focused = false;
                ui.entry_surface = EntrySurface::Home;
                host.mark_editor_state_dirty();
                true
            })?;
            consumed.write(handled);
            if handled {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

#[cfg(test)]
#[path = "editor_back_tests.rs"]
mod tests;
