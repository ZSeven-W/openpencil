use super::*;
use crate::hosted_access_wire::decode;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};

const NOW: u64 = 1_800_000_000;
const SUBJECT: &str = "12345678-1234-4234-8234-123456789abc";

fn fixture() -> Value {
    json!({
        "version": 1,
        "token": format!("opllm_{}", URL_SAFE_NO_PAD.encode([7; 32])),
        "token_id": "0123456789abcdef0123456789abcdef",
        "token_type": "Bearer",
        "issuer": "https://sso.example.test",
        "subject": SUBJECT,
        "audience": "llm-gateway",
        "scopes": ["llm:models", "llm:invoke"],
        "issued_at_unix": NOW,
        "expires_at_unix": NOW + 300,
        "gateway_base_url": "https://gateway.example.test/v1",
        "product": "openpencil",
    })
}

fn decode_value(value: &Value) -> Result<HostedAccess, HostedAccessError> {
    decode(&serde_json::to_vec(value).unwrap(), NOW)
}

#[test]
fn validates_access_without_exposing_metadata_or_secret_in_debug() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<HostedAccess>();
    let fixture = fixture();
    let access = decode_value(&fixture).unwrap();
    assert_eq!(access.subject(), SUBJECT);
    assert_eq!(access.issuer(), "https://sso.example.test");
    assert_eq!(access.gateway_base_url(), "https://gateway.example.test/v1");
    assert_eq!(access.expires_at_unix(), NOW + 300);
    assert_eq!(access.bearer(), fixture["token"].as_str().unwrap());
    assert_eq!(
        format!("{access:?}"),
        "HostedAccess { credential: \"[REDACTED]\" }"
    );
    assert!(!format!("{:?}", HostedAccessPoll::Ready(access)).contains("opllm_"));
}

#[test]
fn rejects_duplicate_unknown_missing_and_escaped_fields() {
    let json = fixture().to_string();
    for prefix in [
        "\"version\":1,",
        "\"unknown\":true,",
        "\"tok\\u0065n\":\"secret\",",
        "\"token\":\"duplicate-secret\",",
    ] {
        let bytes = format!("{{{prefix}{}", &json[1..]);
        assert!(matches!(
            decode(bytes.as_bytes(), NOW),
            Err(HostedAccessError::InvalidPayload)
        ));
    }
    for key in fixture().as_object().unwrap().keys() {
        let mut value = fixture();
        value.as_object_mut().unwrap().remove(key);
        assert!(matches!(
            decode_value(&value),
            Err(HostedAccessError::InvalidPayload)
        ));
    }
    for input in [b"[]".as_slice(), b"{}{}", b"{", &[255]] {
        assert!(matches!(
            decode(input, NOW),
            Err(HostedAccessError::InvalidPayload)
        ));
    }
}

#[test]
fn enforces_input_size_before_parsing() {
    let json = fixture().to_string();
    let mut bytes = json.into_bytes();
    bytes.resize(MAX_HOSTED_ACCESS_BYTES, b' ');
    assert!(decode(&bytes, NOW).is_ok());
    bytes.push(b' ');
    assert!(matches!(
        decode(&bytes, NOW),
        Err(HostedAccessError::InvalidPayload)
    ));
    assert!(matches!(
        HostedAccess::from_provider_json(bytes),
        Err(HostedAccessError::InvalidPayload)
    ));
}

#[test]
fn rejects_wrong_identity_token_domains_and_scope() {
    for (key, value) in [
        ("version", json!(2)),
        ("token", json!("oppt_device-token")),
        ("token", json!("Bearer secret")),
        (
            "token",
            json!(format!("opllm_{}=", URL_SAFE_NO_PAD.encode([7; 32]))),
        ),
        ("token", json!(format!("opllm_{}", "A".repeat(42) + "B"))),
        ("token_id", json!("ABCDEF0123456789abcdef0123456789ab")),
        ("token_id", json!("short")),
        ("token_type", json!("bearer")),
        ("subject", json!("00000000-0000-0000-0000-000000000000")),
        ("subject", json!(SUBJECT.to_uppercase())),
        ("subject", json!("device-session-id")),
        ("audience", json!("collab")),
        ("scopes", json!(["llm:invoke", "llm:models"])),
        ("scopes", json!(["llm:models", "llm:models"])),
        ("scopes", json!(["llm:models", "llm:invoke", "admin"])),
        ("scopes", json!(["llm:models"])),
    ] {
        let mut fixture = fixture();
        fixture[key] = value;
        assert!(
            matches!(
                decode_value(&fixture),
                Err(HostedAccessError::InvalidPayload)
            ),
            "{key}"
        );
    }
}

