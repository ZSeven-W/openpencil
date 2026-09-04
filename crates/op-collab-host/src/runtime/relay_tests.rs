#![cfg(test)]

use super::*;
use ed25519_dalek::{Signer, SigningKey};
use op_collab_relay_control_plane::SignedLocatorResponse;

struct CountingBootstrapProvider(std::sync::atomic::AtomicUsize);

impl RelayBootstrapProvider for CountingBootstrapProvider {
    fn load(&self) -> Result<std::sync::Arc<RelayBootstrap>, CollabRuntimeFailure> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(CollabRuntimeFailure::RelayUnavailable)
    }
}

struct SigningControlPlane(SigningKey);

impl SigningControlPlane {
    fn publish_for_test(
        &self,
        draft: OwnerPublishDraft,
        _ticket: &OpaqueCollabTicket,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
        let now = unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable)? / 1_000;
        let request = draft.request().clone();
        let key_id =
            LocatorKeyId::new("current").map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let claims = UnsignedRelayLocatorV1::new(
            request.home_region(),
            *request.route_id(),
            request.generation(),
            *request.owner_noise_static(),
            request.expected_discovery_id().clone(),
            now,
            now + 60,
            key_id.clone(),
        )
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let signature = self.0.sign(&claims.canonical_signing_bytes()).to_bytes();
        let locator = claims.attach_signature(
            LocatorSignature::new(signature).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        );
        let response = SignedLocatorResponse::decode(&locator.encode())
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let verifier = SingleKeyVerifier {
            key_id: key_id.clone(),
            key: self.0.verifying_key(),
        };
        let published = draft
            .complete(response, &verifier, now)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        published
            .invite()
            .verify(&verifier, now)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
    }
}

impl RelayLocatorControlPlane for SigningControlPlane {
    fn publish_route(
        &self,
        draft: OwnerPublishDraft,
        ticket: &OpaqueCollabTicket,
        _region: &RelayBootstrapRegion,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
        self.publish_for_test(draft, ticket)
    }

    fn publish_pairing_code(
        &self,
        _request: &op_collab_relay_control_plane::PairingPublishRequest,
        _ticket: &OpaqueCollabTicket,
        _region: &RelayBootstrapRegion,
    ) -> Result<(), CollabRuntimeFailure> {
        Err(CollabRuntimeFailure::RelayUnavailable)
    }

    fn claim_pairing_code(
        &self,
        _request: &op_collab_relay_control_plane::PairingClaimRequest,
        _ticket: &OpaqueCollabTicket,
        _region: &RelayBootstrapRegion,
    ) -> Result<op_collab_relay_protocol::SealedPairingInvite, CollabRuntimeFailure> {
        Err(CollabRuntimeFailure::RelayInviteUnavailable)
    }
}

/// Control plane that fails everything — parsed-invite tests only need a
/// value to thread through, never a live publish.
struct UnusedControlPlane;

impl RelayLocatorControlPlane for UnusedControlPlane {
    fn publish_route(
        &self,
        _draft: OwnerPublishDraft,
        _ticket: &OpaqueCollabTicket,
        _region: &RelayBootstrapRegion,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
        Err(CollabRuntimeFailure::RelayUnavailable)
    }

    fn publish_pairing_code(
        &self,
        _request: &op_collab_relay_control_plane::PairingPublishRequest,
        _ticket: &OpaqueCollabTicket,
        _region: &RelayBootstrapRegion,
    ) -> Result<(), CollabRuntimeFailure> {
        Err(CollabRuntimeFailure::RelayUnavailable)
    }

    fn claim_pairing_code(
        &self,
        _request: &op_collab_relay_control_plane::PairingClaimRequest,
        _ticket: &OpaqueCollabTicket,
        _region: &RelayBootstrapRegion,
    ) -> Result<op_collab_relay_protocol::SealedPairingInvite, CollabRuntimeFailure> {
        Err(CollabRuntimeFailure::RelayInviteUnavailable)
    }
}

struct SingleKeyVerifier {
    key_id: LocatorKeyId,
    key: ed25519_dalek::VerifyingKey,
}

