use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::num::NonZeroU64;
use std::sync::Arc;

use op_auth_bridge::OpaqueCollabTicket;
use op_collab::{Epoch, SessionId};
#[cfg(any(test, debug_assertions))]
use op_collab_relay_client::DEFAULT_OWNER_LANE_COUNT;
use op_collab_relay_client::{RelayEndpoint, RelayGuestBridge, RelayHandshake, RelayOwnerBridge};
use op_collab_relay_control_plane::{
    OwnerPublishDraft, PairingClaimError, PairingClaimRequest, PairingPublishRequest,
    RelayLocatorHttpClient, RelayPublishLifetime, MAX_PAIRING_CODE_TTL_SECS,
};
use op_collab_relay_protocol::{
    ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic, PairingCode,
    RelayInviteV1, RelayLocatorVerifier, RelayRegion, RouteCapability, RouteId,
    SealedPairingInvite, UnsignedRelayLocatorV1, VerifiedRelayRoute, MAX_PAIRING_LIFETIME_SECS,
};
use op_collab_transport::{DeviceStaticKey, ServerPrelude};
use op_editor_core::{CollabConnectionPathUi, CollabInviteCode, CollabRelayRegion};

use super::auth::{unix_time_ms, LocalAdmission};
use super::relay_bootstrap::RelayBootstrap;
use super::relay_bootstrap::{
    bootstrap_provider, Ed25519LocatorVerifier, RelayBootstrapProvider, RelayBootstrapRegion,
};
use super::types::{CollabRuntimeError, CollabRuntimeFailure};

mod control_plane_failure;
mod guest_runtime;

use control_plane_failure::{control_plane_failure, report_control_plane_failure};

const RELAY_HOME_REGION_ENV: &str = "OPENPENCIL_COLLAB_RELAY_HOME_REGION";
#[cfg(any(test, debug_assertions))]
const RELAY_DEV_UNSIGNED_ENV: &str = "OPENPENCIL_COLLAB_RELAY_DEV_UNSIGNED";
const DEBUG_LOCATOR_KEY_ID: &str = "openpencil-debug-unsigned";
const OWNER_RELAY_READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(22);

#[derive(Clone)]
pub(super) enum GuestConnectionRoute {
    Lan {
        addresses: Vec<SocketAddr>,
        discovery_id: Option<String>,
        expected_remote_static: Option<[u8; 32]>,
    },
    Relay(Box<RelayGuestRequest>),
}

impl GuestConnectionRoute {
    pub(super) fn lan(
        addresses: Vec<SocketAddr>,
        discovery_id: Option<String>,
        expected_remote_static: Option<[u8; 32]>,
    ) -> Self {
        Self::Lan {
            addresses,
            discovery_id,
            expected_remote_static,
        }
    }

    pub(super) fn retry_with_owner_static(&self, owner_static: [u8; 32]) -> Self {
        match self {
            Self::Lan { addresses, .. } => Self::Lan {
                addresses: addresses.clone(),
                discovery_id: None,
                expected_remote_static: Some(owner_static),
            },
            Self::Relay(request) => Self::Relay(request.clone()),
        }
    }

    pub(super) fn status_endpoint(&self) -> Option<SocketAddr> {
        match self {
            Self::Lan { addresses, .. } => addresses.first().copied(),
            Self::Relay(_) => None,
        }
    }

    pub(super) fn connection_path(&self) -> CollabConnectionPathUi {
        match self {
            Self::Lan { .. } => CollabConnectionPathUi::Lan,
            Self::Relay(request) => CollabConnectionPathUi::Relay {
                home_region: ui_region(request.home_region),
            },
        }
    }
}

/// What the guest holds before the relay route is resolved: either the full
/// invite, or a short pairing code that redeems to one on the network worker.
#[derive(Clone)]
pub(super) enum RelayJoinSecret {
    Invite(Box<RelayInviteV1>),
    Pairing {
        code: PairingCode,
        /// Shared by every retry clone so a redeemed one-time code is never
        /// spent again merely because the transport dropped.
        claimed: Arc<std::sync::Mutex<Option<RelayInviteV1>>>,
    },
}

