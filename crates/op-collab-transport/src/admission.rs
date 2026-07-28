use std::fmt;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use op_collab::{
    ParticipantId, PeerId, PeerNamespace, Role, VerifiedAuthMetadata,
    MAX_COLLAB_PROFILE_AVATAR_URL_BYTES, MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES,
    MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS, MAX_IDENTIFIER_BYTES,
};
use zeroize::Zeroizing;

use crate::{AdmissionError, MAX_CONTROL_TRANSFER_BYTES, MAX_TICKET_BYTES};

const ADMISSION_MAGIC: &[u8; 4] = b"OPAD";
const ADMISSION_VERSION: u16 = 1;
const ADMISSION_FIXED_BYTES: usize = 12;
const MAX_TICKET_ISSUER_BYTES: usize = 256;

/// Signature- and claims-verified ticket output.
///
/// Implementations must pin issuer, audience, algorithm, version, and scope;
/// only already-verified values cross this trait.
#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedTicketClaims {
    issuer: String,
    subject: String,
    device_id: String,
    dh_pub_x25519: [u8; 32],
    expires_at_unix_ms: u64,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

impl VerifiedTicketClaims {
    pub fn new(
        issuer: String,
        subject: String,
        device_id: String,
        dh_pub_x25519: [u8; 32],
        expires_at_unix_ms: u64,
    ) -> Result<Self, AdmissionError> {
        Self::new_with_profile(
            issuer,
            subject,
            device_id,
            dh_pub_x25519,
            expires_at_unix_ms,
            None,
            None,
        )
    }