impl RelayLocatorVerifier for SingleKeyVerifier {
    fn verify(
        &self,
        key_id: &LocatorKeyId,
        canonical_signing_bytes: &[u8],
        signature: &[u8; 64],
    ) -> bool {
        key_id == &self.key_id
            && self
                .key
                .verify_strict(
                    canonical_signing_bytes,
                    &ed25519_dalek::Signature::from_bytes(signature),
                )
                .is_ok()
    }
}

#[test]
fn development_unsigned_requires_all_three_gates() {
    for (debug, loopback, value, expected) in [
        (true, true, Some("1".to_owned()), true),
        (true, false, Some("1".to_owned()), false),
        (false, true, Some("1".to_owned()), false),
        (true, true, Some("true".to_owned()), false),
        (true, true, None, false),
    ] {
        assert_eq!(
            development_unsigned_opt_in(debug, loopback, value),
            expected
        );
    }
}

#[test]
fn home_region_prefers_env_override_and_falls_back_to_the_preference() {
    assert_eq!(
        resolve_home_region(None, RelayRegion::Cn).unwrap(),
        RelayRegion::Cn
    );
    assert_eq!(
        resolve_home_region(None, RelayRegion::Global).unwrap(),
        RelayRegion::Global
    );
    assert_eq!(
        resolve_home_region(Some("cn"), RelayRegion::Global).unwrap(),
        RelayRegion::Cn
    );
    assert_eq!(
        resolve_home_region(Some("global"), RelayRegion::Cn).unwrap(),
        RelayRegion::Global
    );
    let invalid = resolve_home_region(Some("moon"), RelayRegion::Cn).unwrap_err();
    assert_eq!(
        invalid.failure,
        CollabRuntimeFailure::RelayRegionUnavailable,
        "an unrecognized override must not silently re-home the session"
    );
}

#[test]
fn protocol_region_round_trips_the_ui_region() {
    for region in [RelayRegion::Cn, RelayRegion::Global] {
        assert_eq!(protocol_region(ui_region(region)), region);
    }
}

#[test]
fn injected_control_plane_publishes_a_ticket_bound_owner_route() {
    let signing = SigningKey::from_bytes(&[9_u8; 32]);
    let verifying_key = signing.verifying_key();
    let draft = OwnerPublishDraft::generate(
        RelayRegion::Cn,
        OwnerNoiseStatic::new([2_u8; 32]).unwrap(),
        ExpectedDiscoveryId::new("stable-relay-prelude").unwrap(),
        RelayPublishLifetime::new(60).unwrap(),
    )
    .unwrap();
    let ticket = OpaqueCollabTicket::new(b"header.payload.signature".to_vec()).expect("ticket");
    let route = SigningControlPlane(signing)
        .publish_for_test(draft, &ticket)
        .expect("published route");
    assert_eq!(route.locator().claims().home_region(), RelayRegion::Cn);
    assert_eq!(
        route.locator().claims().owner_noise_static().as_bytes(),
        &[2_u8; 32]
    );
    let fragment = RelayInviteV1::new(&route).to_fragment();
    let invite = RelayInviteV1::from_fragment(&fragment).unwrap();
    let verifier = SingleKeyVerifier {
        key_id: LocatorKeyId::new("current").unwrap(),
        key: verifying_key,
    };
    let now = unix_time_ms().unwrap() / 1_000;
    let verified = invite.verify(&verifier, now).expect("signed invite");
    assert_eq!(verified.locator().claims().home_region(), RelayRegion::Cn);

    let provider = std::sync::Arc::new(CountingBootstrapProvider(
        std::sync::atomic::AtomicUsize::new(0),
    ));
    let route = guest_route_from_parsed_invite(
        invite,
        provider.clone(),
        std::sync::Arc::new(UnusedControlPlane),
    );
    assert_eq!(
        provider.0.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "the UI invite parser must not fetch bootstrap HTTP"
    );
    assert_eq!(
        route.connection_path(),
        CollabConnectionPathUi::Relay {
            home_region: CollabRelayRegion::China
        }
    );
}

#[test]
fn cn_home_region_stays_cn_in_the_collaboration_ui() {
    assert_eq!(ui_region(RelayRegion::Cn), CollabRelayRegion::China);
}