impl RelayJoinSecret {
    fn pairing(code: PairingCode) -> Self {
        Self::Pairing {
            code,
            claimed: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

#[derive(Clone)]
pub(super) struct RelayGuestRequest {
    secret: RelayJoinSecret,
    home_region: RelayRegion,
    provider: std::sync::Arc<dyn RelayBootstrapProvider>,
    control_plane: std::sync::Arc<dyn RelayLocatorControlPlane>,
}

pub(super) struct RelayOwnerRequest {
    home_region: RelayRegion,
    provider: std::sync::Arc<dyn RelayBootstrapProvider>,
    control_plane: std::sync::Arc<dyn RelayLocatorControlPlane>,
}

/// Authenticated control-plane boundary for owner route publication.
///
/// The draft keeps the bearer route capability on-device. Implementations send
/// only its bounded publish request and the short-lived collaboration ticket;
/// the service independently verifies ticket-to-owner-key binding and delegates
/// signing to its HSM/KMS. The desktop never receives a signing key.
pub(crate) trait RelayLocatorControlPlane: Send + Sync {
    fn publish_route(
        &self,
        draft: OwnerPublishDraft,
        ticket: &OpaqueCollabTicket,
        region: &RelayBootstrapRegion,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure>;

    /// Store a sealed invite under a short pairing code. Best-effort: a
    /// failure must leave the long invite flow untouched.
    fn publish_pairing_code(
        &self,
        request: &PairingPublishRequest,
        ticket: &OpaqueCollabTicket,
        region: &RelayBootstrapRegion,
    ) -> Result<(), CollabRuntimeFailure>;

    /// Redeem a short pairing code id for the sealed invite blob.
    fn claim_pairing_code(
        &self,
        request: &PairingClaimRequest,
        ticket: &OpaqueCollabTicket,
        region: &RelayBootstrapRegion,
    ) -> Result<SealedPairingInvite, CollabRuntimeFailure>;
}

pub(crate) struct EnvironmentRelayLocatorControlPlane;

impl RelayLocatorControlPlane for EnvironmentRelayLocatorControlPlane {
    fn publish_route(
        &self,
        draft: OwnerPublishDraft,
        ticket: &OpaqueCollabTicket,
        region: &RelayBootstrapRegion,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
        let verifier = region.locator_verifier.clone();
        let client = locator_http_client(region)?;
        let published = client
            .publish(draft, ticket)
            .map_err(|error| control_plane_failure("publish_route", error))?;
        let now = unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable)? / 1_000;
        published
            .invite()
            .verify(&verifier, now)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
    }

    fn publish_pairing_code(
        &self,
        request: &PairingPublishRequest,
        ticket: &OpaqueCollabTicket,
        region: &RelayBootstrapRegion,
    ) -> Result<(), CollabRuntimeFailure> {
        let client = locator_http_client(region)?;
        client
            .publish_pairing_code(request, ticket)
            .map_err(|error| control_plane_failure("publish_pairing_code", error))
    }

    fn claim_pairing_code(
        &self,
        request: &PairingClaimRequest,
        ticket: &OpaqueCollabTicket,
        region: &RelayBootstrapRegion,
    ) -> Result<SealedPairingInvite, CollabRuntimeFailure> {
        let client = locator_http_client(region)?;
        client.claim_pairing_code(request, ticket).map_err(|error| {
            let failure = match error {
                PairingClaimError::NotFound => CollabRuntimeFailure::RelayInviteUnavailable,
                PairingClaimError::Rejected => CollabRuntimeFailure::RelayInviteInvalid,
                PairingClaimError::Unauthorized => CollabRuntimeFailure::TicketRejected,
                PairingClaimError::RateLimited => CollabRuntimeFailure::RelayRateLimited,
                PairingClaimError::TransportUnavailable => CollabRuntimeFailure::RelayUnavailable,
            };
            report_control_plane_failure("claim_pairing_code", failure, &error);
            failure
        })
    }
}

pub(super) struct OwnerRelayRuntime {
    listener: TcpListener,
    prelude: std::sync::Arc<ServerPrelude>,
    /// Short pairing code, present only when the control plane accepted the
    /// sealed publish. The long `opc1_` fragment is deliberately never
    /// surfaced — it is unusable for human sharing.
    invite: Option<CollabInviteCode>,
    path: CollabConnectionPathUi,
    bridge: RelayOwnerBridge,
}

impl OwnerRelayRuntime {
    pub(super) fn start(
        request: RelayOwnerRequest,
        key: std::sync::Arc<DeviceStaticKey>,
        local: std::sync::Arc<std::sync::RwLock<LocalAdmission>>,
        session_id: &SessionId,
        epoch: Epoch,
    ) -> Result<Self, CollabRuntimeFailure> {
        // The owner network worker resolves one signed bootstrap snapshot and
        // uses that same region entry for locator verification, relay pinning,
        // and the bridge endpoint. No HTTP runs on the UI/event-loop thread.
        let bootstrap = request.provider.load()?;
        let region = bootstrap.region(request.home_region)?;
        let endpoint = region.relay_endpoint.clone();
        let development_unsigned = development_unsigned_allowed(&endpoint);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let local_addr = listener
            .local_addr()
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let discovery_id = random_relay_discovery_id()?;
        let prelude = std::sync::Arc::new(
            ServerPrelude::new(discovery_id.clone(), session_id.clone(), epoch)
                .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        );
        let route = owner_route(
            *key.public_key(),
            discovery_id,
            epoch,
            development_unsigned,
            request.control_plane.as_ref(),
            region,
            &local,
        )?;
        let authenticator = if development_unsigned {
            None
        } else {
            Some(
                LocalAdmission::challenge_bound_relay_authenticator(
                    std::sync::Arc::clone(&local),
                    std::sync::Arc::clone(&key),
                    Arc::clone(&region.relay_x25519_keys),
                )
                .map_err(|error| error.failure)?,
            )
        };
        // Production surfaces only the 10-char pairing code; the ~500-char
        // `opc1_` fragment is unusable for human sharing and is no longer
        // shown. The development-unsigned loop has no control plane to hold
        // a sealed code, so it keeps the long fragment for local testing.
        let invite = if development_unsigned {
            CollabInviteCode::new(RelayInviteV1::new(&route).to_fragment())
        } else {
            publish_owner_pairing_code(&route, request.control_plane.as_ref(), region, &local)
                .and_then(|code| CollabInviteCode::new(code.expose_str().to_owned()))
        };
        let auth = LocalAdmission::relay_auth_extension(*key.public_key())
            .map_err(|error| error.failure)?;
        let handshake = RelayHandshake::new(route, auth);
        let bridge = crate::blocking::block_on(async move {
            let bridge = if development_unsigned {
                start_development_owner_bridge(endpoint, handshake, local_addr).await?
            } else {
                RelayOwnerBridge::start_default_lanes(
                    endpoint,
                    handshake,
                    local_addr,
                    authenticator.ok_or(CollabRuntimeFailure::RelayUnavailable)?,
                )
                .await
                .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?
            };
            bridge
                .wait_until_ready(OWNER_RELAY_READY_TIMEOUT)
                .await
                .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
            Ok::<_, CollabRuntimeFailure>(bridge)
        })?;
        Ok(Self {
            listener,
            prelude,
            invite,
            path: CollabConnectionPathUi::Relay {
                home_region: ui_region(request.home_region),
            },
            bridge,
        })
    }

    pub(super) fn accept(&self) -> std::io::Result<(TcpStream, SocketAddr)> {
        self.listener.accept()
    }

    pub(super) fn prelude(&self) -> std::sync::Arc<ServerPrelude> {
        std::sync::Arc::clone(&self.prelude)
    }

    pub(super) fn invite(&self) -> Option<CollabInviteCode> {
        self.invite.clone()
    }

    pub(super) const fn path(&self) -> CollabConnectionPathUi {
        self.path
    }

    /// Coarse, credential-free relay-pool state for the owner network worker's
    /// diagnostics. A session that keeps losing peers is almost always a lane
    /// pool that is empty or degraded, and nothing else in the owner path can
    /// see that.
    pub(super) fn bridge_diagnostic(&self) -> OwnerRelayBridgeReport {
        let status = self.bridge.status();
        OwnerRelayBridgeReport {
            phase: status.phase,
            waiting_lanes: status.waiting_lanes,
            active_tunnels: status.active_tunnels,
            last_error: status.last_error,
            relay_pairing_timeouts: status.relay_pairing_timeouts,
        }
    }
}

/// Owner relay-pool snapshot. Every field is a bounded enum or counter — never
/// endpoints, invites, identities, or raw transport errors — because its
/// `Debug`/`Display` output is written to the local terminal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct OwnerRelayBridgeReport {
    pub(super) phase: op_collab_relay_client::RelayBridgePhase,
    pub(super) waiting_lanes: usize,
    pub(super) active_tunnels: usize,
    pub(super) last_error: Option<op_collab_relay_client::RelayFailureKind>,
    /// Lanes the relay retired with its own pairing timeout. Non-zero means
    /// the relay's waiting window is expiring before the client's lane recycle
    /// budget, which leaves the waiting queue short while the pool re-dials.
    pub(super) relay_pairing_timeouts: u32,
}

impl std::fmt::Debug for OwnerRelayRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OwnerRelayRuntime")
            .field("path", &self.path)
            .field("bridge_status", &self.bridge.status())
            .finish()
    }
}

