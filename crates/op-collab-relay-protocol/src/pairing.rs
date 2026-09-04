//! Short pairing codes: a human-typeable handle that redeems to a full
//! [`RelayInviteV1`].
//!
//! A 503-char invite cannot be read over a shoulder or typed from a phone
//! call. The pairing code fixes that by moving the invite bytes to the
//! control plane, sealed under a key only holders of the code can derive:
//!
//! - The code is 10 chars of Crockford base32. The first char names the
//!   relay region (so a guest contacts exactly one control plane); the other
//!   nine are random → 45 bits of entropy.
//! - `code_id` retains the v0.8.4 domain-separated BLAKE3 derivation,
//!   truncated to 128 bits for the storage key. Keeping this lookup handle
//!   stable does not make the old and new sealed envelopes interoperable.
//! - The sealing key is independently derived with HKDF-SHA256, using the
//!   random 96-bit nonce as salt. The invite is protected by the RFC 8439
//!   ChaCha20-Poly1305 AEAD with the complete versioned header as
//!   domain-separated additional authenticated data (AAD).
//!
//! An online guesser must present a valid `code_id`, which requires the code
//! itself; the 2^45 space, the ≤1h server TTL, and the control plane's rate
//! limits make that infeasible. The control-plane operator holding a blob can
//! grind the 45-bit space offline — recovering (or forging) the sealed
//! invite. Session admission still requires an authenticated guest, and a
//! forged owner identity must survive the guest's owner-confirmation gate,
//! but unlike the long invite the short code is NOT confidential or
//! authentic against the storage operator; see the threat model.

use core::fmt;

use chacha20poly1305::{
    aead::{AeadInPlace as _, KeyInit as _},
    ChaCha20Poly1305, Key, Nonce, Tag,
};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::{RelayInviteV1, RelayProtocolError, RelayRegion, MAX_INVITE_CHARS};

/// Exact canonical length of a pairing code.
pub const PAIRING_CODE_CHARS: usize = 10;
/// First-char region tags. The region rides in the code so a guest claims
/// from exactly one control plane instead of spraying the code id (and its
/// bearer ticket) across every region.
pub const PAIRING_REGION_CN_TAG: u8 = b'1';
pub const PAIRING_REGION_GLOBAL_TAG: u8 = b'2';
/// Crockford base32: no I, L, O (confusable) and no U (accidental words).
pub const PAIRING_CODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Storage lookup handle length.
pub const PAIRING_CODE_ID_BYTES: usize = 16;
/// Version of the sealed pairing-invite envelope. This is intentionally
/// independent from [`crate::RELAY_PROTOCOL_VERSION`].
pub const SEALED_PAIRING_INVITE_VERSION: u8 = 2;
/// Version byte of the legacy sealed envelope published by OpenPencil
/// v0.8.4 (BLAKE3-XOF/XOR stream + keyed-BLAKE3 encrypt-then-MAC).
pub const SEALED_PAIRING_INVITE_V1_VERSION: u8 = 1;
/// RFC 8439 ChaCha20-Poly1305 nonce length (96 bits).
pub const SEALED_INVITE_NONCE_BYTES: usize = 12;
/// Full RFC 8439 Poly1305 tag length (128 bits; never truncated).
pub const SEALED_INVITE_TAG_BYTES: usize = 16;
/// Legacy v1 envelope nonce length (mixed into both BLAKE3 subkeys).
pub const SEALED_INVITE_V1_NONCE_BYTES: usize = 24;
/// Legacy v1 envelope MAC tag length (full BLAKE3 output).
pub const SEALED_INVITE_V1_TAG_BYTES: usize = 32;
/// Opaque transport/storage ceiling retained from the published v0.8.4
/// envelope: version + 24-byte nonce + longest fragment + 32-byte tag. Relay
/// infrastructure must remain able to forward legacy blobs during rollout.
pub const MAX_SEALED_INVITE_BYTES: usize = 1 + 24 + MAX_INVITE_CHARS + 32;
/// Tight parser/sealer ceiling for the sealed-v2 RFC 8439 envelope.
pub const MAX_SEALED_PAIRING_INVITE_V2_BYTES: usize =
    1 + SEALED_INVITE_NONCE_BYTES + MAX_INVITE_CHARS + SEALED_INVITE_TAG_BYTES;

