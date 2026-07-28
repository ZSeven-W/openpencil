use op_collab::{
    decode_renew_ticket_frame_from_json_slice, encode_renew_ticket_frame_to_zeroizing_json,
    CollabMessage, Epoch, FrameEnvelope, OpaqueTicket, ProtocolError, RenewTicket, SessionId,
    WireLimits, MAX_OPAQUE_TICKET_BYTES,
};
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(OpaqueTicket: Clone);
assert_not_impl_any!(RenewTicket: Clone);
assert_not_impl_any!(CollabMessage: Clone);
assert_not_impl_any!(FrameEnvelope: Clone);
assert_not_impl_any!(OpaqueTicket: serde::de::DeserializeOwned);
assert_not_impl_any!(RenewTicket: serde::de::DeserializeOwned);
assert_not_impl_any!(CollabMessage: serde::de::DeserializeOwned);

#[test]
fn generic_raw_codecs_reject_credential_frames() {
    let frame = FrameEnvelope::new(
        SessionId::from("credential-session"),
        Epoch(3),
        CollabMessage::RenewTicket(RenewTicket {
            opaque_ticket: OpaqueTicket::new("header.payload.signature".to_owned()).unwrap(),
        }),
    );
    assert!(matches!(
        frame.to_json_vec(),
        Err(ProtocolError::SensitiveCredentialRequiresDedicatedCodec)
    ));
}

#[test]
fn direct_serde_renewal_serialization_is_fail_closed() {
    const SECRET: &str = "header.payload.signature";

    let renewal = RenewTicket {
        opaque_ticket: OpaqueTicket::new(SECRET.to_owned()).unwrap(),
    };
    let renewal_error = serde_json::to_vec(&renewal).unwrap_err();
    assert!(renewal_error
        .to_string()
        .contains("dedicated renewal encoder"));
    assert!(!renewal_error.to_string().contains(SECRET));

    let message = CollabMessage::RenewTicket(RenewTicket {
        opaque_ticket: OpaqueTicket::new(SECRET.to_owned()).unwrap(),
    });
    let message_error = serde_json::to_vec(&message).unwrap_err();
    assert!(message_error
        .to_string()
        .contains("dedicated renewal encoder"));
    assert!(!message_error.to_string().contains(SECRET));
}

#[test]
fn generic_decoder_rejects_renewal_before_payload_deserialization() {
    let malformed_ticket_payload = br#"{
        "protocolVersion":1,
        "sessionId":"credential-session",
        "epoch":3,
        "body":{
            "type":"renew_ticket",
            "payload":{"opaqueTicket":{"mustNotBeDeserialized":"secret"}}
        }
    }"#;
    assert!(matches!(
        FrameEnvelope::from_json_slice(malformed_ticket_payload),
        Err(ProtocolError::SensitiveCredentialRequiresDedicatedCodec)
    ));
}

#[test]
fn sensitive_discriminator_rejects_duplicate_message_fields() {
    let duplicate_kind = br#"{
        "protocolVersion":1,
        "sessionId":"credential-session",
        "epoch":3,
        "body":{
            "type":"renew_ticket",
            "type":"renew_ticket",
            "payload":{"opaqueTicket":"header.payload.signature"}
        }
    }"#;
    assert!(matches!(
        FrameEnvelope::from_json_slice(duplicate_kind),
        Err(ProtocolError::Decode(_))
    ));
}

#[test]
fn dedicated_codec_round_trips_and_debug_redacts_the_secret() {
    const SECRET: &str = "header.payload.signature";
    let session_id = SessionId::from("credential-session");
    let ticket = OpaqueTicket::new(SECRET.to_owned()).unwrap();
    let encoded = encode_renew_ticket_frame_to_zeroizing_json(
        &session_id,
        Epoch(3),
        &ticket,
        WireLimits::default(),
    )
    .unwrap();
    let debug = format!("{encoded:?}");
    assert!(debug.contains("SensitiveFrameJson"));
    assert!(debug.contains("encoded_len"));
    assert!(!debug.contains(SECRET));
    assert!(!debug.contains(&format!("{:?}", SECRET.as_bytes())));

    let decoded =
        decode_renew_ticket_frame_from_json_slice(encoded.as_bytes(), WireLimits::default())
            .unwrap();
    let CollabMessage::RenewTicket(decoded) = decoded.into_body() else {
        panic!("dedicated codec must decode a renewal frame");
    };
    assert_eq!(decoded.opaque_ticket.expose(), SECRET);
}

