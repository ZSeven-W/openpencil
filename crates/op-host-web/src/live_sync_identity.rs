//! What happens when the tab stops speaking for the account it is showing.
//!
//! Split out of `live_sync_glue.rs` at the 800-line cap. Two related things
//! live here: the latch that stops a tab pushing once the daemon has refused
//! its credential, and the reset that rebuilds the tab when
//! `/api/auth/status` reports a different account.
//!
//! Together they close the same hole — a browser tab outlives a sign-in, so
//! without them account B inherits account A's document, sync baseline and
//! session state, and cannot displace them.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use op_editor_core::sync_gate::SyncGate;
use op_editor_core::web_sync::WebSyncClient;

use crate::repaint_ctx::RepaintContext;

use super::{ACTIVE_SYNC, AUTH_INVALID};

/// Whether the daemon has refused this tab's credential.
pub(crate) fn auth_is_invalid() -> bool {
    AUTH_INVALID.with(std::cell::Cell::get)
}

/// Latch the tab out of pushing after a 401/403.
pub(crate) fn note_auth_invalid() {
    AUTH_INVALID.with(|flag| flag.set(true));
}

/// Release the latch. Only the identity reset calls this.
pub(crate) fn clear_auth_invalid() {
    AUTH_INVALID.with(|flag| flag.set(false));
}

/// Drop everything this tab was showing for the previous account.
///
/// Called when `/api/auth/status` reports a different subject. Without it the
/// tab keeps the previous account's document (the shell simply never replaced
/// it), its sync baseline, and — because the sync client only accepts a
/// version HIGHER than the one it applied — the new account's own document,
/// which starts lower, can never displace it. So this is not tidy-up; it is
/// the only thing that makes the switch take effect at all.
pub(crate) fn reset_for_new_identity<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    ACTIVE_SYNC.with(|slot| {
        if let Some(sync) = slot.borrow().as_ref().and_then(Weak::upgrade) {
            let mut sync = sync.borrow_mut();
            // A fresh client has applied no version, so the next probe
            // downloads the new account's document whatever its number.
            sync.client = WebSyncClient::new();
            sync.gate = SyncGate::default();
            sync.push_busy = false;
        }
    });
    if let Ok(mut context) = inner.try_borrow_mut() {
        let state = context.host_mut().editor_state_mut();
        // The subject has already advanced, but this reset repaints before the
        // new partition is loaded. Never promote a ready catalog belonging to
        // the previous subject during that repaint and dirty the new subject's
        // settings snapshot.
        state.editor_ui.pending_locale = None;
        state.editor_ui.locale_persistence_override = None;
        // Back to the same starter a fresh tab paints, so nothing of the
        // previous account survives on screen.
        state.replace_document(op_editor_core::EditorState::starter().doc);
        state.editor_ui.collab = op_editor_core::CollabUiState::default();
        // A toast describes something that happened to the PREVIOUS account's
        // document. Leaving it up would show one user a sentence about another
        // user's data, and name an undo that no longer has anything to undo.
        state.editor_ui.dismiss_toast();
        context.host_mut().mark_editor_state_dirty();
        // Do not repaint between subject change and partition reload. The
        // caller reloads account-scoped settings immediately and repaints once
        // afterward; an intermediate frame would run persistence under the new
        // subject while the old partition's settings are still in memory.
    }
    crate::collab_sync::reset_for_new_identity();
    // Queued and in-flight credential uploads belong to the previous account;
    // letting them land would push its API keys into the new account's tenant.
    // `reset` drops the queue and the retry timer. It does NOT re-tag the
    // epoch — an XHR already on the wire is made inert instead by the epoch
    // each request captured when it was issued, which its completion callback
    // re-checks against `identity_epoch::epoch()`. Re-tagging here would be
    // the bug: `reset` runs on the switching path, so a global "current epoch"
    // read at completion time would compare the new epoch against itself and
    // let the stale upload through.
    crate::web_credential_sync::reset();
    // The id allocator lives on the host, so it is torn down here where the
    // borrow is already held. B1's namespace latch alone was not enough: the
    // allocator itself would keep minting in the previous account's namespace.
    if let Ok(mut context) = inner.try_borrow_mut() {
        crate::collab_sync::reset_id_allocation(context.host_mut());
    }
    clear_auth_invalid();
}