pub(super) struct GuestRelayRuntime {
    local_addr: SocketAddr,
    expected_discovery_id: String,
    expected_remote_static: [u8; 32],
    bridge: RelayGuestBridge,
}

impl std::fmt::Debug for GuestRelayRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GuestRelayRuntime")
            .field("local_addr", &self.local_addr)
            .field("bridge_status", &self.bridge.status())
            .finish()
    }
}

pub(super) fn owner_request(
    control_plane: std::sync::Arc<dyn RelayLocatorControlPlane>,
    preferred_region: RelayRegion,
) -> Result<RelayOwnerRequest, CollabRuntimeError> {
    let provider = bootstrap_provider(preferred_region)?;
    // `var_os` keeps a present-but-non-Unicode override fail-closed instead
    // of letting it read as "unset" and silently re-home the session.
    let env_value = match std::env::var_os(RELAY_HOME_REGION_ENV) {
        Some(value) => Some(
            value
                .into_string()
                .map_err(|_| runtime_error(CollabRuntimeFailure::RelayRegionUnavailable))?,
        ),
        None => None,
    };
    let home_region = resolve_home_region(env_value.as_deref(), preferred_region)?;
    Ok(RelayOwnerRequest {
        home_region,
        provider,
        control_plane,
    })
}

pub(super) fn guest_route_from_invite(
    invite: &str,
    control_plane: Arc<dyn RelayLocatorControlPlane>,
    preferred_region: RelayRegion,
) -> Result<GuestConnectionRoute, CollabRuntimeError> {
    let invite = RelayInviteV1::from_fragment(invite)
        .map_err(|_| runtime_error(CollabRuntimeFailure::RelayInviteInvalid))?;
    let provider = bootstrap_provider(preferred_region)?;
    Ok(guest_route_from_parsed_invite(
        invite,
        provider,
        control_plane,
    ))
}