const SEALED_INVITE_HEADER_BYTES: usize = 1 + SEALED_INVITE_NONCE_BYTES;
const SEALED_INVITE_V1_HEADER_BYTES: usize = 1 + SEALED_INVITE_V1_NONCE_BYTES;
const SEALED_INVITE_KEY_BYTES: usize = 32;
const CODE_ID_CONTEXT: &str = "openpencil/op-collab-relay-protocol/pairing-code-id/v1";
const V1_ENC_KEY_CONTEXT: &str = "openpencil/op-collab-relay-protocol/pairing-code-enc-key/v1";
const V1_MAC_KEY_CONTEXT: &str = "openpencil/op-collab-relay-protocol/pairing-code-mac-key/v1";
const SEALED_INVITE_KEY_INFO: &[u8] =
    b"openpencil/op-collab-relay-protocol/sealed-pairing-invite/chacha20poly1305-key/v2\0";
const SEALED_INVITE_AAD_DOMAIN: &[u8] =
    b"openpencil/op-collab-relay-protocol/sealed-pairing-invite/chacha20poly1305-aad/v2\0";

/// A canonical (uppercase, exact-length) short pairing code.
///
/// Treat the value as a secret: it derives the sealing key. `Debug` is
/// redacted and the buffer zeroizes on drop.
#[derive(Clone)]
pub struct PairingCode(Zeroizing<[u8; PAIRING_CODE_CHARS]>);

impl PairingCode {
    /// Parse user input into canonical form. Accepts lowercase and the
    /// Crockford confusables (`I`/`L` → `1`, `O` → `0`); everything else
    /// must be in the alphabet, and the length must be exact after
    /// whitespace/dash trimming.
    pub fn parse(input: &str) -> Result<Self, RelayProtocolError> {
        let mut canonical = Zeroizing::new([0_u8; PAIRING_CODE_CHARS]);
        let mut length = 0_usize;
        for byte in input.bytes() {
            if byte.is_ascii_whitespace() || byte == b'-' {
                continue;
            }
            let Some(mapped) = canonical_code_byte(byte) else {
                return Err(RelayProtocolError::InvalidPairingCode);
            };
            if length == PAIRING_CODE_CHARS {
                return Err(RelayProtocolError::InvalidPairingCode);
            }
            canonical[length] = mapped;
            length += 1;
        }
        if length != PAIRING_CODE_CHARS {
            return Err(RelayProtocolError::InvalidPairingCode);
        }
        Ok(Self(canonical))
    }

    /// Whether user input is a claimable pairing code: canonical shape AND a
    /// valid region tag. The region requirement keeps 10-char LAN hostnames
    /// (`renderfarm`) out of the pairing branch of join dispatch.
    pub fn looks_like(input: &str) -> bool {
        Self::parse(input).is_ok_and(|code| code.region().is_some())
    }

    /// The relay region encoded in the first character, `None` when the tag
    /// is not a known region — such a code cannot be claimed anywhere.
    pub fn region(&self) -> Option<RelayRegion> {
        match self.0[0] {
            PAIRING_REGION_CN_TAG => Some(RelayRegion::Cn),
            PAIRING_REGION_GLOBAL_TAG => Some(RelayRegion::Global),
            _ => None,
        }
    }

    /// Generate a fresh random code claimable in `region`: one region tag
    /// plus nine random alphabet chars (45 bits).
    #[cfg(feature = "random")]
    pub fn generate_for(region: RelayRegion) -> Result<Self, RelayProtocolError> {
        let mut raw = Zeroizing::new([0_u8; PAIRING_CODE_CHARS - 1]);
        getrandom::fill(&mut *raw).map_err(|_| RelayProtocolError::RandomUnavailable)?;
        let mut canonical = Zeroizing::new([0_u8; PAIRING_CODE_CHARS]);
        canonical[0] = match region {
            RelayRegion::Cn => PAIRING_REGION_CN_TAG,
            RelayRegion::Global => PAIRING_REGION_GLOBAL_TAG,
        };
        for (slot, byte) in canonical[1..].iter_mut().zip(raw.iter().copied()) {
            *slot = PAIRING_CODE_ALPHABET[(byte % 32) as usize];
        }
        Ok(Self(canonical))
    }

    /// The canonical display form. Secret — show it only in the owner's
    /// share surface, never in logs.
    pub fn expose_str(&self) -> &str {
        core::str::from_utf8(self.0.as_slice()).expect("alphabet is ASCII")
    }

    /// Storage lookup handle. Independent from the sealing key, so handing
    /// it to the control plane does not hand over the blob's contents.
    pub fn code_id(&self) -> [u8; PAIRING_CODE_ID_BYTES] {
        let digest = blake3::Hasher::new_derive_key(CODE_ID_CONTEXT)
            .update(self.0.as_slice())
            .finalize();
        let mut id = [0_u8; PAIRING_CODE_ID_BYTES];
        id.copy_from_slice(&digest.as_bytes()[..PAIRING_CODE_ID_BYTES]);
        id
    }

