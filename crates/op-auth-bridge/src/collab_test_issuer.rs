//! Deterministic signing fixtures for open integration tests.
//!
//! The seeds and issuer in this module are public test material. Production
//! verifier configuration pins a different issuer and JWKS endpoint.

use std::fmt;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signer as _, SigningKey};

use crate::{
    collab_claims::{CollabJwsHeader, UnverifiedCollabClaims},
    CollabJwksFetchError, CollabJwksFetchRequest, CollabJwksFetchResponse, CollabJwksFetcher,
    CollabTicketError, CollabVerifierConfig, CollabVerifierConfigError, OpaqueCollabTicket,
    COLLAB_JWS_ALGORITHM, COLLAB_JWS_TYPE, COLLAB_TICKET_AUDIENCE, COLLAB_TICKET_SCOPE,
    COLLAB_TICKET_VERSION,
};

pub const TEST_COLLAB_ISSUER: &str = "https://collab.test.invalid";
pub const TEST_COLLAB_JWKS_ENDPOINT: &str = "https://collab.test.invalid/jwks";
pub const TEST_SUBJECT: &str = "123e4567-e89b-12d3-a456-426614174000";
pub const TEST_DEVICE_ID: &str = "123e4567-e89b-12d3-a456-426614174001";
pub const TEST_TICKET_ID: &str = "dGVzdC10aWNrZXQtaWQtMDAwMQ";
pub const TEST_DISPLAY_NAME: &str = "Test Collaborator";
pub const TEST_AVATAR_URL: &str = "https://cdn.test.invalid/avatar.png";

