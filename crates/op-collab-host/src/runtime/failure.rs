//! Fail-closed handling for setup and network failures, plus the
//! failure-to-notice mappings. Split off `runtime/effects.rs` at the
//! 800-line cap; pure code motion.

use op_editor_core::{
    CollabConnectErrorUi, CollabConnectionPhase, CollabNoticeKind, CollabRejectUiCode,
};

use super::actor::{set_guest_ui, EditorActor};
use super::types::{CollabRuntimeFailure, CollabStatusEvent};
use super::CollabRuntime;
use crate::host::CollabHost;

impl CollabRuntime {
    pub(super) fn fail(&mut self, host: &mut impl CollabHost, failure: CollabRuntimeFailure) {
        self.block_guest_reconnect_for_terminal_failure(failure);
        if let Some(notice) = setup_failure_notice(failure) {
            self.push_status(CollabStatusEvent::Failed(failure));
            if self.actor.is_none() {
                self.retire_workers();
                self.pending_guest = None;
                host.disable_collaboration_ids();
                host.editor_state_mut()
                    .editor_ui
                    .collab
                    .set_phase(CollabConnectionPhase::Idle);
            }
            self.set_notice(host, notice);
            return;
        }
        if matches!(
            failure,
            CollabRuntimeFailure::ResourceLimit | CollabRuntimeFailure::Transport
        ) {
            self.fail_network(host, failure);
            return;
        }
        let notice = if failure.is_authentication() {
            CollabNoticeKind::Reject(CollabRejectUiCode::Authentication)
        } else if failure == CollabRuntimeFailure::ResourceLimit {
            CollabNoticeKind::Reject(CollabRejectUiCode::ResourceLimit)
        } else {
            CollabNoticeKind::Reject(CollabRejectUiCode::Unknown)
        };
        self.set_notice(host, notice);
        self.push_status(CollabStatusEvent::Failed(failure));
        if matches!(self.actor, Some(EditorActor::Guest(_))) {
            if let Some(EditorActor::Guest(guest)) = self.actor.as_ref() {
                set_guest_ui(host, guest, CollabConnectionPhase::Reconnecting);
            }
        } else if self.actor.is_none() {
            self.retire_workers();
            self.pending_guest = None;
            host.disable_collaboration_ids();
            host.editor_state_mut()
                .editor_ui
                .collab
                .set_phase(CollabConnectionPhase::Idle);
        }
        host.mark_editor_state_dirty();
    }

    /// Fail session-wide network errors closed: owners keep the standalone document;
    /// guests keep confirmed and pending state read-only for an idempotent retry.
    pub(super) fn fail_network(
        &mut self,
        host: &mut impl CollabHost,
        failure: CollabRuntimeFailure,
    ) {
        self.push_status(CollabStatusEvent::Failed(failure));
        match self.actor.as_ref() {
            Some(EditorActor::Owner(_)) => {
                self.leave(host);
            }
            Some(EditorActor::Guest(_)) => {
                self.retire_workers();
                self.pending_guest = None;
                self.transaction_active = false;
                let mut session_ended = false;
                let mut guest_disconnected = false;
                if let Some(EditorActor::Guest(guest)) = self.actor.as_mut() {
                    let ended =
                        guest.session.core().state() == op_collab::GuestConnectionState::Ended;
                    let _ = guest.session.disconnect(host);
                    guest.connection = None;
                    if ended {
                        set_guest_ui(host, guest, CollabConnectionPhase::Ended);
                    } else {
                        set_guest_ui(host, guest, CollabConnectionPhase::Reconnecting);
                        self.push_status(CollabStatusEvent::Reconnecting);
                        guest_disconnected = true;
                    }
                    session_ended = ended;
                }
                if session_ended {
                    self.clear_discarded_stash(host);
                    self.reset_guest_reconnect();
                } else if guest_disconnected {
                    self.schedule_guest_reconnect(failure);
                }
            }
            None => {
                self.retire_workers();
                self.pending_guest = None;
                self.transaction_active = false;
                host.disable_collaboration_ids();
                host.editor_state_mut()
                    .editor_ui
                    .collab
                    .set_phase(CollabConnectionPhase::Idle);
                self.reset_guest_reconnect();
            }
        }
        self.set_notice(host, disconnect_notice(failure));
        host.mark_editor_state_dirty();
    }