/// Short pairing codes resolve to the full invite on the guest network
/// worker; only the bounded code shape is parsed on the UI thread.
///
/// `preferred_region` only picks the hub that serves the signed bootstrap
/// document (both hubs publish both regions); the claim itself is routed by
/// the region riding in the code.
pub(super) fn guest_route_from_pairing_code(
    code: &str,
    control_plane: Arc<dyn RelayLocatorControlPlane>,
    preferred_region: RelayRegion,
) -> Result<GuestConnectionRoute, CollabRuntimeError> {
    let code = PairingCode::parse(code)
        .map_err(|_| runtime_error(CollabRuntimeFailure::RelayInviteInvalid))?;
    // The claimable region rides in the code's first character, so a guest
    // needs no home-region configuration and the UI shows the true region.
    let home_region = code
        .region()
        .ok_or_else(|| runtime_error(CollabRuntimeFailure::RelayInviteInvalid))?;
    let provider = bootstrap_provider(preferred_region)?;
    Ok(GuestConnectionRoute::Relay(Box::new(RelayGuestRequest {
        secret: RelayJoinSecret::pairing(code),
        home_region,
        provider,
        control_plane,
    })))
}

fn guest_route_from_parsed_invite(
    invite: RelayInviteV1,
    provider: Arc<dyn RelayBootstrapProvider>,
    control_plane: Arc<dyn RelayLocatorControlPlane>,
) -> GuestConnectionRoute {
    let region = invite.locator().claims().home_region();
    GuestConnectionRoute::Relay(Box::new(RelayGuestRequest {
        secret: RelayJoinSecret::Invite(Box::new(invite)),
        home_region: region,
        provider,
        control_plane,
    }))
}