const KEY_A_SEED: [u8; 32] = [0x11; 32];
const KEY_B_SEED: [u8; 32] = [0x22; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestCollabSigningKey {
    A,
    B,
}

impl TestCollabSigningKey {
    pub const fn key_id(self) -> &'static str {
        match self {
            Self::A => "test_key_A",
            Self::B => "test_key_B",
        }
    }

    fn signing_key(self) -> SigningKey {
        SigningKey::from_bytes(match self {
            Self::A => &KEY_A_SEED,
            Self::B => &KEY_B_SEED,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TestCollabTicketSpec {
    pub subject: String,
    pub device_id: String,
    pub dh_pub_x25519: [u8; 32],
    pub issued_at_unix_seconds: u64,
    pub not_before_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub ticket_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl TestCollabTicketSpec {
    pub fn valid_at(now_unix_seconds: u64, dh_pub_x25519: [u8; 32]) -> Self {
        Self {
            subject: TEST_SUBJECT.to_owned(),
            device_id: TEST_DEVICE_ID.to_owned(),
            dh_pub_x25519,
            issued_at_unix_seconds: now_unix_seconds,
            not_before_unix_seconds: now_unix_seconds,
            expires_at_unix_seconds: now_unix_seconds.saturating_add(15 * 60),
            ticket_id: TEST_TICKET_ID.to_owned(),
            display_name: Some(TEST_DISPLAY_NAME.to_owned()),
            avatar_url: Some(TEST_AVATAR_URL.to_owned()),
        }
    }
}

impl fmt::Debug for TestCollabTicketSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TestCollabTicketSpec")
            .field("subject", &"[REDACTED]")
            .field("device_id", &"[REDACTED]")
            .field("dh_pub_x25519", &"[REDACTED]")
            .field("issued_at_unix_seconds", &self.issued_at_unix_seconds)
            .field("not_before_unix_seconds", &self.not_before_unix_seconds)
            .field("expires_at_unix_seconds", &self.expires_at_unix_seconds)
            .field("ticket_id", &"[REDACTED]")
            .field(
                "display_name",
                &self.display_name.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "avatar_url",
                &self.avatar_url.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct TestCollabIssuer {
    active: TestCollabSigningKey,
    published: Vec<TestCollabSigningKey>,
}

impl TestCollabIssuer {
    pub fn initial() -> Self {
        Self {
            active: TestCollabSigningKey::A,
            published: vec![TestCollabSigningKey::A],
        }
    }

    /// Rotation overlap: key B signs while both A and B remain verifiable.
    pub fn rotated() -> Self {
        Self {
            active: TestCollabSigningKey::B,
            published: vec![TestCollabSigningKey::A, TestCollabSigningKey::B],
        }
    }

    /// Post-overlap fixture after every key-A ticket has expired.
    pub fn retired_a() -> Self {
        Self {
            active: TestCollabSigningKey::B,
            published: vec![TestCollabSigningKey::B],
        }
    }

    pub fn verifier_config() -> Result<CollabVerifierConfig, CollabVerifierConfigError> {
        CollabVerifierConfig::new_pinned(TEST_COLLAB_ISSUER, TEST_COLLAB_JWKS_ENDPOINT)
    }

    pub fn issue(
        &self,
        spec: &TestCollabTicketSpec,
    ) -> Result<OpaqueCollabTicket, TestCollabIssuerError> {
        let key = self.active.signing_key();
        let header = CollabJwsHeader {
            alg: COLLAB_JWS_ALGORITHM.to_owned(),
            typ: COLLAB_JWS_TYPE.to_owned(),
            kid: self.active.key_id().to_owned(),
        };
        let claims = UnverifiedCollabClaims {
            iss: TEST_COLLAB_ISSUER.to_owned(),
            aud: COLLAB_TICKET_AUDIENCE.to_owned(),
            ver: COLLAB_TICKET_VERSION,
            sub: spec.subject.clone(),
            device_id: spec.device_id.clone(),
            dh_pub_x25519: URL_SAFE_NO_PAD.encode(spec.dh_pub_x25519),
            scope: COLLAB_TICKET_SCOPE.to_owned(),
            iat: spec.issued_at_unix_seconds,
            nbf: spec.not_before_unix_seconds,
            exp: spec.expires_at_unix_seconds,
            jti: spec.ticket_id.clone(),
            display_name: spec.display_name.clone(),
            avatar_url: spec.avatar_url.clone(),
        };
        let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
        let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
        let signing_input = format!("{header}.{claims}");
        let signature = key.sign(signing_input.as_bytes()).to_bytes();
        let compact = format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(signature));
        OpaqueCollabTicket::new(compact.into_bytes()).map_err(TestCollabIssuerError::Ticket)
    }

    pub fn jwks_json(&self) -> Result<Vec<u8>, TestCollabIssuerError> {
        let keys = self
            .published
            .iter()
            .map(|key_id| {
                let public_key = key_id.signing_key().verifying_key().to_bytes();
                serde_json::json!({
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "alg": "Ed25519",
                    "use": "sig",
                    "key_ops": ["verify"],
                    "kid": key_id.key_id(),
                    "x": URL_SAFE_NO_PAD.encode(public_key),
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_vec(&serde_json::json!({ "keys": keys }))
            .map_err(TestCollabIssuerError::Serialize)
    }
}

#[derive(Clone, Debug)]
pub struct StaticTestJwksFetcher {
    body: Vec<u8>,
    max_age_seconds: u64,
}

impl StaticTestJwksFetcher {
    pub fn new(body: Vec<u8>, max_age_seconds: u64) -> Self {
        Self {
            body,
            max_age_seconds,
        }
    }
}

impl CollabJwksFetcher for StaticTestJwksFetcher {
    fn fetch(
        &self,
        request: CollabJwksFetchRequest<'_>,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        if self.body.len() > request.maximum_body_bytes {
            return Err(CollabJwksFetchError::ResponseTooLarge);
        }
        Ok(CollabJwksFetchResponse::Modified {
            body: self.body.clone(),
            etag: Some("\"static-test-keyset\"".to_owned()),
            max_age_seconds: self.max_age_seconds,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TestCollabIssuerError {
    #[error("test collaboration fixture serialization failed")]
    Serialize(#[from] serde_json::Error),
    #[error("test collaboration fixture produced an invalid ticket")]
    Ticket(#[source] CollabTicketError),
}
