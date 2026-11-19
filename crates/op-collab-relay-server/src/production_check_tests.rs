use std::{ffi::OsString, num::NonZeroU64, path::Path};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::SigningKey;
use op_collab_relay_protocol::RelayRegion;
use serde_json::json;
use sha2::{Digest as _, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::{
    production::{check_production_config_at, parse_expected_policy_sha256},
    ProductionRelayAuthConfig, ProductionRelayCheckError,
};

const POLICY: &[u8] = include_bytes!(
    "../../op-auth-bridge/tests/fixtures/zseven-sso-go-union-policy-v2-generation-4.json"
);
const POLICY_NOW: u64 = 1_786_259_066;

#[test]
fn production_check_parses_policy_locator_and_x25519_mounts() {
    let fixture = Fixture::new();
    let config = fixture.config();
    assert_eq!(
        check_production_config_at(&config, POLICY_NOW, &policy_sha256()),
        Ok(())
    );
}

#[test]
fn production_check_rejects_a_policy_with_a_rewritten_signature() {
    let fixture = Fixture::new();
    let mut policy: serde_json::Value = serde_json::from_slice(POLICY).expect("policy fixture");
    policy["signature"] = json!(
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    let rewritten_policy = serde_json::to_vec(&policy).expect("policy JSON");
    write_public_fixture(
        &fixture.directory.path().join("policy.json"),
        &rewritten_policy,
    );
    assert_eq!(
        check_production_config_at(
            &fixture.config(),
            POLICY_NOW,
            &format!("{:x}", Sha256::digest(&rewritten_policy)),
        ),
        Err(ProductionRelayCheckError::Policy)
    );
}

#[test]
fn production_check_rejects_a_valid_policy_with_the_wrong_expected_digest() {
    let fixture = Fixture::new();
    assert_eq!(
        check_production_config_at(&fixture.config(), POLICY_NOW, &"0".repeat(64)),
        Err(ProductionRelayCheckError::Policy)
    );
}

#[test]
fn production_check_errors_are_safe_categories() {
    assert_eq!(
        ProductionRelayCheckError::Configuration.to_string(),
        "configuration"
    );
    assert_eq!(
        ProductionRelayCheckError::Policy.to_string(),
        "signed policy"
    );
    assert_eq!(
        ProductionRelayCheckError::RelayX25519Keys.to_string(),
        "relay proof keys"
    );
}

#[test]
fn expected_policy_digest_parser_is_strict_and_fail_closed() {
    for value in [
        None,
        Some(OsString::from("0".repeat(63))),
        Some(OsString::from("0".repeat(65))),
        Some(OsString::from("A".repeat(64))),
        Some(OsString::from("g".repeat(64))),
    ] {
        assert_eq!(
            parse_expected_policy_sha256(value),
            Err(ProductionRelayCheckError::Configuration)
        );
    }
    assert_eq!(
        parse_expected_policy_sha256(Some(OsString::from("a".repeat(64)))),
        Ok("a".repeat(64))
    );
}

#[cfg(unix)]
#[test]
fn expected_policy_digest_parser_rejects_non_unicode() {
    use std::os::unix::ffi::OsStringExt as _;

    assert_eq!(
        parse_expected_policy_sha256(Some(OsString::from_vec(vec![0xff; 64]))),
        Err(ProductionRelayCheckError::Configuration)
    );
}

struct Fixture {
    directory: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary directory");
        write_public_fixture(&directory.path().join("policy.json"), POLICY);
        write_locator_keys(directory.path());
        write_x25519_keys(directory.path());
        Self { directory }
    }

    fn config(&self) -> ProductionRelayAuthConfig {
        ProductionRelayAuthConfig::new(
            RelayRegion::Cn,
            self.directory.path().join("policy.json"),
            self.directory.path().join("locator-keys.json"),
            Some(self.directory.path().join("x25519-keys.json")),
            NonZeroU64::new(60).expect("non-zero"),
            false,
        )
        .expect("production config")
    }
}

fn write_locator_keys(directory: &Path) {
    let key = SigningKey::from_bytes(&[0x31; 32]);
    let body = json!({
        "version": 1,
        "keys": [{
            "kid": "locator-check-key",
            "public_key_ed25519": URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
        }],
    });
    write_public_fixture(
        &directory.join("locator-keys.json"),
        &serde_json::to_vec(&body).expect("locator keys JSON"),
    );
}

fn write_public_fixture(path: &Path, body: &[u8]) {
    std::fs::write(path, body).expect("public fixture file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644))
            .expect("public fixture permissions");
    }
}

fn write_x25519_keys(directory: &Path) {
    let secret = StaticSecret::from([0x41; 32]);
    let public = PublicKey::from(&secret);
    let body = json!({
        "version": 1,
        "active_kid": "relay-check-key",
        "keys": [{
            "kid": "relay-check-key",
            "private_key_x25519": URL_SAFE_NO_PAD.encode(secret.to_bytes()),
            "public_key_x25519": URL_SAFE_NO_PAD.encode(public.as_bytes()),
        }],
    });
    let path = directory.join("x25519-keys.json");
    std::fs::write(&path, serde_json::to_vec(&body).expect("X25519 keys JSON"))
        .expect("X25519 keys file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .expect("private key permissions");
    }
}

fn policy_sha256() -> String {
    format!("{:x}", Sha256::digest(POLICY))
}