pub(super) fn relay_guest_target(
    relay: &GuestRelayRuntime,
) -> (Vec<SocketAddr>, Option<String>, Option<[u8; 32]>) {
    (
        vec![relay.local_addr()],
        Some(relay.expected_discovery_id.clone()),
        Some(relay.expected_remote_static),
    )
}

/// The owner's home region: the environment override wins when set (and an
/// unrecognized value stays a hard error rather than silently re-homing the
/// session); otherwise the user's service-region preference applies.
fn resolve_home_region(
    env_value: Option<&str>,
    preferred: RelayRegion,
) -> Result<RelayRegion, CollabRuntimeError> {
    match env_value {
        Some("cn") => Ok(RelayRegion::Cn),
        Some("global") => Ok(RelayRegion::Global),
        Some(_) => Err(runtime_error(CollabRuntimeFailure::RelayRegionUnavailable)),
        None => Ok(preferred),
    }
}

#[cfg(any(test, debug_assertions))]
fn locator_http_client(
    region: &RelayBootstrapRegion,
) -> Result<RelayLocatorHttpClient<Ed25519LocatorVerifier>, CollabRuntimeFailure> {
    if let Ok(client) =
        RelayLocatorHttpClient::new(&region.locator_url, region.locator_verifier.clone())
    {
        return Ok(client);
    }
    RelayLocatorHttpClient::new_loopback_http_for_development(
        &region.locator_url,
        region.locator_verifier.clone(),
        region.development_http,
    )
    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(not(any(test, debug_assertions)))]
fn locator_http_client(
    region: &RelayBootstrapRegion,
) -> Result<RelayLocatorHttpClient<Ed25519LocatorVerifier>, CollabRuntimeFailure> {
    RelayLocatorHttpClient::new(&region.locator_url, region.locator_verifier.clone())
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[derive(Clone, Copy)]
struct AcceptAllDevelopmentLocator;

impl RelayLocatorVerifier for AcceptAllDevelopmentLocator {
    fn verify(
        &self,
        _key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        true
    }
}

fn development_unsigned_allowed(endpoint: &RelayEndpoint) -> bool {
    development_unsigned_opt_in(
        cfg!(any(test, debug_assertions)),
        !endpoint.is_encrypted(),
        development_unsigned_environment_value(),
    )
}

#[cfg(any(test, debug_assertions))]
fn development_unsigned_environment_value() -> Option<String> {
    std::env::var(RELAY_DEV_UNSIGNED_ENV).ok()
}

#[cfg(not(any(test, debug_assertions)))]
fn development_unsigned_environment_value() -> Option<String> {
    None
}

fn development_unsigned_opt_in(
    debug_build: bool,
    loopback_ws: bool,
    value: Option<String>,
) -> bool {
    debug_build && loopback_ws && value.as_deref() == Some("1")
}

fn owner_route(
    owner_static: [u8; 32],
    discovery_id: String,
    epoch: Epoch,
    development_unsigned: bool,
    control_plane: &dyn RelayLocatorControlPlane,
    region: &RelayBootstrapRegion,
    local: &std::sync::RwLock<LocalAdmission>,
) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
    if !development_unsigned {
        return publish_production_route(owner_static, discovery_id, control_plane, region, local);
    }
    let now = unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable)? / 1_000;
    let not_before = now.saturating_sub(1).max(1);
    let expires_at = now
        .checked_add(MAX_PAIRING_LIFETIME_SECS.saturating_sub(60))
        .ok_or(CollabRuntimeFailure::RelayUnavailable)?;
    let key_id = LocatorKeyId::new(DEBUG_LOCATOR_KEY_ID.to_owned())
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let claims = UnsignedRelayLocatorV1::new(
        region.region,
        RouteId::generate().map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        NonZeroU64::new(epoch.0).unwrap_or(NonZeroU64::MIN),
        OwnerNoiseStatic::new(owner_static).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        ExpectedDiscoveryId::new(discovery_id)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        not_before,
        expires_at,
        key_id.clone(),
    )
    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let locator = claims.attach_signature(
        LocatorSignature::new([0xA5; 64]).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
    );
    let verified = locator
        .verify(&AcceptAllDevelopmentLocator, now)
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    Ok(VerifiedRelayRoute::new(
        verified,
        RouteCapability::generate().map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
    ))
}

