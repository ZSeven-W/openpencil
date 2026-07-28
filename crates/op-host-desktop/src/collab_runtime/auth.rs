//! Production collaboration authentication adapters.
//!
//! The private bridge only returns an opaque ticket. Identity is derived here
//! from the open verifier and pinned signed-policy authority; account display
//! state and peer payloads never become an authentication fallback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use op_auth_bridge::{
    CollabJwksCacheLimits, CollabJwksError, CollabJwksFetchError, CollabTicketPoll,
    CollabTicketRequest, CollabTicketVerifier, CollabVerifierConfig, CollabVerifyError,
    OpaqueCollabTicket,
};
use op_collab::{OpaqueTicket, VerifiedAuthMetadata};
use op_collab_transport::{
    AdmissionError, AdmissionHello, JoinIntent, TicketVerifier, VerifiedTicketClaims,
};

use crate::collab_jwks::NativeCollabJwksFetcher;

use super::network::TERMINAL_FLUSH_TIMEOUT;
use super::types::{CollabRuntimeError, CollabRuntimeFailure};

const TICKET_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const TICKET_POLL_INTERVAL: Duration = Duration::from_millis(10);
const INITIAL_RENEWAL_RETRY_BACKOFF: Duration = Duration::from_secs(1);
const MAX_RENEWAL_RETRY_BACKOFF: Duration = Duration::from_secs(30);

