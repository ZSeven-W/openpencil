//! When a sync conflict may resolve itself.
//!
//! Split out of `live_sync_glue.rs` at the 800-line cap. It is one decision
//! plus the deployment probe that feeds it, and it is worth reading as a unit:
//! getting it wrong either silently discards a user's unpushed work or leaves
//! a shared document permanently latched.

use std::cell::RefCell;
use std::rc::Rc;

use op_editor_core::CollabConnectionPhase;

use crate::repaint_ctx::RepaintContext;

/// Keep the about-to-be-overwritten local document, and raise the notice that
/// tells the user it is recoverable.
///
/// Returns whether the local document was preserved. A `false` means the
/// caller must NOT accept the remote this tick: overwriting without a backup
/// is the data loss this exists to prevent, and the latch simply stays up for
/// one more tick, which converges a few hundred milliseconds later.
///
/// `now_ms` is passed in rather than read from `performance.now()` here: the
/// clock is the only DOM dependency on this path, and lifting it to the caller
/// is what lets the preserve → notice → toast chain be driven in a test.
pub(super) fn preserve_local_document<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    now_ms: u64,
) -> bool {
    let Ok(mut context) = inner.try_borrow_mut() else {
        return false;
    };
    let state = context.host_mut().editor_state_mut();
    crate::live_sync_recovery::stash(state.doc.clone(), now_ms);
    state
        .editor_ui
        .collab
        .set_notice(op_editor_core::CollabNoticeKind::LocalEditPreserved, now_ms);
    // The collab notice above is only ever READ by the collaboration panel,
    // which is gated on an authenticated session — structurally unreachable in
    // an online tenant, where this path fires most. So online also raises the
    // editor's own toast, which needs no panel to be seen. Desktop and LAN
    // sessions deliberately do not: they have the panel, and two copies of the
    // same sentence on screen is worse than one.
    if server_is_authoritative() {
        state.editor_ui.show_toast(
            op_editor_core::CollabNoticeKind::LocalEditPreserved.i18n_key(),
            Vec::new(),
            // Warn, not Info: the document on screen was replaced under the
            // user. The sentence names undo as the way back.
            op_editor_core::editor_toast::EditorToastLevel::Warn,
            now_ms,
        );
    }
    context.host_mut().mark_editor_state_dirty();
    true
}

/// The safety decision behind [`maybe_auto_resolve_conflict_in_session`],
/// separated so it can be tested without a live shell.
///
/// Two situations qualify, for the same underlying reason — there is an
/// authoritative document to accept, so accepting it converges rather than
/// destroys:
///
/// 1. **A live collaboration session** (`Active`). The session core sequences
///    every edit, and a rejected local edit is projected into
///    `collab.discarded_edit` for the panel to replay.
///
/// 2. **A server-authoritative deployment** (`serveMode: online`). The daemon
///    owns the one in-memory document every writer reaches, and its version
///    counter is the total order. A 409 there means only "someone else's push
///    landed between this tab's read and its write" — and the SSE stream is
///    already delivering that newer document, so re-reading it is not a
///    choice between two candidate truths, it is catching up to the one.
///
/// Why overwriting is acceptable in case 2: the lost window is a single
/// push's diff — at most the ~2 s since this tab last synced — and the
/// alternative is the latch, which in a shared tenant never clears, because
/// nothing outside a session ever resolves it. A visitor would simply be
/// frozen out of the document. Demo-grade concurrency (409 → refetch) is the
/// documented semantic for a shared online tenant; a latch that requires a
/// session to lift is not a stricter version of that, it is a hang.
///
/// Outside both — a local or managed daemon with no session — the daemon is a
/// peer holding the operator's file, not an authority, and nothing preserves
/// the losing edit. The latch stays and explicit resolution is untouched.
pub(super) const fn auto_resolve_is_safe(
    has_conflict: bool,
    phase: CollabConnectionPhase,
    server_authoritative: bool,
) -> bool {
    has_conflict && (server_authoritative || matches!(phase, CollabConnectionPhase::Active))
}

/// Whether the daemon this shell talks to is the sole sequencer for the
/// document.
///
/// Learned once from `GET /api/mcp/server`'s `serveMode` (see
/// [`probe_serve_mode`]) and cached, because it is a property of the
/// deployment and cannot change without a reload. Defaults to `false` until
/// the probe answers, so the conservative behaviour is what runs during
/// start-up rather than the permissive one.
pub(super) fn server_is_authoritative() -> bool {
    SERVER_AUTHORITATIVE.with(|flag| flag.get())
}

