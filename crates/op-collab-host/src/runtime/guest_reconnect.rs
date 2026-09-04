use std::time::{Duration, Instant};

use op_collab::GuestConnectionState;

use super::actor::EditorActor;
use super::types::CollabRuntimeFailure;
use super::CollabRuntime;
use crate::host::CollabHost;

const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(500);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);
const MAX_AUTOMATIC_RECONNECT_ATTEMPTS: u32 = 8;

#[derive(Default)]
pub(super) struct GuestReconnectState {
    consecutive_failures: u32,
    deadline: Option<Instant>,
    terminally_blocked: bool,
}

impl GuestReconnectState {
    fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.deadline = None;
        self.terminally_blocked = false;
    }

    fn cancel_deadline(&mut self) {
        self.deadline = None;
    }

    fn note_attempt_started(&mut self) {
        self.deadline = None;
        // An explicit Retry is the user's authorization to try again after a
        // terminal authentication/protocol result. Automatic attempts only
        // reach here when no terminal block was set.
        self.terminally_blocked = false;
        self.consecutive_failures = self.consecutive_failures.max(1);
    }

    fn block_automatic(&mut self) {
        self.deadline = None;
        self.terminally_blocked = true;
    }

    fn schedule_after_failure(&mut self, now: Instant) -> Option<Instant> {
        if self.terminally_blocked || self.consecutive_failures >= MAX_AUTOMATIC_RECONNECT_ATTEMPTS
        {
            self.deadline = None;
            return None;
        }
        let delay = reconnect_delay(self.consecutive_failures);
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        let deadline = now.checked_add(delay).unwrap_or(now);
        self.deadline = Some(deadline);
        Some(deadline)
    }

    fn take_due(&mut self, now: Instant) -> bool {
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.deadline = None;
            true
        } else {
            false
        }
    }
}

fn reconnect_delay(consecutive_failures: u32) -> Duration {
    let shift = consecutive_failures.min(31);
    INITIAL_RECONNECT_DELAY
        .checked_mul(1_u32 << shift)
        .unwrap_or(MAX_RECONNECT_DELAY)
        .min(MAX_RECONNECT_DELAY)
}

fn retryable_guest_failure(failure: CollabRuntimeFailure) -> bool {
    matches!(
        failure,
        CollabRuntimeFailure::RelayUnavailable
            | CollabRuntimeFailure::RelayRateLimited
            | CollabRuntimeFailure::Transport
            | CollabRuntimeFailure::ResourceLimit
    )
}

impl CollabRuntime {
    pub(super) fn reset_guest_reconnect(&mut self) {
        self.guest_reconnect.reset();
    }

    pub(super) fn note_guest_retry_started(&mut self) {
        self.guest_reconnect.note_attempt_started();
    }

    pub(super) fn block_guest_reconnect_for_terminal_failure(
        &mut self,
        failure: CollabRuntimeFailure,
    ) {
        if !retryable_guest_failure(failure) {
            self.guest_reconnect.block_automatic();
        }
    }

    /// Schedule another resume only for a retained, securely pinned guest.
    ///
    /// Authentication rejection, terminal session state, protocol failures,
    /// and unpinned routes remain user-visible stop conditions. A retry always
    /// reuses `retry_guest`, which preserves both the owner static pin and the
    /// core's participant/peer/namespace resume identity.
    pub(super) fn schedule_guest_reconnect(&mut self, failure: CollabRuntimeFailure) -> bool {
        let eligible = retryable_guest_failure(failure)
            && self.last_join.is_some()
            && self.pinned_owner_static.is_some()
            && matches!(
                self.actor.as_ref(),
                Some(EditorActor::Guest(guest))
                    if guest.session.core().state() == GuestConnectionState::Disconnected
            );
        if !eligible {
            if retryable_guest_failure(failure) {
                self.guest_reconnect.cancel_deadline();
            } else {
                self.guest_reconnect.block_automatic();
            }
            return false;
        }
        self.guest_reconnect
            .schedule_after_failure(Instant::now())
            .is_some()
    }