struct PendingRenewal {
    receiver: Option<Receiver<Result<LocalAdmission, CollabRuntimeError>>>,
    cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for PendingRenewal {
    fn drop(&mut self) {
        // Retire the publication channel before waking the worker so even a
        // result racing with cancellation cannot re-enter the active session.
        self.receiver.take();
        self.cancelled.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(super) struct ProductionTicketVerifier {
    inner: CollabTicketVerifier<NativeCollabJwksFetcher>,
}

impl ProductionTicketVerifier {
    pub(super) fn new() -> Result<Self, CollabRuntimeError> {
        let config = op_auth_bridge::desktop_collab_verifier_config().map_err(|_| {
            CollabRuntimeError::new(CollabRuntimeFailure::AuthenticationUnavailable)
        })?;
        Self::new_with_config(config)
    }

    fn new_with_config(config: CollabVerifierConfig) -> Result<Self, CollabRuntimeError> {
        let fetcher = NativeCollabJwksFetcher::new().map_err(|_| {
            CollabRuntimeError::new(CollabRuntimeFailure::AuthenticationUnavailable)
        })?;
        let inner = CollabTicketVerifier::new(config, fetcher, CollabJwksCacheLimits::default())
            .map_err(|_| {
                CollabRuntimeError::new(CollabRuntimeFailure::AuthenticationUnavailable)
            })?;
        Ok(Self { inner })
    }

    fn expected_issuer(&self) -> &str {
        self.inner.config().issuer()
    }

    fn verify_cancellable(
        &self,
        opaque_ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
        now_unix_ms: u64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<VerifiedTicketClaims, CollabRuntimeError> {
        if cancelled() {
            return Err(authentication_unavailable());
        }
        let verified = self
            .inner
            .verify_at_cancellable(
                opaque_ticket,
                expected_dh_pub_x25519,
                now_unix_ms / 1_000,
                Instant::now(),
                cancelled,
            )
            .map_err(|error| verification_runtime_error(&error, cancelled()))?;
        if cancelled() {
            return Err(authentication_unavailable());
        }
        VerifiedTicketClaims::new_with_profile(
            verified.issuer().to_owned(),
            verified.subject().to_owned(),
            verified.device_id().to_owned(),
            *verified.dh_pub_x25519(),
            verified.expires_at_unix_ms(),
            verified.display_name().map(str::to_owned),
            verified.avatar_url().map(str::to_owned),
        )
        .map_err(|_| ticket_rejected())
    }
}

impl TicketVerifier for ProductionTicketVerifier {
    fn verify(
        &self,
        opaque_ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
        now_unix_ms: u64,
    ) -> Result<VerifiedTicketClaims, AdmissionError> {
        self.verify_cancellable(
            opaque_ticket,
            expected_dh_pub_x25519,
            now_unix_ms,
            &never_cancelled,
        )
        .map_err(|_| AdmissionError::Verification)
    }
}

pub(super) struct LocalAdmission {
    ticket: OpaqueCollabTicket,
    auth: VerifiedAuthMetadata,
}

impl LocalAdmission {
    pub(super) fn request_cancellable(
        local_static: &[u8; 32],
        verifier: &ProductionTicketVerifier,
        cancelled: impl Fn() -> bool,
    ) -> Result<Self, CollabRuntimeError> {
        if cancelled() {
            return Err(authentication_unavailable());
        }
        let provider = op_auth_bridge::collab_ticket_provider();
        if !provider.available() {
            return Err(authentication_unavailable());
        }
        let request =
            CollabTicketRequest::new(*local_static).map_err(|_| authentication_unavailable())?;
        let request_id = provider
            .begin_ticket(request)
            .map_err(|_| authentication_unavailable())?;
        let deadline = Instant::now()
            .checked_add(TICKET_REQUEST_TIMEOUT)
            .ok_or_else(authentication_unavailable)?;
        let ticket = loop {
            if cancelled() {
                provider.cancel_ticket(request_id);
                return Err(authentication_unavailable());
            }
            match provider.poll_ticket(request_id) {
                CollabTicketPoll::Pending if Instant::now() < deadline => {
                    std::thread::sleep(TICKET_POLL_INTERVAL);
                }
                CollabTicketPoll::Ready { ticket, .. } => break ticket,
                CollabTicketPoll::Pending => {
                    provider.cancel_ticket(request_id);
                    return Err(authentication_unavailable());
                }
                CollabTicketPoll::Failed(_) => {
                    return Err(authentication_unavailable());
                }
            }
        };
        if cancelled() {
            return Err(authentication_unavailable());
        }
        let now_unix_ms = unix_time_ms()?;
        let claims =
            verifier.verify_cancellable(ticket.expose(), local_static, now_unix_ms, &cancelled)?;
        if cancelled() {
            return Err(authentication_unavailable());
        }
        if claims.issuer() != verifier.expected_issuer() {
            return Err(ticket_rejected());
        }
        let auth = VerifiedAuthMetadata {
            issuer: claims.issuer().to_owned(),
            subject: claims.subject().to_owned(),
            device_id: claims.device_id().to_owned(),
            proof_binding: URL_SAFE_NO_PAD.encode(claims.dh_pub_x25519()),
            expires_at_unix_ms: claims.expires_at_unix_ms(),
            display_name: claims.display_name().map(str::to_owned),
            avatar_url: claims.avatar_url().map(str::to_owned),
        };
        if cancelled() {
            return Err(authentication_unavailable());
        }
        Ok(Self { ticket, auth })
    }

    pub(super) fn hello(&self, intent: JoinIntent) -> Result<AdmissionHello, CollabRuntimeError> {
        AdmissionHello::from_ticket_bytes(self.ticket.expose(), intent)
            .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::TicketRejected))
    }

    pub(super) fn auth(&self) -> &VerifiedAuthMetadata {
        &self.auth
    }

    pub(super) fn expected_issuer(&self) -> &str {
        &self.auth.issuer
    }

    pub(super) fn expected_subject(&self) -> &str {
        &self.auth.subject
    }

    pub(super) fn renewal_ticket(&self) -> Result<OpaqueTicket, CollabRuntimeError> {
        OpaqueTicket::from_utf8_bytes(self.ticket.expose())
            .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::TicketRejected))
    }
}

/// Non-blocking local ticket renewal. Provider polling runs on a dedicated
/// worker; socket drivers only poll this receiver and keep heartbeats moving.
pub(super) struct LocalTicketRenewer {
    local_static: [u8; 32],
    verifier: Arc<ProductionTicketVerifier>,
    binding: VerifiedAuthMetadata,
    renew_at: Instant,
    shutdown_at: Instant,
    expires_at: Instant,
    retry_backoff: Duration,
    pending: Option<PendingRenewal>,
}

