//! Credential-aware collaboration JSON decoding.
//!
//! The generic decoder must identify renewal frames before constructing a
//! `serde_json::Value`. The dedicated decoder must additionally avoid
//! serde_json's ordinary scratch buffer when unescaping the ticket string.

use std::borrow::Cow;

use serde::Deserialize;
use serde_json::value::RawValue;
use zeroize::Zeroizing;

use crate::{
    codec::{
        enforce_envelope_limit, enforce_json_nesting_limit, json_nesting_limit,
        validate_wire_limits,
    },
    CollabMessage, Epoch, FrameEnvelope, OpaqueTicket, OpaqueTicketError, ProtocolError,
    RenewTicket, SessionId, WireLimits, COLLAB_PROTOCOL_VERSION, MAX_OPAQUE_TICKET_BYTES,
};

/// A strict envelope whose values remain borrowed JSON slices.
///
/// `&RawValue` validates and skips each value without decoding string contents.
/// In particular, the credential-bearing body stays in the caller's bounded
/// input buffer until the dedicated zeroizing decoder handles it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BorrowedFrameEnvelope<'a> {
    #[serde(rename = "protocolVersion", borrow)]
    protocol_version: &'a RawValue,
    #[serde(rename = "sessionId", borrow)]
    session_id: &'a RawValue,
    #[serde(rename = "epoch", borrow)]
    epoch: &'a RawValue,
    #[serde(borrow)]
    body: &'a RawValue,
}

/// The discriminant may use serde's ordinary storage when JSON-escaped, but it
/// is fixed non-secret metadata. The entire payload remains borrowed raw JSON.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BorrowedMessageBody<'a> {
    #[serde(rename = "type", borrow)]
    kind: Cow<'a, str>,
    #[serde(rename = "payload", borrow)]
    payload: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BorrowedRenewTicketPayload<'a> {
    #[serde(borrow)]
    opaque_ticket: &'a RawValue,
}

/// Rejects credential-bearing input before the generic decoder can construct a
/// JSON value tree. The caller has already enforced envelope and nesting limits.
pub(crate) fn reject_renew_ticket_before_generic_value_decode(
    encoded: &[u8],
) -> Result<(), ProtocolError> {
    let frame = parse_borrowed_envelope(encoded)?;
    let body = parse_borrowed_body(frame.body)?;
    if body.kind == "renew_ticket" {
        return Err(ProtocolError::SensitiveCredentialRequiresDedicatedCodec);
    }
    Ok(())
}

/// Decodes a declared renewal transfer without copying ticket plaintext into
/// ordinary decoded storage.
///
/// The caller retains ownership of `encoded`; the native transport passes its
/// zeroizing reassembly buffer here. The returned credential is zeroizing.
pub fn decode_renew_ticket_frame_from_json_slice(
    encoded: &[u8],
    limits: WireLimits,
) -> Result<FrameEnvelope, ProtocolError> {
    validate_wire_limits(limits)?;
    enforce_envelope_limit(encoded.len(), limits)?;
    enforce_json_nesting_limit(encoded, json_nesting_limit(limits))?;

    let frame = parse_borrowed_envelope(encoded)?;
    let body = parse_borrowed_body(frame.body)?;
    if body.kind != "renew_ticket" {
        return Err(ProtocolError::SensitiveCredentialRequiresDedicatedCodec);
    }
    let payload: BorrowedRenewTicketPayload<'_> =
        serde_json::from_str(body.payload.get()).map_err(ProtocolError::Decode)?;
    let opaque_ticket = decode_ticket_string(payload.opaque_ticket.get())?;

    let protocol_version: u16 =
        serde_json::from_str(frame.protocol_version.get()).map_err(ProtocolError::Decode)?;
    if protocol_version != COLLAB_PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion {
            actual: protocol_version,
            expected: COLLAB_PROTOCOL_VERSION,
        });
    }
    let session_id: SessionId =
        serde_json::from_str(frame.session_id.get()).map_err(ProtocolError::Decode)?;
    let epoch: Epoch = serde_json::from_str(frame.epoch.get()).map_err(ProtocolError::Decode)?;
    let envelope = FrameEnvelope {
        protocol_version,
        session_id,
        epoch,
        body: CollabMessage::RenewTicket(RenewTicket { opaque_ticket }),
    };
    crate::codec::validate_envelope(&envelope, limits)?;
    Ok(envelope)
}

fn parse_borrowed_envelope(encoded: &[u8]) -> Result<BorrowedFrameEnvelope<'_>, ProtocolError> {
    serde_json::from_slice(encoded).map_err(ProtocolError::Decode)
}

fn parse_borrowed_body(body: &RawValue) -> Result<BorrowedMessageBody<'_>, ProtocolError> {
    serde_json::from_str(body.get()).map_err(ProtocolError::Decode)
}