    fn sealing_key(
        &self,
        nonce: &[u8; SEALED_INVITE_NONCE_BYTES],
    ) -> Result<Zeroizing<[u8; SEALED_INVITE_KEY_BYTES]>, RelayProtocolError> {
        let mut key = Zeroizing::new([0_u8; SEALED_INVITE_KEY_BYTES]);
        {
            let hkdf = Hkdf::<Sha256>::new(Some(nonce), self.0.as_slice());
            hkdf.expand(SEALED_INVITE_KEY_INFO, &mut *key)
                .map_err(|_| RelayProtocolError::SealedInviteKeyDerivationFailed)?;
        }
        Ok(key)
    }

    /// Legacy v1 subkey derivation: domain-separated BLAKE3 over the
    /// canonical code bytes and the 24-byte envelope nonce. Retained so a
    /// mixed-version fleet can keep pairing during the v1→v2 transition.
    fn legacy_subkey(
        &self,
        context: &str,
        nonce: &[u8; SEALED_INVITE_V1_NONCE_BYTES],
    ) -> Zeroizing<[u8; SEALED_INVITE_KEY_BYTES]> {
        Zeroizing::new(
            *blake3::Hasher::new_derive_key(context)
                .update(self.0.as_slice())
                .update(nonce)
                .finalize()
                .as_bytes(),
        )
    }
}

impl fmt::Debug for PairingCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingCode([REDACTED])")
    }
}

fn canonical_code_byte(byte: u8) -> Option<u8> {
    let upper = byte.to_ascii_uppercase();
    let mapped = match upper {
        b'I' | b'L' => b'1',
        b'O' => b'0',
        other => other,
    };
    PAIRING_CODE_ALPHABET.contains(&mapped).then_some(mapped)
}

/// An invite fragment sealed under a pairing code.
///
/// Binary layout: `[sealed_version=2][nonce:12][ciphertext][tag:16]`.
/// HKDF-SHA256 derives a 256-bit key from the pairing code with the nonce as
/// salt. ChaCha20-Poly1305 authenticates the invite and the AAD
/// `domain || sealed_version || nonce` with its full RFC 8439 tag. The owned
/// blob is zeroized on drop even though it contains ciphertext rather than
/// plaintext.
#[derive(Clone, PartialEq, Eq)]
pub struct SealedPairingInvite(Zeroizing<Vec<u8>>);