impl LocalTicketRenewer {
    pub(super) fn new(
        local_static: [u8; 32],
        verifier: Arc<ProductionTicketVerifier>,
        binding: VerifiedAuthMetadata,
    ) -> Result<Self, CollabRuntimeError> {
        let now_unix_ms = unix_time_ms()?;
        let now = Instant::now();
        let (renew_at, expires_at) =
            renewal_schedule(now_unix_ms, binding.expires_at_unix_ms, now)?;
        let shutdown_at = terminal_shutdown_at(now, expires_at);
        Ok(Self {
            local_static,
            verifier,
            binding,
            renew_at,
            shutdown_at,
            expires_at,
            retry_backoff: INITIAL_RENEWAL_RETRY_BACKOFF,
            pending: None,
        })
    }

    pub(super) fn poll(
        &mut self,
        now: Instant,
    ) -> Result<Option<LocalAdmission>, CollabRuntimeError> {
        if now >= self.shutdown_at {
            return Err(CollabRuntimeError::new(
                CollabRuntimeFailure::TicketRejected,
            ));
        }
        if self.pending.is_none() && now >= self.renew_at {
            let local_static = self.local_static;
            let verifier = Arc::clone(&self.verifier);
            let (sender, receiver) = mpsc::channel();
            let cancelled = Arc::new(AtomicBool::new(false));
            let worker_cancelled = Arc::clone(&cancelled);
            let worker = match std::thread::Builder::new()
                .name("op-collab-ticket-renewal".to_string())
                .spawn(move || {
                    let _ = sender.send(LocalAdmission::request_cancellable(
                        &local_static,
                        verifier.as_ref(),
                        || worker_cancelled.load(Ordering::Acquire),
                    ));
                }) {
                Ok(worker) => worker,
                Err(_) => {
                    self.schedule_transient_retry(now)?;
                    return Ok(None);
                }
            };
            self.pending = Some(PendingRenewal {
                receiver: Some(receiver),
                cancelled,
                worker: Some(worker),
            });
        }
        let Some(pending) = self.pending.as_ref() else {
            return Ok(None);
        };
        let admission = match pending
            .receiver
            .as_ref()
            .expect("pending renewal receiver must exist")
            .try_recv()
        {
            Ok(Ok(admission)) => admission,
            Ok(Err(error)) if error.failure == CollabRuntimeFailure::AuthenticationUnavailable => {
                self.pending = None;
                self.schedule_transient_retry(now)?;
                return Ok(None);
            }
            Ok(Err(error)) => {
                self.pending = None;
                return Err(error);
            }
            Err(TryRecvError::Empty) => return Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                self.schedule_transient_retry(now)?;
                return Ok(None);
            }
        };
        self.pending = None;
        validate_renewed_binding(&self.binding, admission.auth())?;
        let now_unix_ms = unix_time_ms()?;
        (self.renew_at, self.expires_at) =
            renewal_schedule(now_unix_ms, admission.auth().expires_at_unix_ms, now)?;
        self.shutdown_at = terminal_shutdown_at(now, self.expires_at);
        self.retry_backoff = INITIAL_RENEWAL_RETRY_BACKOFF;
        self.binding = admission.auth().clone();
        Ok(Some(admission))
    }

    fn schedule_transient_retry(&mut self, now: Instant) -> Result<(), CollabRuntimeError> {
        let (retry_at, next_backoff) =
            transient_retry_schedule(now, self.expires_at, self.retry_backoff)?;
        self.renew_at = retry_at;
        self.retry_backoff = next_backoff;
        Ok(())
    }
}

pub(super) fn production_verifier() -> Result<Arc<ProductionTicketVerifier>, CollabRuntimeError> {
    Ok(Arc::new(ProductionTicketVerifier::new()?))
}

fn never_cancelled() -> bool {
    false
}

fn authentication_unavailable() -> CollabRuntimeError {
    CollabRuntimeError::new(CollabRuntimeFailure::AuthenticationUnavailable)
}

fn ticket_rejected() -> CollabRuntimeError {
    CollabRuntimeError::new(CollabRuntimeFailure::TicketRejected)
}

fn verification_runtime_error(
    error: &CollabVerifyError,
    cancellation_requested: bool,
) -> CollabRuntimeError {
    let failure = if cancellation_requested {
        CollabRuntimeFailure::AuthenticationUnavailable
    } else {
        verification_failure(error)
    };
    CollabRuntimeError::new(failure)
}