    /// Construct verified transport claims with optional signed profile data.
    ///
    /// Callers may only pass values obtained from the same verified ticket.
    /// Missing values are supported for issuer rollout compatibility.
    pub fn new_with_profile(
        issuer: String,
        subject: String,
        device_id: String,
        dh_pub_x25519: [u8; 32],
        expires_at_unix_ms: u64,
        display_name: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<Self, AdmissionError> {
        if !valid_https_origin(&issuer) {
            return Err(AdmissionError::InvalidIdentifier { field: "issuer" });
        }
        if !valid_canonical_uuid(&subject) {
            return Err(AdmissionError::InvalidIdentifier { field: "subject" });
        }
        if !valid_canonical_uuid(&device_id) {
            return Err(AdmissionError::InvalidIdentifier { field: "device_id" });
        }
        if dh_pub_x25519.iter().all(|byte| *byte == 0) {
            return Err(AdmissionError::StaticKeyMismatch);
        }
        if display_name
            .as_deref()
            .is_some_and(|value| !valid_profile_display_name(value))
        {
            return Err(AdmissionError::InvalidIdentifier {
                field: "display_name",
            });
        }
        if avatar_url
            .as_deref()
            .is_some_and(|value| !valid_profile_avatar_url(value))
        {
            return Err(AdmissionError::InvalidIdentifier {
                field: "avatar_url",
            });
        }
        Ok(Self {
            issuer,
            subject,
            device_id,
            dh_pub_x25519,
            expires_at_unix_ms,
            display_name,
            avatar_url,
        })
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn dh_pub_x25519(&self) -> &[u8; 32] {
        &self.dh_pub_x25519
    }

    pub const fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at_unix_ms
    }

    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    pub fn avatar_url(&self) -> Option<&str> {
        self.avatar_url.as_deref()
    }
}

impl fmt::Debug for VerifiedTicketClaims {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedTicketClaims")
            .field("issuer", &self.issuer)
            .field("subject", &"[REDACTED]")
            .field("device_id", &"[REDACTED]")
            .field("dh_pub_x25519", &"[REDACTED]")
            .field("expires_at_unix_ms", &self.expires_at_unix_ms)
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

pub trait TicketVerifier: Send + Sync {
    /// Verifies signature, channel binding, and all issuer-controlled claims.
    ///
    /// The expected Noise static is supplied per call so an adapter does not
    /// need mutable connection-specific state. Errors deliberately collapse
    /// to a log-safe admission verdict at the admission boundary.
    fn verify(
        &self,
        opaque_ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
        now_unix_ms: u64,
    ) -> Result<VerifiedTicketClaims, AdmissionError>;
}

impl<F> TicketVerifier for F
where
    F: Fn(&[u8], &[u8; 32], u64) -> Result<VerifiedTicketClaims, AdmissionError> + Send + Sync,
{
    fn verify(
        &self,
        opaque_ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
        now_unix_ms: u64,
    ) -> Result<VerifiedTicketClaims, AdmissionError> {
        self(opaque_ticket, expected_dh_pub_x25519, now_unix_ms)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AdmissionIdentity {
    claims: VerifiedTicketClaims,
    remote_static: [u8; 32],
}

impl AdmissionIdentity {
    pub fn claims(&self) -> &VerifiedTicketClaims {
        &self.claims
    }

    pub fn remote_static(&self) -> &[u8; 32] {
        &self.remote_static
    }

    pub fn to_auth_metadata(&self) -> VerifiedAuthMetadata {
        VerifiedAuthMetadata {
            issuer: self.claims.issuer.clone(),
            subject: self.claims.subject.clone(),
            device_id: self.claims.device_id.clone(),
            proof_binding: URL_SAFE_NO_PAD.encode(self.remote_static),
            expires_at_unix_ms: self.claims.expires_at_unix_ms,
            display_name: self.claims.display_name.clone(),
            avatar_url: self.claims.avatar_url.clone(),
        }
    }
}

impl fmt::Debug for AdmissionIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmissionIdentity")
            .field("claims", &self.claims)
            .field("remote_static", &"[REDACTED]")
            .finish()
    }
}

pub fn verify_initial_ticket(
    verifier: &dyn TicketVerifier,
    opaque_ticket: &[u8],
    remote_static: &[u8; 32],
    expected_issuer: &str,
    expected_subject: &str,
    now_unix_ms: u64,
) -> Result<AdmissionIdentity, AdmissionError> {
    let claims = verifier
        .verify(opaque_ticket, remote_static, now_unix_ms)
        .map_err(|_| AdmissionError::Verification)?;
    if claims.issuer != expected_issuer {
        return Err(AdmissionError::WrongIssuer);
    }
    if claims.subject != expected_subject {
        return Err(AdmissionError::WrongSubject);
    }
    if claims.expires_at_unix_ms <= now_unix_ms {
        return Err(AdmissionError::Expired);
    }
    if claims.dh_pub_x25519 != *remote_static {
        return Err(AdmissionError::StaticKeyMismatch);
    }
    Ok(AdmissionIdentity {
        claims,
        remote_static: *remote_static,
    })
}

pub fn verify_renewal_ticket(
    verifier: &dyn TicketVerifier,
    opaque_ticket: &[u8],
    previous: &AdmissionIdentity,
    now_unix_ms: u64,
) -> Result<AdmissionIdentity, AdmissionError> {
    let next = verifier
        .verify(opaque_ticket, &previous.remote_static, now_unix_ms)
        .map_err(|_| AdmissionError::Verification)?;
    if next.expires_at_unix_ms <= now_unix_ms {
        return Err(AdmissionError::Expired);
    }
    if next.issuer != previous.claims.issuer
        || next.subject != previous.claims.subject
        || next.device_id != previous.claims.device_id
        || next.dh_pub_x25519 != previous.remote_static
    {
        return Err(AdmissionError::RenewalIdentityChanged);
    }
    if next.expires_at_unix_ms <= previous.claims.expires_at_unix_ms {
        return Err(AdmissionError::RenewalDidNotExtend);
    }
    Ok(AdmissionIdentity {
        claims: next,
        remote_static: previous.remote_static,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeHint {
    pub participant_id: ParticipantId,
    pub peer_id: PeerId,
    pub peer_namespace: PeerNamespace,
    pub role: Role,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinIntent {
    New,
    Resume(ResumeHint),
}

pub struct AdmissionHello {
    ticket: Zeroizing<Vec<u8>>,
    intent: JoinIntent,
}

impl AdmissionHello {
    pub fn new(ticket: Vec<u8>, intent: JoinIntent) -> Result<Self, AdmissionError> {
        Self::new_sensitive(Zeroizing::new(ticket), intent)
    }

    /// Copies borrowed credential bytes directly into zeroizing ownership.
    pub fn from_ticket_bytes(ticket: &[u8], intent: JoinIntent) -> Result<Self, AdmissionError> {
        let mut owned = Zeroizing::new(Vec::with_capacity(ticket.len()));
        owned.extend_from_slice(ticket);
        Self::new_sensitive(owned, intent)
    }

    pub fn new_sensitive(
        ticket: Zeroizing<Vec<u8>>,
        intent: JoinIntent,
    ) -> Result<Self, AdmissionError> {
        if ticket.is_empty() || ticket.len() > MAX_TICKET_BYTES {
            return Err(AdmissionError::TooLarge {
                actual: ticket.len(),
                maximum: MAX_TICKET_BYTES,
            });
        }
        validate_intent(&intent)?;
        Ok(Self { ticket, intent })
    }

    pub fn ticket(&self) -> &[u8] {
        &self.ticket
    }

    pub fn intent(&self) -> &JoinIntent {
        &self.intent
    }

    pub fn encode(&self) -> Result<Zeroizing<Vec<u8>>, AdmissionError> {
        validate_intent(&self.intent)?;
        let ticket_len =
            u32::try_from(self.ticket.len()).map_err(|_| AdmissionError::TooLarge {
                actual: self.ticket.len(),
                maximum: MAX_TICKET_BYTES,
            })?;
        let (intent_tag, role_tag) = match &self.intent {
            JoinIntent::New => (0, 0),
            JoinIntent::Resume(hint) => (1, encode_role(hint.role)),
        };
        let mut encoded = Zeroizing::new(Vec::with_capacity(
            ADMISSION_FIXED_BYTES + self.ticket.len() + 64,
        ));
        encoded.extend_from_slice(ADMISSION_MAGIC);
        encoded.extend_from_slice(&ADMISSION_VERSION.to_be_bytes());
        encoded.push(intent_tag);
        encoded.push(role_tag);
        encoded.extend_from_slice(&ticket_len.to_be_bytes());
        encoded.extend_from_slice(&self.ticket);
        if let JoinIntent::Resume(hint) = &self.intent {
            append_string(&mut encoded, hint.participant_id.as_ref())?;
            append_string(&mut encoded, hint.peer_id.as_ref())?;
            append_string(&mut encoded, hint.peer_namespace.as_str())?;
        }
        if encoded.len() > MAX_CONTROL_TRANSFER_BYTES {
            return Err(AdmissionError::TooLarge {
                actual: encoded.len(),
                maximum: MAX_CONTROL_TRANSFER_BYTES,
            });
        }
        Ok(encoded)
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, AdmissionError> {
        if encoded.len() < ADMISSION_FIXED_BYTES || &encoded[..4] != ADMISSION_MAGIC {
            return Err(AdmissionError::Malformed);
        }
        if encoded.len() > MAX_CONTROL_TRANSFER_BYTES {
            return Err(AdmissionError::TooLarge {
                actual: encoded.len(),
                maximum: MAX_CONTROL_TRANSFER_BYTES,
            });
        }
        if u16::from_be_bytes([encoded[4], encoded[5]]) != ADMISSION_VERSION {
            return Err(AdmissionError::Malformed);
        }
        let intent_tag = encoded[6];
        let role_tag = encoded[7];
        let ticket_len =
            u32::from_be_bytes([encoded[8], encoded[9], encoded[10], encoded[11]]) as usize;
        if ticket_len == 0 || ticket_len > MAX_TICKET_BYTES {
            return Err(AdmissionError::TooLarge {
                actual: ticket_len,
                maximum: MAX_TICKET_BYTES,
            });
        }
        let ticket_end = ADMISSION_FIXED_BYTES
            .checked_add(ticket_len)
            .filter(|end| *end <= encoded.len())
            .ok_or(AdmissionError::Malformed)?;
        let mut ticket = Zeroizing::new(Vec::with_capacity(ticket_len));
        ticket.extend_from_slice(&encoded[ADMISSION_FIXED_BYTES..ticket_end]);
        let mut offset = ticket_end;
        let intent = match intent_tag {
            0 if role_tag == 0 && offset == encoded.len() => JoinIntent::New,
            1 => {
                let participant_id = read_string(encoded, &mut offset)?;
                let peer_id = read_string(encoded, &mut offset)?;
                let namespace = read_string(encoded, &mut offset)?;
                if offset != encoded.len() {
                    return Err(AdmissionError::Malformed);
                }
                let peer_namespace = PeerNamespace::try_from(namespace.as_str()).map_err(|_| {
                    AdmissionError::InvalidIdentifier {
                        field: "peer_namespace",
                    }
                })?;
                JoinIntent::Resume(ResumeHint {
                    participant_id: ParticipantId::from(participant_id),
                    peer_id: PeerId::from(peer_id),
                    peer_namespace,
                    role: decode_role(role_tag)?,
                })
            }
            _ => return Err(AdmissionError::Malformed),
        };
        Self::new_sensitive(ticket, intent)
    }
}

impl fmt::Debug for AdmissionHello {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmissionHello")
            .field("ticket", &"[REDACTED]")
            .field("intent", &self.intent)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionPhase {
    Encrypted,
    IdentityVerified,
    OwnerAuthorized,
    Active,
}

#[derive(Clone)]
pub enum AdmissionState {
    Encrypted {
        remote_static: [u8; 32],
    },
    IdentityVerified(AdmissionIdentity),
    OwnerAuthorized {
        identity: AdmissionIdentity,
        role: Role,
    },
    Active {
        identity: AdmissionIdentity,
        role: Role,
    },
}

impl AdmissionState {
    pub const fn phase(&self) -> AdmissionPhase {
        match self {
            Self::Encrypted { .. } => AdmissionPhase::Encrypted,
            Self::IdentityVerified(_) => AdmissionPhase::IdentityVerified,
            Self::OwnerAuthorized { .. } => AdmissionPhase::OwnerAuthorized,
            Self::Active { .. } => AdmissionPhase::Active,
        }
    }

    pub fn identity(&self) -> Option<&AdmissionIdentity> {
        match self {
            Self::Encrypted { .. } => None,
            Self::IdentityVerified(identity)
            | Self::OwnerAuthorized { identity, .. }
            | Self::Active { identity, .. } => Some(identity),
        }
    }

    pub const fn role(&self) -> Option<Role> {
        match self {
            Self::OwnerAuthorized { role, .. } | Self::Active { role, .. } => Some(*role),
            Self::Encrypted { .. } | Self::IdentityVerified(_) => None,
        }
    }

    pub fn authorize(self, role: Role) -> Result<Self, AdmissionError> {
        match self {
            Self::IdentityVerified(identity) => Ok(Self::OwnerAuthorized { identity, role }),
            _ => Err(AdmissionError::InvalidState),
        }
    }

    pub fn activate(self) -> Result<Self, AdmissionError> {
        match self {
            Self::OwnerAuthorized { identity, role } => Ok(Self::Active { identity, role }),
            _ => Err(AdmissionError::InvalidState),
        }
    }

    pub fn renew(
        self,
        verifier: &dyn TicketVerifier,
        opaque_ticket: &[u8],
        now_unix_ms: u64,
    ) -> Result<Self, AdmissionError> {
        match self {
            Self::IdentityVerified(identity) => {
                let identity =
                    verify_renewal_ticket(verifier, opaque_ticket, &identity, now_unix_ms)?;
                Ok(Self::IdentityVerified(identity))
            }
            Self::OwnerAuthorized { identity, role } => {
                let identity =
                    verify_renewal_ticket(verifier, opaque_ticket, &identity, now_unix_ms)?;
                Ok(Self::OwnerAuthorized { identity, role })
            }
            Self::Active { identity, role } => {
                let identity =
                    verify_renewal_ticket(verifier, opaque_ticket, &identity, now_unix_ms)?;
                Ok(Self::Active { identity, role })
            }
            Self::Encrypted { .. } => Err(AdmissionError::InvalidState),
        }
    }
}

impl fmt::Debug for AdmissionState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut state = formatter.debug_struct("AdmissionState");
        state.field("phase", &self.phase());
        if let Some(identity) = self.identity() {
            state.field("identity", identity);
        }
        if let Some(role) = self.role() {
            state.field("role", &role);
        }
        state.finish()
    }
}

fn validate_intent(intent: &JoinIntent) -> Result<(), AdmissionError> {
    let JoinIntent::Resume(hint) = intent else {
        return Ok(());
    };
    validate_identifier("participant_id", hint.participant_id.as_ref())?;
    validate_identifier("peer_id", hint.peer_id.as_ref())?;
    validate_identifier("peer_namespace", hint.peer_namespace.as_str())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), AdmissionError> {
    if value.is_empty() || value.len() > MAX_IDENTIFIER_BYTES as usize {
        return Err(AdmissionError::InvalidIdentifier { field });
    }
    Ok(())
}

fn valid_canonical_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36
        || bytes[8] != b'-'
        || bytes[13] != b'-'
        || bytes[18] != b'-'
        || bytes[23] != b'-'
    {
        return false;
    }
    let mut any_nonzero = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            continue;
        }
        if !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return false;
        }
        any_nonzero |= byte != b'0';
    }
    any_nonzero
}

fn valid_profile_display_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES
        && value.chars().count() <= MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_profile_avatar_url(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_COLLAB_PROFILE_AVATAR_URL_BYTES
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        || value.contains(['#', '\\'])
    {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = rest
        .split_once(['/', '?'])
        .map_or(rest, |(authority, _)| authority);
    valid_https_authority(authority)
}

fn valid_https_origin(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_TICKET_ISSUER_BYTES
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return false;
    }
    let Some(authority) = value.strip_prefix("https://") else {
        return false;
    };
    !authority.is_empty()
        && !authority.contains(['/', '?', '#', '\\', '@'])
        && valid_https_authority(authority)
}

fn valid_https_authority(authority: &str) -> bool {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let Some(closing_bracket) = bracketed.find(']') else {
            return false;
        };
        let host = &bracketed[..closing_bracket];
        let suffix = &bracketed[closing_bracket + 1..];
        return host.parse::<std::net::Ipv6Addr>().is_ok()
            && (suffix.is_empty() || suffix.strip_prefix(':').is_some_and(valid_https_port));
    }
    if authority.contains(['[', ']']) {
        return false;
    }
    let host = match authority.rsplit_once(':') {
        Some((host, port)) => {
            if host.contains(':') || !valid_https_port(port) {
                return false;
            }
            host
        }
        None => authority,
    };
    if host.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

fn valid_https_port(port: &str) -> bool {
    !port.is_empty()
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|port| port != 0)
}

fn append_string(output: &mut Vec<u8>, value: &str) -> Result<(), AdmissionError> {
    validate_identifier("resume", value)?;
    let length = u16::try_from(value.len()).map_err(|_| AdmissionError::Malformed)?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(encoded: &[u8], offset: &mut usize) -> Result<String, AdmissionError> {
    let length_bytes = encoded
        .get(*offset..*offset + 2)
        .ok_or(AdmissionError::Malformed)?;
    let length = usize::from(u16::from_be_bytes([length_bytes[0], length_bytes[1]]));
    *offset += 2;
    if length == 0 || length > MAX_IDENTIFIER_BYTES as usize {
        return Err(AdmissionError::Malformed);
    }
    let end = offset
        .checked_add(length)
        .filter(|end| *end <= encoded.len())
        .ok_or(AdmissionError::Malformed)?;
    let value = std::str::from_utf8(&encoded[*offset..end])
        .map_err(|_| AdmissionError::Malformed)?
        .to_owned();
    *offset = end;
    Ok(value)
}

fn encode_role(role: Role) -> u8 {
    match role {
        Role::Owner => 1,
        Role::Editor => 2,
        Role::Viewer => 3,
    }
}

fn decode_role(encoded: u8) -> Result<Role, AdmissionError> {
    match encoded {
        1 => Ok(Role::Owner),
        2 => Ok(Role::Editor),
        3 => Ok(Role::Viewer),
        _ => Err(AdmissionError::Malformed),
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