impl SealedPairingInvite {
    /// Seal an invite under `code` with a caller-supplied nonce. It must be
    /// nonzero and must never be reused with the same code.
    pub fn seal(
        code: &PairingCode,
        invite: &RelayInviteV1,
        nonce: [u8; SEALED_INVITE_NONCE_BYTES],
    ) -> Result<Self, RelayProtocolError> {
        validate_nonce(&nonce)?;
        let mut body = Zeroizing::new(invite.to_fragment().into_bytes());
        let key = code.sealing_key(&nonce)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&*key));
        let header = sealed_invite_header(&nonce);
        let aad = sealed_invite_aad(&header);
        let tag = cipher
            .encrypt_in_place_detached(Nonce::from_slice(&nonce), &aad, &mut body)
            .map_err(|_| RelayProtocolError::SealedInviteEncryptionFailed)?;
        let mut raw = Vec::with_capacity(
            1 + SEALED_INVITE_NONCE_BYTES + body.len() + SEALED_INVITE_TAG_BYTES,
        );
        raw.extend_from_slice(&header);
        raw.extend_from_slice(&body);
        raw.extend_from_slice(&tag);
        Ok(Self(Zeroizing::new(raw)))
    }

    /// Seal with a fresh random nonce.
    #[cfg(feature = "random")]
    pub fn seal_random(
        code: &PairingCode,
        invite: &RelayInviteV1,
    ) -> Result<Self, RelayProtocolError> {
        let mut nonce = [0_u8; SEALED_INVITE_NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| RelayProtocolError::RandomUnavailable)?;
        Self::seal(code, invite, nonce)
    }

    /// Seal in the legacy v1 envelope that every fielded client can open.
    ///
    /// Fleet-transition writer: OpenPencil v0.8.4 desktops reject any other
    /// envelope version at claim time, so an owner that seals v2 mints codes
    /// those guests can claim but never open ("invalid code"). Publish v1
    /// until the fielded readers understand v2, then switch the owner back
    /// to [`Self::seal_random`]. Opening supports both versions either way.
    pub fn seal_legacy_compat(
        code: &PairingCode,
        invite: &RelayInviteV1,
        nonce: [u8; SEALED_INVITE_V1_NONCE_BYTES],
    ) -> Self {
        let fragment = Zeroizing::new(invite.to_fragment());
        let mut body = Zeroizing::new(fragment.as_bytes().to_vec());
        legacy_apply_keystream(&code.legacy_subkey(V1_ENC_KEY_CONTEXT, &nonce), &mut body);
        let mut raw = Vec::with_capacity(
            SEALED_INVITE_V1_HEADER_BYTES + body.len() + SEALED_INVITE_V1_TAG_BYTES,
        );
        raw.push(SEALED_PAIRING_INVITE_V1_VERSION);
        raw.extend_from_slice(&nonce);
        raw.extend_from_slice(&body);
        let tag = legacy_mac_tag(&code.legacy_subkey(V1_MAC_KEY_CONTEXT, &nonce), &raw);
        raw.extend_from_slice(tag.as_bytes());
        Self(Zeroizing::new(raw))
    }

    /// Legacy v1 seal with a fresh random nonce.
    #[cfg(feature = "random")]
    pub fn seal_random_legacy_compat(
        code: &PairingCode,
        invite: &RelayInviteV1,
    ) -> Result<Self, RelayProtocolError> {
        let mut nonce = [0_u8; SEALED_INVITE_V1_NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| RelayProtocolError::RandomUnavailable)?;
        Ok(Self::seal_legacy_compat(code, invite, nonce))
    }

    /// Accept an untrusted blob from the wire, checking envelope version,
    /// bounds, and nonce shape. The authenticity check happens in
    /// [`Self::open`].
    pub fn from_bytes(raw: &[u8]) -> Result<Self, RelayProtocolError> {
        validate_sealed_invite_bytes(raw)?;
        Ok(Self(Zeroizing::new(raw.to_vec())))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Authenticate and decrypt with `code`, returning the parsed invite.
    /// A wrong code fails the constant-time tag comparison before any
    /// plaintext is interpreted. Both the current v2 AEAD envelope and the
    /// legacy v1 envelope published by v0.8.4 owners are accepted.
    pub fn open(&self, code: &PairingCode) -> Result<RelayInviteV1, RelayProtocolError> {
        let raw = &self.0;
        validate_sealed_invite_bytes(raw)?;
        if raw[0] == SEALED_PAIRING_INVITE_V1_VERSION {
            return self.open_v1(code);
        }
        let tag_offset = raw.len() - SEALED_INVITE_TAG_BYTES;
        let mut nonce = [0_u8; SEALED_INVITE_NONCE_BYTES];
        nonce.copy_from_slice(&raw[1..SEALED_INVITE_HEADER_BYTES]);
        let key = code.sealing_key(&nonce)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&*key));
        let aad = sealed_invite_aad(&raw[..SEALED_INVITE_HEADER_BYTES]);
        let mut body = Zeroizing::new(raw[SEALED_INVITE_HEADER_BYTES..tag_offset].to_vec());
        cipher
            .decrypt_in_place_detached(
                Nonce::from_slice(&nonce),
                &aad,
                &mut body,
                Tag::from_slice(&raw[tag_offset..]),
            )
            .map_err(|_| RelayProtocolError::InvalidPairingCode)?;
        let fragment =
            core::str::from_utf8(&body).map_err(|_| RelayProtocolError::InvalidSealedInvite)?;
        RelayInviteV1::from_fragment(fragment).map_err(|_| RelayProtocolError::InvalidSealedInvite)
    }

    /// Legacy v1 open: keyed-BLAKE3 encrypt-then-MAC over
    /// `[version=1][nonce:24][ciphertext]`, byte-compatible with the sealed
    /// envelope shipped in OpenPencil v0.8.4.
    fn open_v1(&self, code: &PairingCode) -> Result<RelayInviteV1, RelayProtocolError> {
        let raw = &self.0;
        let tag_offset = raw.len() - SEALED_INVITE_V1_TAG_BYTES;
        let mut nonce = [0_u8; SEALED_INVITE_V1_NONCE_BYTES];
        nonce.copy_from_slice(&raw[1..SEALED_INVITE_V1_HEADER_BYTES]);
        let expected = legacy_mac_tag(
            &code.legacy_subkey(V1_MAC_KEY_CONTEXT, &nonce),
            &raw[..tag_offset],
        );
        let presented = blake3::Hash::from_bytes(
            raw[tag_offset..]
                .try_into()
                .expect("v1 tag range is fixed width"),
        );
        // `blake3::Hash` equality is constant-time.
        if expected != presented {
            return Err(RelayProtocolError::InvalidPairingCode);
        }
        let mut body = Zeroizing::new(raw[SEALED_INVITE_V1_HEADER_BYTES..tag_offset].to_vec());
        legacy_apply_keystream(&code.legacy_subkey(V1_ENC_KEY_CONTEXT, &nonce), &mut body);
        let fragment =
            core::str::from_utf8(&body).map_err(|_| RelayProtocolError::InvalidSealedInvite)?;
        RelayInviteV1::from_fragment(fragment).map_err(|_| RelayProtocolError::InvalidSealedInvite)
    }
}