fn verification_failure(error: &CollabVerifyError) -> CollabRuntimeFailure {
    match error {
        CollabVerifyError::Cancelled => CollabRuntimeFailure::AuthenticationUnavailable,
        CollabVerifyError::Jwks(error) => jwks_verification_failure(error),
        CollabVerifyError::InvalidTicketSize { .. }
        | CollabVerifyError::MalformedCompactJws
        | CollabVerifyError::SegmentTooLarge { .. }
        | CollabVerifyError::InvalidBase64 { .. }
        | CollabVerifyError::MalformedJson { .. }
        | CollabVerifyError::WrongAlgorithm
        | CollabVerifyError::WrongType
        | CollabVerifyError::InvalidKeyId
        | CollabVerifyError::InvalidSignature
        | CollabVerifyError::InvalidIssuer
        | CollabVerifyError::InvalidAudience
        | CollabVerifyError::InvalidVersion
        | CollabVerifyError::InvalidScope
        | CollabVerifyError::InvalidSubject
        | CollabVerifyError::InvalidDeviceId
        | CollabVerifyError::InvalidChannelBinding
        | CollabVerifyError::ChannelBindingMismatch
        | CollabVerifyError::InvalidTicketId
        | CollabVerifyError::InvalidDisplayName
        | CollabVerifyError::InvalidAvatarUrl
        | CollabVerifyError::InvalidTimestamps
        | CollabVerifyError::NotYetValid
        | CollabVerifyError::Expired
        | CollabVerifyError::LifetimeTooLong
        | CollabVerifyError::ExpiryOverflow => CollabRuntimeFailure::TicketRejected,
    }
}

fn jwks_verification_failure(error: &CollabJwksError) -> CollabRuntimeFailure {
    // A bad authority response cannot validate the renewal, but it also does
    // not invalidate the already-verified ticket. Unknown kid is different.
    match error {
        CollabJwksError::UnknownKey => CollabRuntimeFailure::TicketRejected,
        CollabJwksError::InvalidBodySize { .. }
        | CollabJwksError::MalformedJson
        | CollabJwksError::EmptyKeyset
        | CollabJwksError::TooManyKeys { .. }
        | CollabJwksError::DuplicateKeyId
        | CollabJwksError::InvalidKey { .. }
        | CollabJwksError::InvalidEtag { .. }
        | CollabJwksError::RefreshThrottled
        | CollabJwksError::CacheUnavailable
        | CollabJwksError::NotModifiedWithoutCache
        | CollabJwksError::Policy(_) => CollabRuntimeFailure::AuthenticationUnavailable,
        CollabJwksError::Fetch(error) => jwks_fetch_failure(error),
    }
}

fn jwks_fetch_failure(error: &CollabJwksFetchError) -> CollabRuntimeFailure {
    match error {
        CollabJwksFetchError::Cancelled
        | CollabJwksFetchError::Unavailable
        | CollabJwksFetchError::RejectedResponse
        | CollabJwksFetchError::ResponseTooLarge => CollabRuntimeFailure::AuthenticationUnavailable,
    }
}

pub(super) fn unix_time_ms() -> Result<u64, CollabRuntimeError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::ClockUnavailable))?
        .as_millis();
    u64::try_from(millis)
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::ClockUnavailable))
}

fn renewal_schedule(
    now_unix_ms: u64,
    expires_at_unix_ms: u64,
    now: Instant,
) -> Result<(Instant, Instant), CollabRuntimeError> {
    let ttl = expires_at_unix_ms
        .checked_sub(now_unix_ms)
        .filter(|ttl| *ttl > 0)
        .ok_or_else(|| CollabRuntimeError::new(CollabRuntimeFailure::TicketRejected))?;
    let renewal_ms = ttl.saturating_mul(4) / 5;
    let renew_at = now
        .checked_add(Duration::from_millis(renewal_ms))
        .ok_or_else(|| CollabRuntimeError::new(CollabRuntimeFailure::ClockUnavailable))?;
    let expires_at = now
        .checked_add(Duration::from_millis(ttl))
        .ok_or_else(|| CollabRuntimeError::new(CollabRuntimeFailure::ClockUnavailable))?;
    Ok((renew_at, expires_at))
}