thread_local! {
    static SERVER_AUTHORITATIVE: std::cell::Cell<bool> = const {
        std::cell::Cell::new(false)
    };
}

/// Force the deployment flag, for tests that must drive the online branch of
/// [`preserve_local_document`] without a daemon to probe.
#[cfg(test)]
pub(super) fn set_server_authoritative_for_test(authoritative: bool) {
    SERVER_AUTHORITATIVE.with(|flag| flag.set(authoritative));
}

/// Record what `GET /api/mcp/server` said about the deployment.
///
/// Split from the fetch so the parse is testable without a DOM.
fn note_serve_mode(body: &str) {
    if let Some(authoritative) = parse_server_authoritative(body) {
        SERVER_AUTHORITATIVE.with(|flag| flag.set(authoritative));
    }
}

/// Read `serveMode` out of the health response.
///
/// `None` when the field is absent — an older daemon, which is by definition
/// not an online one, so the caller leaves the conservative default in place.
fn parse_server_authoritative(body: &str) -> Option<bool> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let mode = parsed.get("serveMode")?.as_str()?;
    Some(mode == "online")
}

/// Ask the daemon once which deployment and encoder are available.
pub(super) fn probe_serve_mode<C: RepaintContext + 'static>(base: &str, inner: &Rc<RefCell<C>>) {
    let target = Rc::downgrade(inner);
    crate::live_sync::get(
        &format!("{base}/api/mcp/server"),
        Rc::new(move |body: String| {
            note_serve_mode(&body);
            let available = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| value["motionMp4"].as_bool())
                .unwrap_or(false);
            if let Some(target) = target.upgrade() {
                let mut context = target.borrow_mut();
                let host = context.host_mut();
                host.editor_state_mut()
                    .editor_ui
                    .motion_export
                    .mp4_available = available;
                host.mark_editor_state_dirty();
                let _ = context.repaint();
            }
        }),
    );
}

#[cfg(test)]
mod auto_resolve_tests {
    use super::*;

    use op_editor_core::CollabConnectionPhase;

    #[test]
    fn a_conflict_inside_an_active_session_resolves_itself() {
        assert!(auto_resolve_is_safe(
            true,
            CollabConnectionPhase::Active,
            false
        ));
    }

    #[test]
    fn no_conflict_means_nothing_to_resolve() {
        for phase in [
            CollabConnectionPhase::Idle,
            CollabConnectionPhase::Active,
            CollabConnectionPhase::ReadOnly,
        ] {
            assert!(!auto_resolve_is_safe(false, phase, false));
        }
    }

    #[test]
    fn every_non_active_phase_keeps_the_latch() {
        // Outside an Active session there is no authoritative server document
        // to accept and no `discarded_edit` projection to recover the losing
        // edit from, so auto-accepting would silently destroy unpushed work.
        // `Reconnecting` and `ReadOnly` look session-ish and are deliberately
        // included: neither can sequence an edit.
        for phase in [
            CollabConnectionPhase::Idle,
            CollabConnectionPhase::Starting,
            CollabConnectionPhase::Discovering,
            CollabConnectionPhase::Joining,
            CollabConnectionPhase::Authenticating,
            CollabConnectionPhase::Reconnecting,
            CollabConnectionPhase::ReadOnly,
            CollabConnectionPhase::Ended,
        ] {
            assert!(
                !auto_resolve_is_safe(true, phase, false),
                "{phase:?} must keep the existing explicit-resolution semantics"
            );
        }
    }
}

#[cfg(test)]
mod server_authority_tests {
    use super::{auto_resolve_is_safe, parse_server_authoritative};

    use op_editor_core::CollabConnectionPhase;

