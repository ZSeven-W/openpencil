use anyhow::{Context, Result};
use op_auth_bridge::{
    CollabJwksCacheLimits, CollabTicketVerifier, OpaqueCollabTicket, StaticTestJwksFetcher,
    TestCollabIssuer, TestCollabTicketSpec, VerifiedCollabClaims, TEST_COLLAB_ISSUER, TEST_SUBJECT,
};
use op_collab::VerifiedAuthMetadata;
use op_collab_transport::{
    verify_initial_ticket, AdmissionError, DeviceStaticKey, TicketVerifier, VerifiedTicketClaims,
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const OWNER_DEVICE_ID: &str = "123e4567-e89b-12d3-a456-426614174010";
pub const GUEST_DEVICE_ID: &str = "123e4567-e89b-12d3-a456-426614174011";
pub const OWNER_DISPLAY_NAME: &str = "Smoke Owner";
pub const GUEST_DISPLAY_NAME: &str = "Smoke Guest";
pub const OWNER_AVATAR_URL: &str = "https://cdn.test.invalid/owner.png";
pub const GUEST_AVATAR_URL: &str = "https://cdn.test.invalid/guest.png";

pub struct SmokeAuth {
    verifier: SmokeTicketVerifier,
    ticket: OpaqueCollabTicket,
    now_unix_ms: u64,
    local_auth: VerifiedAuthMetadata,
}

impl SmokeAuth {
    pub fn for_device(
        key: &DeviceStaticKey,
        device_id: &str,
        ticket_id: &str,
        display_name: &str,
        avatar_url: &str,
    ) -> Result<Self> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system time is before Unix epoch")?;
        let now_seconds = now.as_secs();
        let now_unix_ms = u64::try_from(now.as_millis()).context("wall clock exceeds u64")?;
        let issuer = TestCollabIssuer::initial();
        let mut spec = TestCollabTicketSpec::valid_at(now_seconds, *key.public_key());
        spec.device_id = device_id.to_owned();
        spec.ticket_id = ticket_id.to_owned();
        spec.display_name = Some(display_name.to_owned());
        spec.avatar_url = Some(avatar_url.to_owned());
        let ticket = issuer.issue(&spec)?;
        let fetcher = StaticTestJwksFetcher::new(issuer.jwks_json()?, 60);
        let verifier = SmokeTicketVerifier(CollabTicketVerifier::new(
            TestCollabIssuer::verifier_config()?,
            fetcher,
            CollabJwksCacheLimits::default(),
        )?);
        let local_auth = verify_initial_ticket(
            &verifier,
            ticket.expose(),
            key.public_key(),
            TEST_COLLAB_ISSUER,
            TEST_SUBJECT,
            now_unix_ms,
        )?
        .to_auth_metadata();
        Ok(Self {
            verifier,
            ticket,
            now_unix_ms,
            local_auth,
        })
    }

    pub fn verifier(&self) -> &dyn TicketVerifier {
        &self.verifier
    }

    pub fn ticket(&self) -> &[u8] {
        self.ticket.expose()
    }

    pub const fn now_unix_ms(&self) -> u64 {
        self.now_unix_ms
    }

    pub fn local_auth(&self) -> &VerifiedAuthMetadata {
        &self.local_auth
    }
}

struct SmokeTicketVerifier(CollabTicketVerifier<StaticTestJwksFetcher>);

impl TicketVerifier for SmokeTicketVerifier {
    fn verify(
        &self,
        opaque_ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
        now_unix_ms: u64,
    ) -> Result<VerifiedTicketClaims, AdmissionError> {
        let verified = self
            .0
            .verify_at(
                opaque_ticket,
                expected_dh_pub_x25519,
                now_unix_ms / 1_000,
                Instant::now(),
            )
            .map_err(|_| AdmissionError::Verification)?;
        transport_claims(verified).map_err(|_| AdmissionError::Verification)
    }
}

fn transport_claims(
    verified: VerifiedCollabClaims,
) -> Result<VerifiedTicketClaims, AdmissionError> {
    VerifiedTicketClaims::new_with_profile(
        verified.issuer().to_owned(),
        verified.subject().to_owned(),
        verified.device_id().to_owned(),
        *verified.dh_pub_x25519(),
        verified.expires_at_unix_ms(),
        verified.display_name().map(str::to_owned),
        verified.avatar_url().map(str::to_owned),
    )
}

pub const fn expected_issuer() -> &'static str {
    TEST_COLLAB_ISSUER
}

pub const fn expected_subject() -> &'static str {
    TEST_SUBJECT
}