fn publish_production_route(
    owner_static: [u8; 32],
    discovery_id: String,
    control_plane: &dyn RelayLocatorControlPlane,
    region: &RelayBootstrapRegion,
    local: &std::sync::RwLock<LocalAdmission>,
) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
    let draft = OwnerPublishDraft::generate(
        region.region,
        OwnerNoiseStatic::new(owner_static).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        ExpectedDiscoveryId::new(discovery_id)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        RelayPublishLifetime::new(MAX_PAIRING_LIFETIME_SECS.saturating_sub(60))
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
    )
    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let local = local
        .read()
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    control_plane.publish_route(draft, local.relay_ticket(), region)
}

fn random_relay_discovery_id() -> Result<String, CollabRuntimeFailure> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    use std::fmt::Write as _;
    for byte in bytes {
        write!(encoded, "{byte:02x}").map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    }
    Ok(encoded)
}

#[cfg(any(test, debug_assertions))]
async fn start_development_owner_bridge(
    endpoint: RelayEndpoint,
    handshake: RelayHandshake,
    local_addr: SocketAddr,
) -> Result<RelayOwnerBridge, CollabRuntimeFailure> {
    RelayOwnerBridge::start_unauthenticated_for_development(
        endpoint,
        handshake,
        local_addr,
        DEFAULT_OWNER_LANE_COUNT,
    )
    .await
    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(not(any(test, debug_assertions)))]