    pub(super) fn network_stopped(&mut self, host: &mut impl CollabHost) {
        if self.network.is_some() {
            self.fail_network(host, CollabRuntimeFailure::Transport);
        }
    }
}

pub(super) fn disconnect_notice(failure: CollabRuntimeFailure) -> CollabNoticeKind {
    match failure {
        CollabRuntimeFailure::RelayInviteUnavailable => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::InviteUnavailable)
        }
        CollabRuntimeFailure::RelayInviteInvalid => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::InviteInvalid)
        }
        CollabRuntimeFailure::RelayInviteExpired => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::InviteExpired)
        }
        CollabRuntimeFailure::RelayUnavailable => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::RelayUnavailable)
        }
        CollabRuntimeFailure::RelayNotConfigured => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::RelayNotConfigured)
        }
        CollabRuntimeFailure::RelayRegionUnavailable => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::RegionUnavailable)
        }
        CollabRuntimeFailure::RelayRateLimited => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::RateLimited)
        }
        CollabRuntimeFailure::TicketRejected => CollabNoticeKind::TicketExpired,
        CollabRuntimeFailure::OwnerIdentityRejected => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::OwnerNotConfirmed)
        }
        CollabRuntimeFailure::AuthenticationUnavailable => {
            CollabNoticeKind::Reject(CollabRejectUiCode::Authentication)
        }
        CollabRuntimeFailure::ResourceLimit => {
            CollabNoticeKind::Reject(CollabRejectUiCode::ResourceLimit)
        }
        // A refused key store is a hard local condition, not a dropped
        // session — the reconnect copy would mislead.
        CollabRuntimeFailure::SecureKeyUnavailable => {
            CollabNoticeKind::Connect(CollabConnectErrorUi::SecureKeyUnavailable)
        }
        CollabRuntimeFailure::ClockUnavailable
        | CollabRuntimeFailure::InvalidAddress
        | CollabRuntimeFailure::InvalidSession
        | CollabRuntimeFailure::Transport
        | CollabRuntimeFailure::Protocol => CollabNoticeKind::DisconnectedReadOnly,
    }
}

fn setup_failure_notice(failure: CollabRuntimeFailure) -> Option<CollabNoticeKind> {
    match failure {
        CollabRuntimeFailure::RelayInviteUnavailable => Some(CollabNoticeKind::Connect(
            CollabConnectErrorUi::InviteUnavailable,
        )),
        CollabRuntimeFailure::RelayInviteInvalid => Some(CollabNoticeKind::Connect(
            CollabConnectErrorUi::InviteInvalid,
        )),
        CollabRuntimeFailure::RelayInviteExpired => Some(CollabNoticeKind::Connect(
            CollabConnectErrorUi::InviteExpired,
        )),
        CollabRuntimeFailure::RelayUnavailable => Some(CollabNoticeKind::Connect(
            CollabConnectErrorUi::RelayUnavailable,
        )),
        CollabRuntimeFailure::RelayNotConfigured => Some(CollabNoticeKind::Connect(
            CollabConnectErrorUi::RelayNotConfigured,
        )),
        CollabRuntimeFailure::RelayRegionUnavailable => Some(CollabNoticeKind::Connect(
            CollabConnectErrorUi::RegionUnavailable,
        )),
        CollabRuntimeFailure::RelayRateLimited => {
            Some(CollabNoticeKind::Connect(CollabConnectErrorUi::RateLimited))
        }
        // A declined host is a completed setup decision, not a live session
        // that broke: the runtime retires the workers and returns to Idle
        // instead of offering a reconnect.
        CollabRuntimeFailure::OwnerIdentityRejected => Some(CollabNoticeKind::Connect(
            CollabConnectErrorUi::OwnerNotConfirmed,
        )),
        CollabRuntimeFailure::SecureKeyUnavailable => Some(CollabNoticeKind::Connect(
            CollabConnectErrorUi::SecureKeyUnavailable,
        )),
        _ => None,
    }
}
