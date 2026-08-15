//! Controlled collaboration fixtures for downstream test suites.
//!
//! `op-host-services` needs to drive the REAL begin → install → reject/fail
//! paths to prove what its ingest does with a session's verdict. Without a
//! fixture its tests can only assert against a hand-built `LocalEditOutcome`,
//! which passes whether or not the wiring behind it is correct.
//!
//! Standing up an activated owner session needs `crate::runtime`'s private
//! actor and channel internals, so this module lives inside it and re-exports
//! exactly two capabilities:
//!
//! 1. [`owner_session`] — an activated owner runtime over a caller-supplied
//!    baseline document, ready for `begin_local_edit`.
//! 2. [`owner_session_with_saturated_command_lane`] — the same, with the
//!    outbound command lane already full, so the next commit cannot be
//!    delivered and the runtime falls back to standalone.
//!
//! ## The lane guard is not optional
//!
//! Both constructors return an [`OwnerLaneGuard`] alongside the runtime, and
//! the caller MUST hold it for as long as it drives the runtime. The guard
//! owns the channel's receiver, and a bounded `SyncSender` reports two
//! different failures: `Full` when the lane is saturated, `Disconnected` once
//! the receiver is gone (`network.rs`'s `NetworkCommandSendError`, which the
//! runtime maps to `ResourceLimit` and `Transport` respectively). Drop the
//! guard early and the "full lane" test silently becomes a "dead channel"
//! test — a different code path with a different projected failure.
//!
//! ## Not a production surface
//!
//! Gated behind `#[cfg(any(test, feature = "test-support"))]`, and the feature
//! is turned on only by a `dev-dependencies` entry. A normal build of this
//! crate compiles none of it, so it cannot reach the shipped API.

use std::sync::mpsc::Receiver;

use op_collab::{ConnectionKey, Epoch, Role, SessionId, VerifiedAuthMetadata};
use op_editor_core::PenDocument;

use super::actor::{set_owner_ui, EditorActor, OwnerActor};
use super::network::owner_command_channel_with_capacity_for_test;
use super::types::OwnerNetworkCommand;
use super::CollabRuntime;
use crate::host::HeadlessCollabHost;

/// Session id every fixture runs under.
const FIXTURE_SESSION: &str = "collab-host-test-support";

/// Room for exactly one command, which the saturating constructor then uses.
const FIXTURE_COMMAND_CAPACITY: usize = 1;

/// Keeps the owner's outbound command lane connected.
///
/// Hold it for the whole test — see the module docs for why dropping it early
/// silently changes which failure the runtime reports.
#[must_use = "dropping the guard disconnects the lane and changes the failure \
              the runtime reports from Full to Disconnected"]
pub struct OwnerLaneGuard {
    _commands: Receiver<OwnerNetworkCommand>,
}

impl OwnerLaneGuard {
    /// Drain and count commands emitted since the previous observation.
    ///
    /// This exists only for downstream runtime-wiring tests that must prove a
    /// UI gesture closes into exactly one collaboration commit.
    pub fn drain_command_count(&self) -> usize {
        self._commands.try_iter().count()
    }
}

/// Build an owner runtime with one admitted editor peer, ready for
/// `begin_local_edit`.
///
/// `baseline` is the document the session is activated over, and it must be
/// the same document the caller's own editor holds. The owner core validates
/// each commit against the document the capture opened on, so a session
/// activated over a *different* document reports a candidate mismatch instead
/// of the delivery outcome the caller is trying to observe.
pub fn owner_session(baseline: PenDocument) -> (CollabRuntime, OwnerLaneGuard) {
    build(baseline, false)
}

/// [`owner_session`] with the outbound command lane already full.
///
/// The next commit cannot be handed to the network worker, so the runtime
/// retires the session and `finish_local_edit` reports
/// `Failed { document_rolled_back: false }` — the standalone fallback, which
/// deliberately KEEPS the edit because the user's work is still theirs even
/// though the session is gone. The projected failure is `ResourceLimit`, which
/// is how a caller can tell this apart from a dead channel.
pub fn owner_session_with_saturated_command_lane(
    baseline: PenDocument,
) -> (CollabRuntime, OwnerLaneGuard) {
    build(baseline, true)
}