/// Decodes one already-syntax-checked JSON string directly into zeroizing
/// storage. No serde string visitor or ordinary decoded scratch buffer sees the
/// ticket plaintext, including for escape sequences and Unicode surrogate
/// pairs.
fn decode_ticket_string(raw: &str) -> Result<OpaqueTicket, ProtocolError> {
    let bytes = raw.as_bytes();
    if bytes.len() < 2 || bytes.first() != Some(&b'"') || bytes.last() != Some(&b'"') {
        return Err(ticket_json_error("opaque ticket must be a JSON string"));
    }
    let maximum_literal_bytes = MAX_OPAQUE_TICKET_BYTES
        .checked_mul(6)
        .and_then(|maximum| maximum.checked_add(2))
        .expect("ticket limit expansion is representable");
    if bytes.len() > maximum_literal_bytes {
        return Err(ticket_json_error(
            "encoded opaque ticket exceeds the maximum expansion bound",
        ));
    }

    let content_end = bytes.len() - 1;
    let mut decoded = Zeroizing::new(String::with_capacity(
        content_end.saturating_sub(1).min(MAX_OPAQUE_TICKET_BYTES),
    ));
    let mut index = 1;
    while index < content_end {
        let byte = bytes[index];
        match byte {
            b'\\' => {
                index += 1;
                let escape = *bytes
                    .get(index)
                    .filter(|_| index < content_end)
                    .ok_or_else(|| ticket_json_error("incomplete opaque ticket escape"))?;
                match escape {
                    b'"' => {
                        push_ticket_char(&mut decoded, '"')?;
                        index += 1;
                    }
                    b'\\' => {
                        push_ticket_char(&mut decoded, '\\')?;
                        index += 1;
                    }
                    b'/' => {
                        push_ticket_char(&mut decoded, '/')?;
                        index += 1;
                    }
                    b'b' => {
                        push_ticket_char(&mut decoded, '\u{0008}')?;
                        index += 1;
                    }
                    b'f' => {
                        push_ticket_char(&mut decoded, '\u{000c}')?;
                        index += 1;
                    }
                    b'n' => {
                        push_ticket_char(&mut decoded, '\n')?;
                        index += 1;
                    }
                    b'r' => {
                        push_ticket_char(&mut decoded, '\r')?;
                        index += 1;
                    }
                    b't' => {
                        push_ticket_char(&mut decoded, '\t')?;
                        index += 1;
                    }
                    b'u' => {
                        let (first, next) = decode_hex_quad(bytes, index + 1, content_end)?;
                        index = next;
                        let scalar = if (0xd800..=0xdbff).contains(&first) {
                            if bytes.get(index) != Some(&b'\\')
                                || bytes.get(index + 1) != Some(&b'u')
                            {
                                return Err(ticket_json_error(
                                    "high surrogate must be followed by a low surrogate",
                                ));
                            }
                            let (second, next) = decode_hex_quad(bytes, index + 2, content_end)?;
                            if !(0xdc00..=0xdfff).contains(&second) {
                                return Err(ticket_json_error(
                                    "high surrogate must be followed by a low surrogate",
                                ));
                            }
                            index = next;
                            0x1_0000
                                + ((u32::from(first) - 0xd800) << 10)
                                + (u32::from(second) - 0xdc00)
                        } else if (0xdc00..=0xdfff).contains(&first) {
                            return Err(ticket_json_error(
                                "low surrogate must follow a high surrogate",
                            ));
                        } else {
                            u32::from(first)
                        };
                        let character = char::from_u32(scalar)
                            .ok_or_else(|| ticket_json_error("invalid Unicode scalar"))?;
                        push_ticket_char(&mut decoded, character)?;
                    }
                    _ => return Err(ticket_json_error("invalid opaque ticket escape")),
                }
            }
            b'"' => return Err(ticket_json_error("unescaped quote in opaque ticket")),
            0x00..=0x1f => {
                return Err(ticket_json_error(
                    "unescaped control character in opaque ticket",
                ))
            }
            0x20..=0x7f => {
                push_ticket_char(&mut decoded, char::from(byte))?;
                index += 1;
            }
            _ => {
                let character = raw
                    .get(index..content_end)
                    .ok_or_else(|| ticket_json_error("opaque ticket must be valid UTF-8"))?
                    .chars()
                    .next()
                    .ok_or_else(|| ticket_json_error("opaque ticket must be valid UTF-8"))?;
                push_ticket_char(&mut decoded, character)?;
                index += character.len_utf8();
            }
        }
    }

    OpaqueTicket::from_zeroizing(decoded).map_err(opaque_ticket_error)
}

fn push_ticket_char(decoded: &mut Zeroizing<String>, character: char) -> Result<(), ProtocolError> {
    let actual = decoded.len().saturating_add(character.len_utf8());
    if actual > MAX_OPAQUE_TICKET_BYTES {
        return Err(opaque_ticket_error(OpaqueTicketError::TooLarge {
            actual,
            limit: MAX_OPAQUE_TICKET_BYTES,
        }));
    }
    decoded.push(character);
    Ok(())
}

fn decode_hex_quad(
    bytes: &[u8],
    start: usize,
    content_end: usize,
) -> Result<(u16, usize), ProtocolError> {
    let end = start
        .checked_add(4)
        .filter(|end| *end <= content_end)
        .ok_or_else(|| ticket_json_error("incomplete Unicode escape in opaque ticket"))?;
    let mut value = 0_u16;
    for byte in &bytes[start..end] {
        let nibble = match byte {
            b'0'..=b'9' => u16::from(*byte - b'0'),
            b'a'..=b'f' => u16::from(*byte - b'a' + 10),
            b'A'..=b'F' => u16::from(*byte - b'A' + 10),
            _ => return Err(ticket_json_error("invalid Unicode escape in opaque ticket")),
        };
        value = (value << 4) | nibble;
    }
    Ok((value, end))
}

fn ticket_json_error(message: &'static str) -> ProtocolError {
    ProtocolError::Decode(<serde_json::Error as serde::de::Error>::custom(message))
}

fn opaque_ticket_error(error: OpaqueTicketError) -> ProtocolError {
    ProtocolError::Decode(<serde_json::Error as serde::de::Error>::custom(error))
}