    pub(super) fn launch_due_guest_reconnect(&mut self, host: &mut impl CollabHost) -> bool {
        if !self.guest_reconnect.take_due(Instant::now()) {
            return false;
        }
        if let Err(error) = self.retry_guest(host) {
            // A due retry can become ineligible while waiting (for example the
            // account signed out). Surface that state once and do not spin.
            self.fail(host, error.failure);
        }
        true
    }

    /// Exact timer deadline for hosts that sleep instead of running a fixed
    /// collaboration poll cadence.
    pub fn next_reconnect_deadline(&self) -> Option<Instant> {
        self.guest_reconnect.deadline
    }
}

#[cfg(test)]
mod tests {
    use op_collab::{Bye, ByeReason, CollabMessage, Epoch, FrameEnvelope, SessionId};
    use op_editor_core::CollabConnectionPhase;

    use super::*;
    use crate::runtime::relay::GuestConnectionRoute;
    use crate::runtime::tests::{guest_runtime, SESSION};
    use crate::runtime::types::NetworkEvent;

    #[test]
    fn reconnect_backoff_doubles_and_caps_without_a_hot_loop() {
        let start = Instant::now();
        let mut state = GuestReconnectState::default();
        let expected = [500, 1_000, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000];

        for expected_ms in expected {
            let deadline = state
                .schedule_after_failure(start)
                .expect("the bounded automatic attempt is scheduled");
            assert_eq!(
                deadline.duration_since(start),
                Duration::from_millis(expected_ms)
            );
            assert!(!state.take_due(start));
            assert!(state.take_due(deadline));
        }
        assert_eq!(state.consecutive_failures, MAX_AUTOMATIC_RECONNECT_ATTEMPTS);
        assert!(state.schedule_after_failure(start).is_none());
        assert!(state.deadline.is_none());
        assert!(!state.take_due(start + Duration::from_secs(24 * 60 * 60)));
    }

    #[test]
    fn only_transient_transport_failures_are_automatic() {
        for failure in [
            CollabRuntimeFailure::Transport,
            CollabRuntimeFailure::RelayUnavailable,
            CollabRuntimeFailure::RelayRateLimited,
            CollabRuntimeFailure::ResourceLimit,
        ] {
            assert!(retryable_guest_failure(failure));
        }
        for failure in [
            CollabRuntimeFailure::AuthenticationUnavailable,
            CollabRuntimeFailure::TicketRejected,
            CollabRuntimeFailure::SecureKeyUnavailable,
            CollabRuntimeFailure::RelayInviteUnavailable,
            CollabRuntimeFailure::RelayInviteExpired,
            CollabRuntimeFailure::Protocol,
            CollabRuntimeFailure::OwnerIdentityRejected,
        ] {
            assert!(!retryable_guest_failure(failure));
        }
    }

    #[test]
    fn a_terminal_failure_blocks_the_following_transport_eof_until_manual_retry() {
        let start = Instant::now();
        let mut state = GuestReconnectState::default();
        state.block_automatic();
        assert!(state.schedule_after_failure(start).is_none());
        assert!(state.deadline.is_none());

        state.note_attempt_started();
        assert!(state.schedule_after_failure(start).is_some());
    }

    fn pin_retry_route(runtime: &mut CollabRuntime) {
        runtime.last_join = Some(GuestConnectionRoute::lan(
            vec!["127.0.0.1:43120".parse().unwrap()],
            Some("original-discovery-id".to_owned()),
            None,
        ));
        runtime.pinned_owner_static = Some([0x5a; 32]);
    }

    #[test]
    fn live_guest_transport_drop_schedules_a_visible_resume_deadline() {
        let (mut runtime, mut host, _commands, connection, _) = guest_runtime(4);
        pin_retry_route(&mut runtime);
        let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
            panic!("guest actor");
        };
        let expected_participant = guest.session.core().participant_id().clone();
        let expected_peer = guest.session.core().peer_id().clone();
        let expected_namespace = guest.session.core().peer_namespace().clone();
        let before = Instant::now();

        runtime.handle_event(
            NetworkEvent::ConnectionClosed {
                connection,
                failure: Some(CollabRuntimeFailure::Transport),
                remote_bye: None,
            },
            &mut host,
        );