async fn start_development_owner_bridge(
    _endpoint: RelayEndpoint,
    _handshake: RelayHandshake,
    _local_addr: SocketAddr,
) -> Result<RelayOwnerBridge, CollabRuntimeFailure> {
    Err(CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(any(test, debug_assertions))]
async fn start_development_guest_bridge(
    endpoint: RelayEndpoint,
    handshake: RelayHandshake,
) -> Result<RelayGuestBridge, CollabRuntimeFailure> {
    RelayGuestBridge::start_unauthenticated_for_development(endpoint, handshake)
        .await
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(not(any(test, debug_assertions)))]
async fn start_development_guest_bridge(
    _endpoint: RelayEndpoint,
    _handshake: RelayHandshake,
) -> Result<RelayGuestBridge, CollabRuntimeFailure> {
    Err(CollabRuntimeFailure::RelayUnavailable)
}

const fn ui_region(region: RelayRegion) -> CollabRelayRegion {
    match region {
        RelayRegion::Cn => CollabRelayRegion::China,
        RelayRegion::Global => CollabRelayRegion::Global,
    }
}

pub(super) const fn protocol_region(region: CollabRelayRegion) -> RelayRegion {
    match region {
        CollabRelayRegion::China => RelayRegion::Cn,
        CollabRelayRegion::Global => RelayRegion::Global,
    }
}

/// Redeem a short pairing code from exactly the region named in its first
/// character. A single-region claim keeps the code id and the guest's
/// bearer ticket away from control planes that were never part of the
/// session.
fn claim_pairing_invite(
    bootstrap: &RelayBootstrap,
    code: &PairingCode,
    control_plane: &dyn RelayLocatorControlPlane,
    key: &DeviceStaticKey,
    local: &std::sync::RwLock<LocalAdmission>,
) -> Result<RelayInviteV1, CollabRuntimeFailure> {
    let code_region = code
        .region()
        .ok_or(CollabRuntimeFailure::RelayInviteInvalid)?;
    let region = bootstrap.region(code_region)?;
    let request = PairingClaimRequest::new(*key.public_key(), code.code_id());
    // Copy the ticket out of the admission lock: the claim is a blocking
    // HTTP round-trip and must not stall ticket renewal for its duration.
    let ticket = {
        let admission = local
            .read()
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        op_auth_bridge::OpaqueCollabTicket::new(admission.relay_ticket().expose().to_vec())
            .map_err(|_| CollabRuntimeFailure::AuthenticationUnavailable)?
    };
    let sealed = control_plane.claim_pairing_code(&request, &ticket, region)?;
    sealed
        .open(code)
        .map_err(|_| CollabRuntimeFailure::RelayInviteInvalid)
}

/// Best-effort short-code publish for the owner. Every failure path returns
/// `None`: the session still starts (and keeps its LAN share address), but
/// the owner panel shows no public code and the runtime raises a relay
/// notice so the gap is visible instead of silent.
fn publish_owner_pairing_code(
    route: &VerifiedRelayRoute,
    control_plane: &dyn RelayLocatorControlPlane,
    region: &RelayBootstrapRegion,
    local: &std::sync::RwLock<LocalAdmission>,
) -> Option<PairingCode> {
    let now = unix_time_ms().ok()? / 1_000;
    let ttl = route
        .locator()
        .claims()
        .expires_at_unix()
        .saturating_sub(now)
        .min(u64::from(MAX_PAIRING_CODE_TTL_SECS));
    let ttl = u32::try_from(ttl).ok().filter(|ttl| *ttl > 0)?;
    let owner_static = *route.locator().claims().owner_noise_static().as_bytes();
    let invite = RelayInviteV1::new(route);
    let ticket = {
        let admission = local.read().ok()?;
        op_auth_bridge::OpaqueCollabTicket::new(admission.relay_ticket().expose().to_vec()).ok()?
    };
    // Two attempts with independently random codes: a duplicate-id refusal
    // or a transient control-plane hiccup should not cost the session its
    // only shareable code.
    for _ in 0..2 {
        let Ok(code) = PairingCode::generate_for(region.region) else {
            return None;
        };
        // Seal the legacy v1 envelope during the v1→v2 transition: fielded
        // v0.8.4 desktops reject any other envelope version when they claim,
        // so a v2-sealed publish mints codes those guests can never open.
        // Opening accepts both versions; move back to `seal_random` (v2)
        // once the fielded readers understand the v2 envelope.
        let Ok(sealed) = SealedPairingInvite::seal_random_legacy_compat(&code, &invite) else {
            return None;
        };
        let Ok(request) = PairingPublishRequest::new(
            owner_static,
            code.code_id(),
            ttl,
            sealed.as_bytes().to_vec(),
        ) else {
            return None;
        };
        if control_plane
            .publish_pairing_code(&request, &ticket, region)
            .is_ok()
        {
            return Some(code);
        }
    }
    None
}

/// Classify a failed invite verification: an authentic-but-lapsed pairing
/// window is the user-fixable case and gets its own message; every other
/// verification failure collapses to "invalid".
fn invite_verify_failure(
    error: op_collab_relay_protocol::RelayProtocolError,
) -> CollabRuntimeFailure {
    use op_collab_relay_protocol::RelayProtocolError as E;
    match error {
        E::Expired | E::NotYetValid => CollabRuntimeFailure::RelayInviteExpired,
        _ => CollabRuntimeFailure::RelayInviteInvalid,
    }
}

const fn runtime_error(failure: CollabRuntimeFailure) -> CollabRuntimeError {
    CollabRuntimeError::new(failure)
}

#[cfg(test)]
#[path = "relay_tests.rs"]
mod tests;