fn build(baseline: PenDocument, saturate: bool) -> (CollabRuntime, OwnerLaneGuard) {
    let mut host = HeadlessCollabHost::new();
    host.editor_state_mut().doc = baseline;
    let mut owner = OwnerActor::new(
        SessionId::from(FIXTURE_SESSION),
        Epoch(1),
        fixture_auth(0),
        &mut host,
    )
    .expect("owner actor");
    let peer = ConnectionKey::new(2).expect("non-zero connection");
    let grant = owner
        .grant_new_peer(fixture_auth(1), Role::Editor)
        .expect("peer grant");
    owner
        .session
        .activate_peer(peer, grant, &host)
        .expect("activate peer");
    owner.connections.insert(peer);
    set_owner_ui(&mut host, &owner);

    let (network, commands) =
        owner_command_channel_with_capacity_for_test(FIXTURE_COMMAND_CAPACITY);
    let mut runtime = CollabRuntime::new();
    runtime.network = Some(network);
    runtime.actor = Some(EditorActor::Owner(Box::new(owner)));
    if saturate {
        runtime
            .send_owner(OwnerNetworkCommand::Close {
                connection: ConnectionKey::new(99).expect("non-zero connection"),
            })
            .expect("the first send fills the lane");
    }
    // `host` is dropped here on purpose: the actor owns its own session state,
    // and this host existed only to seed the baseline and take the projection.
    (
        runtime,
        OwnerLaneGuard {
            _commands: commands,
        },
    )
}

fn fixture_auth(index: usize) -> VerifiedAuthMetadata {
    VerifiedAuthMetadata {
        issuer: "https://issuer.example".into(),
        subject: format!("subject-{index}"),
        device_id: format!("device-{index}"),
        proof_binding: format!("binding-{index}"),
        expires_at_unix_ms: 10_000,
        display_name: None,
        avatar_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::types::CollabRuntimeFailure;

    /// The bug this module's guard exists to prevent: a caller that lets the
    /// receiver drop gets `Disconnected` (projected as `Transport`) where it
    /// meant to test `Full` (projected as `ResourceLimit`).
    #[test]
    fn a_saturated_lane_reports_resource_limit_while_the_guard_is_held() {
        let (runtime, guard) =
            owner_session_with_saturated_command_lane(op_editor_core::EditorState::new().doc);

        let error = runtime
            .send_owner(OwnerNetworkCommand::Close {
                connection: ConnectionKey::new(98).expect("non-zero connection"),
            })
            .expect_err("the lane is already full");
        assert_eq!(
            error.failure,
            CollabRuntimeFailure::ResourceLimit,
            "a full lane is a resource limit, not a transport failure"
        );
        drop(guard);
    }

    #[test]
    fn dropping_the_guard_turns_the_same_send_into_a_transport_failure() {
        // Pins the distinction the guard protects, so a future refactor that
        // silently drops the receiver fails here rather than in a downstream
        // test that looks like it is passing.
        let (runtime, guard) =
            owner_session_with_saturated_command_lane(op_editor_core::EditorState::new().doc);
        drop(guard);

        let error = runtime
            .send_owner(OwnerNetworkCommand::Close {
                connection: ConnectionKey::new(98).expect("non-zero connection"),
            })
            .expect_err("the receiver is gone");
        assert_eq!(error.failure, CollabRuntimeFailure::Transport);
    }

    #[test]
    fn an_unsaturated_lane_accepts_one_command() {
        let (runtime, _guard) = owner_session(op_editor_core::EditorState::new().doc);
        assert!(runtime
            .send_owner(OwnerNetworkCommand::Close {
                connection: ConnectionKey::new(98).expect("non-zero connection"),
            })
            .is_ok());
    }
}
