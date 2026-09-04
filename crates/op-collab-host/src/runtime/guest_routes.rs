//! Guest-side connection entry points: discovered-row joins, the manual
//! invite-or-address field, and session resume.

use std::sync::Arc;

use op_collab_transport::{JoinIntent, ResumeHint};

use super::actor::EditorActor;
use super::relay::{guest_route_from_invite, guest_route_from_pairing_code, GuestConnectionRoute};
use super::types::{CollabRuntimeError, CollabRuntimeFailure};
use super::CollabRuntime;
use crate::host::CollabHost;

impl CollabRuntime {
    pub(super) fn join_discovered(
        &mut self,
        host: &mut impl CollabHost,
        discovery_id: &str,
    ) -> Result<(), CollabRuntimeError> {
        let discovered = self
            .discovered
            .get(discovery_id)
            .cloned()
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        if !discovered.compatible {
            return Err(CollabRuntimeError::invalid_session());
        }
        self.start_guest_route(
            host,
            GuestConnectionRoute::lan(discovered.addresses, Some(discovered.discovery_id), None),
            JoinIntent::New,
        )
    }

    pub(super) fn join_address(
        &mut self,
        host: &mut impl CollabHost,
        endpoint: &str,
    ) -> Result<(), CollabRuntimeError> {
        let endpoint = endpoint.trim();
        if endpoint.starts_with(op_collab_relay_protocol::RELAY_INVITE_PREFIX) {
            let route = guest_route_from_invite(
                endpoint,
                Arc::clone(&self.relay_locator_control_plane),
                self.preferred_region(),
            )?;
            return self.start_guest_route(host, route, JoinIntent::New);
        }
        if op_collab_relay_protocol::PairingCode::looks_like(endpoint) {
            let route = guest_route_from_pairing_code(
                endpoint,
                Arc::clone(&self.relay_locator_control_plane),
                self.preferred_region(),
            )?;
            return self.start_guest_route(host, route, JoinIntent::New);
        }
        let endpoint = endpoint
            .parse()
            .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::InvalidAddress))?;
        self.start_guest_route(
            host,
            GuestConnectionRoute::lan(vec![endpoint], None, None),
            JoinIntent::New,
        )
    }

    pub(super) fn retry_guest(
        &mut self,
        host: &mut impl CollabHost,
    ) -> Result<(), CollabRuntimeError> {
        let (route, intent) = self.guest_retry_target()?;
        self.note_guest_retry_started();
        self.spawn_guest_route(host, route, intent)
    }

    pub(super) fn guest_retry_target(
        &self,
    ) -> Result<(GuestConnectionRoute, JoinIntent), CollabRuntimeError> {
        let route = self
            .last_join
            .clone()
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        let Some(EditorActor::Guest(guest)) = self.actor.as_ref() else {
            return Err(CollabRuntimeError::invalid_session());
        };
        let core = guest.session.core();
        if core.state() != op_collab::GuestConnectionState::Disconnected {
            return Err(CollabRuntimeError::invalid_session());
        }
        let intent = JoinIntent::Resume(ResumeHint {
            participant_id: core.participant_id().clone(),
            peer_id: core.peer_id().clone(),
            peer_namespace: core.peer_namespace().clone(),
            role: core.role(),
        });
        // Discovery ids rotate while a session is live. Resume is bound by
        // the Noise static, verified ticket, and core resume identity, so the
        // initial mDNS id must not become a stale reconnect pin.
        let expected_remote_static = self
            .pinned_owner_static
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        Ok((
            route.retry_with_owner_static(expected_remote_static),
            intent,
        ))
    }
}