#[test]
fn dedicated_decoder_unescapes_directly_into_zeroizing_storage() {
    let escaped = br#"{
        "protocolVersion":1,
        "sessionId":"credential-session",
        "epoch":3,
        "body":{
            "type":"renew\u005fticket",
            "payload":{
                "opaqueTicket":"ascii\\slash\/quote\"controls\n\r\t\b\f unicode-\u4e2d nul-\u0000 surrogate-\uD83D\uDD10 escaped-\u00e9"
            }
        }
    }"#;
    let decoded =
        decode_renew_ticket_frame_from_json_slice(escaped, WireLimits::default()).unwrap();
    let CollabMessage::RenewTicket(decoded) = decoded.into_body() else {
        panic!("escaped renewal must retain its message kind");
    };
    assert_eq!(
        decoded.opaque_ticket.expose(),
        "ascii\\slash/quote\"controls\n\r\t\u{0008}\u{000c} unicode-中 nul-\0 surrogate-🔐 escaped-é"
    );
}

#[test]
fn dedicated_decoder_rejects_malformed_escapes_and_surrogates() {
    for opaque_ticket in [
        r#""bad\xescape""#,
        r#""lone-high-\uD800""#,
        r#""wrong-pair-\uD800\u0041""#,
        r#""lone-low-\uDC00""#,
        r#""short-\u123""#,
    ] {
        let encoded = format!(
            r#"{{"protocolVersion":1,"sessionId":"credential-session","epoch":3,"body":{{"type":"renew_ticket","payload":{{"opaqueTicket":{opaque_ticket}}}}}}}"#
        );
        assert!(matches!(
            decode_renew_ticket_frame_from_json_slice(encoded.as_bytes(), WireLimits::default()),
            Err(ProtocolError::Decode(_))
        ));
    }
}

#[test]
fn dedicated_decoder_rejects_duplicate_and_unknown_ticket_fields() {
    for payload in [
        r#"{"opaqueTicket":"first","opaqueTicket":"second"}"#,
        r#"{"opaqueTicket":"secret","unexpected":true}"#,
    ] {
        let encoded = format!(
            r#"{{"protocolVersion":1,"sessionId":"credential-session","epoch":3,"body":{{"type":"renew_ticket","payload":{payload}}}}}"#
        );
        assert!(matches!(
            decode_renew_ticket_frame_from_json_slice(encoded.as_bytes(), WireLimits::default()),
            Err(ProtocolError::Decode(_))
        ));
    }
}

#[test]
fn dedicated_decoder_enforces_decoded_ticket_bounds() {
    let exact = OpaqueTicket::new("x".repeat(MAX_OPAQUE_TICKET_BYTES)).unwrap();
    let encoded = encode_renew_ticket_frame_to_zeroizing_json(
        &SessionId::from("credential-session"),
        Epoch(3),
        &exact,
        WireLimits::default(),
    )
    .unwrap();
    let decoded =
        decode_renew_ticket_frame_from_json_slice(encoded.as_bytes(), WireLimits::default())
            .unwrap();
    let CollabMessage::RenewTicket(decoded) = decoded.into_body() else {
        panic!("maximum-size renewal must retain its message kind");
    };
    assert_eq!(
        decoded.opaque_ticket.expose().len(),
        MAX_OPAQUE_TICKET_BYTES
    );

    let multibyte_value = "界".repeat(MAX_OPAQUE_TICKET_BYTES / "界".len());
    assert_eq!(multibyte_value.len(), MAX_OPAQUE_TICKET_BYTES);
    let multibyte = OpaqueTicket::new(multibyte_value.clone()).unwrap();
    let encoded = encode_renew_ticket_frame_to_zeroizing_json(
        &SessionId::from("credential-session"),
        Epoch(3),
        &multibyte,
        WireLimits::default(),
    )
    .unwrap();
    let decoded =
        decode_renew_ticket_frame_from_json_slice(encoded.as_bytes(), WireLimits::default())
            .unwrap();
    let CollabMessage::RenewTicket(decoded) = decoded.into_body() else {
        panic!("maximum-size Unicode renewal must retain its message kind");
    };
    assert_eq!(decoded.opaque_ticket.expose(), multibyte_value);

    let escaped_oversized = r"\u0061".repeat(MAX_OPAQUE_TICKET_BYTES + 1);
    let oversized = format!(
        r#"{{"protocolVersion":1,"sessionId":"credential-session","epoch":3,"body":{{"type":"renew_ticket","payload":{{"opaqueTicket":"{escaped_oversized}"}}}}}}"#
    );
    assert!(matches!(
        decode_renew_ticket_frame_from_json_slice(
            oversized.as_bytes(),
            WireLimits {
                max_envelope_bytes: u32::try_from(oversized.len()).unwrap(),
                ..WireLimits::default()
            }
        ),
        Err(ProtocolError::Decode(_))
    ));
}