#[test]
fn relay_setup_failures_use_connect_notices() {
    use op_editor_core::{CollabConnectErrorUi, CollabNoticeKind};

    for (failure, expected) in [
        (
            CollabRuntimeFailure::RelayInviteUnavailable,
            CollabConnectErrorUi::InviteUnavailable,
        ),
        (
            CollabRuntimeFailure::RelayInviteInvalid,
            CollabConnectErrorUi::InviteInvalid,
        ),
        (
            CollabRuntimeFailure::RelayInviteExpired,
            CollabConnectErrorUi::InviteExpired,
        ),
        (
            CollabRuntimeFailure::RelayUnavailable,
            CollabConnectErrorUi::RelayUnavailable,
        ),
        (
            CollabRuntimeFailure::RelayNotConfigured,
            CollabConnectErrorUi::RelayNotConfigured,
        ),
        (
            CollabRuntimeFailure::RelayRegionUnavailable,
            CollabConnectErrorUi::RegionUnavailable,
        ),
    ] {
        assert_eq!(
            super::super::failure::disconnect_notice(failure),
            CollabNoticeKind::Connect(expected)
        );
    }
}

#[test]
fn pairing_code_route_carries_the_code_and_skips_bootstrap_http() {
    let provider = std::sync::Arc::new(CountingBootstrapProvider(
        std::sync::atomic::AtomicUsize::new(0),
    ));
    let code = op_collab_relay_protocol::PairingCode::parse("2A2C4E6G8J").unwrap();
    let route = GuestConnectionRoute::Relay(Box::new(RelayGuestRequest {
        secret: RelayJoinSecret::pairing(code),
        home_region: RelayRegion::Global,
        provider: provider.clone(),
        control_plane: std::sync::Arc::new(UnusedControlPlane),
    }));
    let retry_route = route.clone();
    assert_eq!(
        provider.0.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "building a pairing route must not fetch bootstrap HTTP"
    );
    assert_eq!(
        route.connection_path(),
        CollabConnectionPathUi::Relay {
            home_region: CollabRelayRegion::Global
        }
    );
    let (GuestConnectionRoute::Relay(original), GuestConnectionRoute::Relay(retry)) =
        (&route, &retry_route)
    else {
        panic!("pairing code routes must stay on relay");
    };
    let (
        RelayJoinSecret::Pairing {
            claimed: original_claim,
            ..
        },
        RelayJoinSecret::Pairing {
            claimed: retry_claim,
            ..
        },
    ) = (&original.secret, &retry.secret)
    else {
        panic!("pairing code routes must preserve the pairing secret");
    };
    assert!(std::sync::Arc::ptr_eq(original_claim, retry_claim));
}

#[test]
fn malformed_pairing_code_fails_as_invalid_invite_before_any_setup() {
    for rejected in [
        "A2C4E6G8J",  // one char short
        "A2C4E6G8J0", // right shape, no region tag — a 10-char hostname
    ] {
        let result = guest_route_from_pairing_code(
            rejected,
            std::sync::Arc::new(UnusedControlPlane),
            RelayRegion::Cn,
        );
        match result {
            Ok(_) => panic!("{rejected:?} must not become a pairing route"),
            Err(error) => {
                assert_eq!(error.failure, CollabRuntimeFailure::RelayInviteInvalid)
            }
        }
    }
    // A region-tagged code passes parse + region derivation without any
    // home-region environment: the route's region comes from the code, not
    // from the caller's service-region preference (deliberately Cn here).
    // With build-injected hubs the route materializes; an uninjected build
    // may only fail at the bootstrap-configuration gate. Either way this
    // proves no region environment is consulted.
    let result = guest_route_from_pairing_code(
        "2A2C4E6G8J",
        std::sync::Arc::new(UnusedControlPlane),
        RelayRegion::Cn,
    );
    match result {
        Ok(route) => assert_eq!(
            route.connection_path(),
            CollabConnectionPathUi::Relay {
                home_region: CollabRelayRegion::Global
            }
        ),
        Err(error) => assert_eq!(
            error.failure,
            CollabRuntimeFailure::RelayNotConfigured,
            "the only acceptable failure is the bootstrap-configuration gate"
        ),
    }
}