        let deadline = runtime
            .next_reconnect_deadline()
            .expect("transient pinned disconnect schedules retry");
        assert!(deadline >= before + INITIAL_RECONNECT_DELAY);
        assert!(deadline <= Instant::now() + INITIAL_RECONNECT_DELAY);
        assert!(runtime.needs_poll());
        assert_eq!(
            host.editor_state().editor_ui.collab.phase,
            CollabConnectionPhase::Reconnecting
        );
        let (route, intent) = runtime.guest_retry_target().unwrap();
        let GuestConnectionRoute::Lan {
            discovery_id,
            expected_remote_static,
            ..
        } = route
        else {
            panic!("the reconnect route must retain the original LAN transport");
        };
        assert!(discovery_id.is_none());
        assert_eq!(expected_remote_static, Some([0x5a; 32]));
        let op_collab_transport::JoinIntent::Resume(hint) = intent else {
            panic!("automatic reconnect must resume, never create a new identity");
        };
        assert_eq!(hint.participant_id, expected_participant);
        assert_eq!(hint.peer_id, expected_peer);
        assert_eq!(hint.peer_namespace, expected_namespace);
    }

    #[test]
    fn a_due_deadline_attempts_once_and_auth_unavailability_cannot_hot_loop() {
        let (mut runtime, mut host, _commands, connection, _) = guest_runtime(4);
        pin_retry_route(&mut runtime);
        runtime.handle_event(
            NetworkEvent::ConnectionClosed {
                connection,
                failure: Some(CollabRuntimeFailure::Transport),
                remote_bye: None,
            },
            &mut host,
        );
        runtime.guest_reconnect.deadline = Some(Instant::now());

        assert!(runtime.launch_due_guest_reconnect(&mut host));
        // The open-source test backend intentionally has no ticket provider,
        // so the real resume launch fails its authentication precondition.
        // What matters here is that the due automatic attempt ran exactly once
        // and the terminal auth failure did not schedule itself again.
        assert!(runtime.pending_network_launch.is_none());
        assert!(runtime.next_reconnect_deadline().is_none());
        assert!(!runtime.launch_due_guest_reconnect(&mut host));
        assert_eq!(
            host.editor_state().editor_ui.collab.phase,
            CollabConnectionPhase::Reconnecting
        );

        runtime.shutdown(&mut host);
    }

    #[test]
    fn authentication_rejection_and_missing_pin_never_schedule_automatically() {
        let (mut rejected, mut rejected_host, _commands, connection, _) = guest_runtime(4);
        pin_retry_route(&mut rejected);
        rejected.handle_event(
            NetworkEvent::ConnectionClosed {
                connection,
                failure: Some(CollabRuntimeFailure::TicketRejected),
                remote_bye: None,
            },
            &mut rejected_host,
        );
        assert!(rejected.next_reconnect_deadline().is_none());

        let (mut unpinned, mut unpinned_host, _commands, connection, _) = guest_runtime(4);
        unpinned.last_join = Some(GuestConnectionRoute::lan(
            vec!["127.0.0.1:43120".parse().unwrap()],
            None,
            None,
        ));
        unpinned.handle_event(
            NetworkEvent::ConnectionClosed {
                connection,
                failure: None,
                remote_bye: None,
            },
            &mut unpinned_host,
        );
        assert!(unpinned.next_reconnect_deadline().is_none());
    }

    #[test]
    fn terminal_owner_bye_cancels_any_automatic_resume() {
        let (mut runtime, mut host, _commands, connection, _) = guest_runtime(4);
        pin_retry_route(&mut runtime);
        runtime.handle_event(
            NetworkEvent::Frame {
                connection,
                frame: FrameEnvelope::new(
                    SessionId::from(SESSION),
                    Epoch(1),
                    CollabMessage::Bye(Bye {
                        reason: ByeReason::OwnerLeft,
                    }),
                ),
            },
            &mut host,
        );
        runtime.handle_event(
            NetworkEvent::ConnectionClosed {
                connection,
                failure: None,
                remote_bye: None,
            },
            &mut host,
        );

        assert!(runtime.next_reconnect_deadline().is_none());
        assert_eq!(
            host.editor_state().editor_ui.collab.phase,
            CollabConnectionPhase::Ended
        );
    }
}