    #[test]
    fn an_online_daemon_is_authoritative() {
        assert_eq!(
            parse_server_authoritative(r#"{"running":true,"port":3100,"serveMode":"online"}"#),
            Some(true)
        );
    }

    #[test]
    fn a_local_or_managed_daemon_is_not() {
        for mode in ["local", "managed"] {
            assert_eq!(
                parse_server_authoritative(&format!(r#"{{"serveMode":"{mode}"}}"#)),
                Some(false),
                "{mode}"
            );
        }
    }

    #[test]
    fn a_daemon_that_does_not_report_a_mode_leaves_the_default_alone() {
        // An older daemon has no `serveMode`; it is by definition not an
        // online one, so the conservative default must survive the probe.
        for body in [r#"{"running":true}"#, "not json", "", "{}"] {
            assert_eq!(parse_server_authoritative(body), None, "{body:?}");
        }
    }

    #[test]
    fn a_server_authoritative_deployment_auto_resolves_outside_a_session() {
        // This is the M4 case: an online shared tenant has no collaboration
        // session, so without this the 409 latch would never lift and the
        // visitor would be frozen out of the document.
        for phase in [
            CollabConnectionPhase::Idle,
            CollabConnectionPhase::Starting,
            CollabConnectionPhase::Reconnecting,
            CollabConnectionPhase::ReadOnly,
        ] {
            assert!(
                auto_resolve_is_safe(true, phase, true),
                "{phase:?} must auto-resolve when the server is the sequencer"
            );
        }
    }

    #[test]
    fn no_conflict_never_resolves_however_authoritative_the_server_is() {
        assert!(!auto_resolve_is_safe(
            false,
            CollabConnectionPhase::Idle,
            true
        ));
        assert!(!auto_resolve_is_safe(
            false,
            CollabConnectionPhase::Active,
            true
        ));
    }

    #[test]
    fn a_peer_daemon_outside_a_session_still_latches() {
        // The local daemon holds the operator's file and arbitrates nothing;
        // auto-accepting there would silently discard unpushed work.
        for phase in [
            CollabConnectionPhase::Idle,
            CollabConnectionPhase::Reconnecting,
            CollabConnectionPhase::ReadOnly,
        ] {
            assert!(!auto_resolve_is_safe(true, phase, false), "{phase:?}");
        }
    }
}

#[cfg(test)]
mod auth_latch_tests {
    use crate::live_sync_glue::{auth_is_invalid, clear_auth_invalid, note_auth_invalid};

    #[test]
    fn a_refused_credential_stops_the_tab_pushing() {
        clear_auth_invalid();
        assert!(!auth_is_invalid(), "a healthy tab pushes");

        // The document on screen belongs to whoever WAS signed in; pushing it
        // after a switch would write one account's work into another's tenant.
        note_auth_invalid();
        assert!(auth_is_invalid());

        // Only the identity reset lifts it, once the tab has been rebuilt.
        clear_auth_invalid();
        assert!(!auth_is_invalid());
    }
}

#[cfg(test)]
mod preserve_tests {
    use op_editor_core::CollabNoticeKind;

    #[test]
    fn the_preserved_notice_has_its_own_message() {
        // Distinct from `EditConflictDiscarded`, which is a session rejecting
        // one edit; this is a whole-document accept with nothing rejected, and
        // the user needs to be told the overwritten copy still exists.
        assert_eq!(
            CollabNoticeKind::LocalEditPreserved.i18n_key(),
            "collab.status.localEditPreserved"
        );
        assert_ne!(
            CollabNoticeKind::LocalEditPreserved.i18n_key(),
            CollabNoticeKind::EditConflictDiscarded.i18n_key()
        );
    }

    // The 15-locale coverage of this key is asserted by `op-i18n`'s own
    // catalog-integrity tests, which own the locale set.
}

#[cfg(test)]
mod undo_recovery_tests {
    use op_editor_core::web_sync::WebSyncClient;

    #[test]
    fn every_apply_after_the_first_is_undoable() {
        // `apply_document_response` derives its `undoable` flag from
        // `WebSyncClient::initialized`, and hands it to
        // `replace_document_from_sync`, which picks
        // `replace_document_with_undo` when it is set.
        //
        // So the conflict auto-accept — which is always a later apply, never
        // the mount pull — restores through undo without a separate flag. This
        // pins the property the recovery path depends on.
        let mut client = WebSyncClient::new();
        assert!(
            !client.initialized(),
            "the mount pull must NOT be an undo step: it is starter -> daemon, \
             not a user edit being overwritten"
        );

        client.mark_applied(1);
        assert!(client.initialized());

        // Never reset by later applies, so a conflict accept at any version is
        // still undoable.
        for version in 2..10 {
            client.mark_applied(version);
            assert!(client.initialized(), "version {version}");
        }
    }
}