#[test]
fn rejects_noncanonical_or_unsafe_endpoints() {
    for endpoint in [
        "http://gateway.example.test/v1",
        "https://u:p@gateway.example.test/v1",
        "https://gateway.example.test/v1?token=secret",
        "https://gateway.example.test/v1#secret",
        "https://gateway.example.test/V1",
        "https://gateway.example.test/v1/",
        "https://gateway.example.test/a/../v1",
        "https://gateway.example.test/%76%31",
        "https://gateway.example.test//v1",
        "https://GATEWAY.example.test/v1",
        "https://gateway.example.test:443/v1",
        "https://gateway.example.test/a%2fb/v1",
    ] {
        let mut fixture = fixture();
        fixture["gateway_base_url"] = json!(endpoint);
        assert!(
            matches!(
                decode_value(&fixture),
                Err(HostedAccessError::InvalidPayload)
            ),
            "{endpoint}"
        );
    }
    for issuer in [
        "http://sso.example.test",
        "https://sso.example.test/",
        "https://sso.example.test/path",
        "https://user@sso.example.test",
        "https://sso.example.test?x=1",
        "https://sso.example.test#x",
    ] {
        let mut fixture = fixture();
        fixture["issuer"] = json!(issuer);
        assert!(matches!(
            decode_value(&fixture),
            Err(HostedAccessError::InvalidPayload)
        ));
    }
    let mut fixture = fixture();
    fixture["gateway_base_url"] = json!("https://gateway.example.test:8443/llm-gateway/v1");
    assert!(decode_value(&fixture).is_ok());
}

#[test]
fn rejects_expired_future_and_excessive_lifetime_but_accepts_cached_receipt() {
    for (issued, expires, error) in [
        (NOW - 300, NOW, HostedAccessError::Expired),
        (NOW + 31, NOW + 331, HostedAccessError::InvalidPayload),
        (NOW, NOW + 301, HostedAccessError::InvalidPayload),
        (NOW, NOW + 59, HostedAccessError::InvalidPayload),
        (NOW, NOW - 1, HostedAccessError::InvalidPayload),
        (0, 300, HostedAccessError::InvalidPayload),
        (u64::MAX - 300, u64::MAX, HostedAccessError::InvalidPayload),
    ] {
        let mut fixture = fixture();
        fixture["issued_at_unix"] = json!(issued);
        fixture["expires_at_unix"] = json!(expires);
        assert_eq!(decode_value(&fixture).unwrap_err(), error);
    }
    for (issued, expires) in [
        (NOW, NOW + 60),
        (NOW - 290, NOW + 10),
        (NOW + 30, NOW + 330),
    ] {
        let mut fixture = fixture();
        fixture["issued_at_unix"] = json!(issued);
        fixture["expires_at_unix"] = json!(expires);
        assert!(decode_value(&fixture).is_ok());
    }
}

#[test]
fn unavailable_capability_does_not_replace_base_authentication() {
    assert_eq!(HostedAccessRequestId::new(0), None);
    let id = HostedAccessRequestId::new(9).unwrap();
    assert_eq!(id.get(), 9);
    let provider = UnavailableHostedAccessProvider;
    assert!(!provider.available());
    assert_eq!(provider.session_epoch(), 0);
    assert_eq!(provider.begin(), Err(HostedAccessError::Unavailable));
    assert!(matches!(
        provider.poll(id),
        Err(HostedAccessError::Unavailable)
    ));
    provider.cancel(id);
    #[cfg(not(op_auth_hosted_access_prebuilt))]
    assert!(!crate::hosted_access_provider().available());
    assert_eq!(crate::REQUIRED_ABI_VERSION, 1);
    assert_eq!(crate::HOSTED_ACCESS_ABI_VERSION, 4);
}

#[test]
#[cfg(all(op_auth_development_prebuilt, op_auth_hosted_access_prebuilt))]
fn development_v4_exports_fail_closed_before_runtime_initialization() {
    let provider = crate::hosted_access_provider();
    assert!(crate::available());
    assert!(provider.available());
    assert_eq!(provider.session_epoch(), 0);
    assert_eq!(provider.begin(), Err(HostedAccessError::Internal));
    let id = HostedAccessRequestId::new(1).unwrap();
    assert!(matches!(
        provider.poll(id),
        Err(HostedAccessError::RequestNotFound)
    ));
    provider.cancel(id);
}

#[test]
fn legacy_or_other_product_access_cannot_bypass_product_deletion() {
    let mut payload = fixture();
    payload.as_object_mut().unwrap().remove("product");
    assert!(decode(payload.to_string().as_bytes(), NOW).is_err());
    payload["product"] = json!("another-product");
    assert!(decode(payload.to_string().as_bytes(), NOW).is_err());
}