fn terminal_shutdown_at(now: Instant, expires_at: Instant) -> Instant {
    expires_at
        .checked_sub(TERMINAL_FLUSH_TIMEOUT)
        .filter(|shutdown_at| *shutdown_at >= now)
        .unwrap_or(now)
}

fn transient_retry_schedule(
    now: Instant,
    expires_at: Instant,
    backoff: Duration,
) -> Result<(Instant, Duration), CollabRuntimeError> {
    if now >= expires_at {
        return Err(CollabRuntimeError::new(
            CollabRuntimeFailure::TicketRejected,
        ));
    }
    let remaining = expires_at.saturating_duration_since(now);
    let delay = backoff.min(MAX_RENEWAL_RETRY_BACKOFF).min(remaining);
    let retry_at = now
        .checked_add(delay)
        .ok_or_else(|| CollabRuntimeError::new(CollabRuntimeFailure::ClockUnavailable))?;
    let next_backoff = backoff
        .checked_mul(2)
        .unwrap_or(MAX_RENEWAL_RETRY_BACKOFF)
        .min(MAX_RENEWAL_RETRY_BACKOFF);
    Ok((retry_at, next_backoff))
}

fn validate_renewed_binding(
    current: &VerifiedAuthMetadata,
    renewed: &VerifiedAuthMetadata,
) -> Result<(), CollabRuntimeError> {
    if current.issuer != renewed.issuer
        || current.subject != renewed.subject
        || current.device_id != renewed.device_id
        || current.proof_binding != renewed.proof_binding
        || renewed.expires_at_unix_ms <= current.expires_at_unix_ms
    {
        return Err(CollabRuntimeError::new(
            CollabRuntimeFailure::TicketRejected,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn auth(subject: &str, expires_at_unix_ms: u64) -> VerifiedAuthMetadata {
        VerifiedAuthMetadata {
            issuer: op_auth_bridge::PRODUCTION_COLLAB_ISSUER.to_string(),
            subject: subject.to_string(),
            device_id: "device".to_string(),
            proof_binding: "binding".to_string(),
            expires_at_unix_ms,
            display_name: None,
            avatar_url: None,
        }
    }

    #[test]
    fn short_ticket_renews_at_eighty_percent() {
        let start = Instant::now();
        let (due, _) = renewal_schedule(1_000, 1_100, start).expect("renewal deadline");
        assert_eq!(due.duration_since(start), Duration::from_millis(80));
    }

    #[test]
    fn local_auth_shutdown_reserves_the_terminal_flush_window() {
        let start = Instant::now();
        let expires_at = start + Duration::from_secs(30);
        assert_eq!(
            expires_at.duration_since(terminal_shutdown_at(start, expires_at)),
            TERMINAL_FLUSH_TIMEOUT
        );
        assert_eq!(
            terminal_shutdown_at(start, start + Duration::from_millis(100)),
            start
        );
    }

    #[test]
    fn verifier_uses_the_pinned_self_hosted_desktop_trust_root() {
        let config =
            CollabVerifierConfig::for_sso_origin("https://auth.self-hosted.example:9443").unwrap();
        let verifier = ProductionTicketVerifier::new_with_config(config).unwrap();
        assert_eq!(
            verifier.expected_issuer(),
            "https://auth.self-hosted.example:9443"
        );
        assert_eq!(
            verifier.inner.config().jwks_endpoint(),
            "https://auth.self-hosted.example:9443/api/v1/collab/jwks"
        );
    }

    #[test]
    fn verifier_configuration_rejects_an_insecure_self_hosted_origin() {
        let error = CollabVerifierConfig::for_sso_origin("http://auth.self-hosted.example")
            .expect_err("HTTP trust roots must fail closed");
        assert_eq!(
            error,
            op_auth_bridge::CollabVerifierConfigError::InvalidIssuer
        );
    }

    #[test]
    fn renewal_verification_error_classification_is_fail_closed() {
        let transient = [
            CollabVerifyError::Cancelled,
            CollabVerifyError::Jwks(CollabJwksError::InvalidBodySize { maximum: 1 }),
            CollabVerifyError::Jwks(CollabJwksError::MalformedJson),
            CollabVerifyError::Jwks(CollabJwksError::EmptyKeyset),
            CollabVerifyError::Jwks(CollabJwksError::TooManyKeys { maximum: 1 }),
            CollabVerifyError::Jwks(CollabJwksError::DuplicateKeyId),
            CollabVerifyError::Jwks(CollabJwksError::InvalidKey {
                index: 0,
                kind: op_auth_bridge::CollabJwkErrorKind::WrongAlgorithm,
            }),
            CollabVerifyError::Jwks(CollabJwksError::InvalidEtag { maximum: 1 }),
            CollabVerifyError::Jwks(CollabJwksError::RefreshThrottled),
            CollabVerifyError::Jwks(CollabJwksError::CacheUnavailable),
            CollabVerifyError::Jwks(CollabJwksError::NotModifiedWithoutCache),
            CollabVerifyError::Jwks(CollabJwksError::Policy(
                op_auth_bridge::CollabUnionPolicyError::InvalidSignature,
            )),
            CollabVerifyError::Jwks(CollabJwksError::Fetch(CollabJwksFetchError::Cancelled)),
            CollabVerifyError::Jwks(CollabJwksError::Fetch(CollabJwksFetchError::Unavailable)),
            CollabVerifyError::Jwks(CollabJwksError::Fetch(
                CollabJwksFetchError::RejectedResponse,
            )),
            CollabVerifyError::Jwks(CollabJwksError::Fetch(
                CollabJwksFetchError::ResponseTooLarge,
            )),
        ];
        for error in transient {
            assert_eq!(
                verification_failure(&error),
                CollabRuntimeFailure::AuthenticationUnavailable,
                "{error:?}"
            );
        }

        let rejected = [
            CollabVerifyError::MalformedCompactJws,
            CollabVerifyError::MalformedJson { part: "claims" },
            CollabVerifyError::InvalidSignature,
            CollabVerifyError::InvalidChannelBinding,
            CollabVerifyError::ChannelBindingMismatch,
            CollabVerifyError::Jwks(CollabJwksError::UnknownKey),
        ];
        for error in rejected {
            assert_eq!(
                verification_failure(&error),
                CollabRuntimeFailure::TicketRejected,
                "{error:?}"
            );
        }

        assert_eq!(
            verification_runtime_error(&CollabVerifyError::InvalidSignature, true).failure,
            CollabRuntimeFailure::AuthenticationUnavailable,
            "session retirement cancellation must suppress publication"
        );
    }

    #[test]
    fn renewal_must_extend_the_same_noise_bound_principal() {
        let current = auth("account-a", 1_100);
        assert!(validate_renewed_binding(&current, &auth("account-a", 1_200)).is_ok());
        assert!(validate_renewed_binding(&current, &auth("account-b", 1_200)).is_err());
        assert!(validate_renewed_binding(&current, &auth("account-a", 1_100)).is_err());
    }

    #[test]
    fn unavailable_jwks_renewal_backs_off_while_the_old_ticket_remains_valid() {
        let start = Instant::now();
        let expires_at = start + Duration::from_secs(180);
        let shutdown_at = terminal_shutdown_at(start, expires_at);
        let binding = auth("account-a", 180_000);
        let (sender, receiver) = mpsc::channel();
        let unavailable =
            CollabVerifyError::Jwks(CollabJwksError::Fetch(CollabJwksFetchError::Unavailable));
        sender
            .send(Err(verification_runtime_error(&unavailable, false)))
            .unwrap();
        drop(sender);
        let config =
            CollabVerifierConfig::for_sso_origin("https://auth.self-hosted.example").unwrap();
        let mut renewer = LocalTicketRenewer {
            local_static: [0x42; 32],
            verifier: Arc::new(ProductionTicketVerifier::new_with_config(config).unwrap()),
            binding: binding.clone(),
            renew_at: start,
            shutdown_at,
            expires_at,
            retry_backoff: INITIAL_RENEWAL_RETRY_BACKOFF,
            pending: Some(PendingRenewal {
                receiver: Some(receiver),
                cancelled: Arc::new(AtomicBool::new(false)),
                worker: None,
            }),
        };

        assert!(matches!(renewer.poll(start), Ok(None)));
        assert!(renewer.pending.is_none());
        assert_eq!(
            renewer.renew_at.duration_since(start),
            INITIAL_RENEWAL_RETRY_BACKOFF
        );
        assert_eq!(renewer.retry_backoff, Duration::from_secs(2));
        assert_eq!(renewer.expires_at, expires_at);
        assert_eq!(renewer.shutdown_at, shutdown_at);
        assert_eq!(renewer.binding, binding);
    }

    #[test]
    fn transient_renewal_failures_back_off_then_allow_success_before_expiry() {
        let start = Instant::now();
        let expires_at = start + Duration::from_secs(180);
        let (first_retry, second_backoff) =
            transient_retry_schedule(start, expires_at, INITIAL_RENEWAL_RETRY_BACKOFF).unwrap();
        assert_eq!(
            first_retry.duration_since(start),
            INITIAL_RENEWAL_RETRY_BACKOFF
        );
        let (second_retry, third_backoff) =
            transient_retry_schedule(first_retry, expires_at, second_backoff).unwrap();
        assert_eq!(
            second_retry.duration_since(first_retry),
            Duration::from_secs(2)
        );
        assert_eq!(third_backoff, Duration::from_secs(4));

        let (renew_at, renewed_expiry) = renewal_schedule(10_000, 910_000, second_retry).unwrap();
        assert!(renew_at < renewed_expiry);
        assert!(second_retry < renew_at);
    }

    #[test]
    fn persistent_transient_renewal_failure_becomes_fatal_at_old_expiry() {
        let start = Instant::now();
        let expires_at = start + Duration::from_secs(3);
        let (retry_at, _) =
            transient_retry_schedule(start, expires_at, Duration::from_secs(30)).unwrap();
        assert_eq!(retry_at, expires_at);
        let error = transient_retry_schedule(retry_at, expires_at, Duration::from_secs(30))
            .expect_err("old ticket expiry is fail closed");
        assert_eq!(error.failure, CollabRuntimeFailure::TicketRejected);
    }

    #[test]
    fn pending_fetch_is_cancelled_joined_and_does_not_block_restart() {
        let live = Arc::new(AtomicUsize::new(0));
        let worker_live = Arc::clone(&live);
        let published = Arc::new(AtomicUsize::new(0));
        let worker_published = Arc::clone(&published);
        let dropped = Arc::new(AtomicBool::new(false));
        let worker_dropped = Arc::clone(&dropped);
        let (sender, receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (ready, started) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            worker_live.fetch_add(1, Ordering::SeqCst);
            let _result = op_host_services::chat_runtime::block_on_anywhere(
                crate::collab_jwks::await_or_cancel(
                    async move {
                        struct DropMarker(Arc<AtomicBool>);

                        impl Drop for DropMarker {
                            fn drop(&mut self) {
                                self.0.store(true, Ordering::Release);
                            }
                        }

                        let _drop_marker = DropMarker(worker_dropped);
                        ready.send(()).unwrap();
                        std::future::pending::<()>().await;
                    },
                    &|| worker_cancelled.load(Ordering::Acquire),
                ),
            );
            if sender.send(Err(authentication_unavailable())).is_ok() {
                worker_published.fetch_add(1, Ordering::SeqCst);
            }
            worker_live.fetch_sub(1, Ordering::SeqCst);
        });
        started.recv().unwrap();
        assert_eq!(live.load(Ordering::SeqCst), 1);

        let start = Instant::now();
        drop(PendingRenewal {
            receiver: Some(receiver),
            cancelled,
            worker: Some(worker),
        });
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(live.load(Ordering::SeqCst), 0);
        assert_eq!(published.load(Ordering::SeqCst), 0);
        assert!(dropped.load(Ordering::Acquire));

        let restart_started = Instant::now();
        let replacement = std::thread::spawn(|| {
            op_host_services::chat_runtime::block_on_anywhere(crate::collab_jwks::await_or_cancel(
                async { 42 },
                &never_cancelled,
            ))
        });
        assert_eq!(replacement.join().unwrap(), Ok(42));
        assert!(restart_started.elapsed() < Duration::from_secs(1));
    }
}
