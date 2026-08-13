//! Page switching for multi-page documents.
//!
//! The engine renders the ACTIVE page (`state.ui.active_page_index`),
//! exactly like the desktop editor's canvas. Switching pages rebuilds the
//! layout scene from the live editor state, re-fits the new page into the
//! viewport, and commits any in-flight text edit.

use crate::error::{FfiError, FfiResult};
use crate::lifecycle::{call_session, Session};
use crate::OpStatus;

/// Number of document pages (the `.op` `pages` array, or the implicit
/// single-page fallback).
///
/// # Safety
///
/// `engine` must be live and `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_get_page_count(
    engine: *mut crate::OpEngine,
    out: *mut u32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if out.is_null() {
                return Err(FfiError::invalid("page-count output pointer is null"));
            }
            out.write(session.page_count());
            Ok(())
        })
    }
}

/// Switch to page `index` (0-based). The scene is rebuilt for the new
/// page, the viewport re-fits, and any active text edit commits.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_set_active_page(engine: *mut crate::OpEngine, index: u32) -> OpStatus {
    unsafe { call_session(engine, |session| session.set_active_page(index)) }
}

impl Session {
    /// Number of document pages (the `.op` `pages` array, or the implicit
    /// single-page fallback).
    pub(crate) fn page_count(&self) -> u32 {
        self.state
            .doc
            .pages
            .as_ref()
            .map(|pages| pages.len() as u32)
            .unwrap_or(1)
    }

    /// Switch to page `index`, rebuilding the scene + re-fitting the
    /// viewport. Commits any in-flight text edit first.
    pub(crate) fn set_active_page(&mut self, index: u32) -> FfiResult<()> {
        let count = self.page_count();
        if index >= count {
            return Err(FfiError::invalid(format!(
                "page index {index} out of range (0..{count})"
            )));
        }
        if self.state.ui.text_editing.is_some() {
            self.state.text_edit_commit();
            self.focus_changed(false, 0, 0);
        }
        // The EDITOR host owns the visible page: switching the session
        // state alone would leave `WidgetHostNative`'s separate editor
        // state (and its layout scene) on the old page.
        #[cfg(feature = "editor")]
        if let Some(host) = self.editor.as_mut() {
            if !host.editor_state_mut().set_active_page(index as usize) {
                return Err(FfiError::invalid(format!(
                    "page index {index} out of range (0..{count})"
                )));
            }
        } else {
            self.state.ui.active_page_index = index as usize;
        }
        self.selected = None;
        self.rebuild_scene();
        // A page switch always re-fits (the user expects the new page on
        // screen), regardless of prior pan/zoom.
        self.user_interacted = false;
        self.fit_content_to_viewports();
        self.request_redraw();
        Ok(())
    }
}