impl fmt::Debug for SealedPairingInvite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SealedPairingInvite([REDACTED])")
    }
}

fn validate_nonce(nonce: &[u8; SEALED_INVITE_NONCE_BYTES]) -> Result<(), RelayProtocolError> {
    if nonce.iter().all(|byte| *byte == 0) {
        return Err(RelayProtocolError::ZeroSealedInviteNonce);
    }
    Ok(())
}

fn validate_sealed_invite_bytes(raw: &[u8]) -> Result<(), RelayProtocolError> {
    // The one-byte minimum admits the version dispatch below; per-version
    // bounds are checked once the envelope version is known.
    if raw.is_empty() || raw.len() > MAX_SEALED_INVITE_BYTES {
        return Err(RelayProtocolError::InvalidSealedInvite);
    }
    match raw[0] {
        SEALED_PAIRING_INVITE_VERSION => {
            let minimum = SEALED_INVITE_HEADER_BYTES + SEALED_INVITE_TAG_BYTES + 1;
            if raw.len() < minimum || raw.len() > MAX_SEALED_PAIRING_INVITE_V2_BYTES {
                return Err(RelayProtocolError::InvalidSealedInvite);
            }
            let nonce: &[u8; SEALED_INVITE_NONCE_BYTES] = raw[1..SEALED_INVITE_HEADER_BYTES]
                .try_into()
                .expect("sealed invite nonce range is fixed width");
            validate_nonce(nonce)
        }
        // Legacy v1 blobs carry no nonce-shape rule beyond bounds; v0.8.4
        // published CSPRNG nonces and its parser accepted any value.
        SEALED_PAIRING_INVITE_V1_VERSION => {
            let minimum = SEALED_INVITE_V1_HEADER_BYTES + SEALED_INVITE_V1_TAG_BYTES + 1;
            if raw.len() < minimum {
                return Err(RelayProtocolError::InvalidSealedInvite);
            }
            Ok(())
        }
        other => Err(RelayProtocolError::UnsupportedSealedInviteVersion {
            actual: other,
            expected: SEALED_PAIRING_INVITE_VERSION,
        }),
    }
}

fn sealed_invite_header(
    nonce: &[u8; SEALED_INVITE_NONCE_BYTES],
) -> [u8; SEALED_INVITE_HEADER_BYTES] {
    let mut header = [0_u8; SEALED_INVITE_HEADER_BYTES];
    header[0] = SEALED_PAIRING_INVITE_VERSION;
    header[1..].copy_from_slice(nonce);
    header
}

/// Legacy v1 stream cipher: XOR with a keyed-BLAKE3 XOF keystream.
fn legacy_apply_keystream(key: &[u8; SEALED_INVITE_KEY_BYTES], body: &mut [u8]) {
    let mut reader = blake3::Hasher::new_keyed(key).finalize_xof();
    let mut stream = Zeroizing::new(vec![0_u8; body.len()]);
    reader.fill(stream.as_mut_slice());
    for (byte, mask) in body.iter_mut().zip(stream.iter()) {
        *byte ^= mask;
    }
}

/// Legacy v1 MAC: keyed BLAKE3 over `[version][nonce][ciphertext]`.
fn legacy_mac_tag(key: &[u8; SEALED_INVITE_KEY_BYTES], bytes: &[u8]) -> blake3::Hash {
    blake3::Hasher::new_keyed(key).update(bytes).finalize()
}

fn sealed_invite_aad(header: &[u8]) -> Vec<u8> {
    debug_assert_eq!(header.len(), SEALED_INVITE_HEADER_BYTES);
    let mut aad = Vec::with_capacity(SEALED_INVITE_AAD_DOMAIN.len() + header.len());
    aad.extend_from_slice(SEALED_INVITE_AAD_DOMAIN);
    aad.extend_from_slice(header);
    aad
}
