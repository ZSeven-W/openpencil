#![cfg(unix)]

use std::{
    ffi::OsString,
    io::{Read as _, Write as _},
    net::SocketAddr,
    os::unix::net::UnixListener,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use ed25519_dalek::{Signer as _, SigningKey};
use op_collab_relay_protocol::{LocatorKeyId, RelayRegion};
use serde_json::json;
use sha2::{Digest as _, Sha256};

use super::{
    production_check::{check_production_config_at, parse_expected_policy_sha256},
    ExpectedUnixPeer, LocatorHttpLimits, LocatorServerConfig, ProductionLocatorCheckError,
    ProductionLocatorConfig,
};
use crate::{HSM_SIGN_REQUEST_BYTES, HSM_SIGN_RESPONSE_BYTES};

const POLICY: &[u8] = include_bytes!(
    "../../../op-auth-bridge/tests/fixtures/zseven-sso-go-union-policy-v2-generation-4.json"
);
const POLICY_NOW: u64 = 1_786_259_066;
const KEY_ID: &str = "locator-check-key";

#[test]
fn production_check_uses_policy_mount_and_real_hsm_signature() {
    let result = run_check([0x61; 32], [0x61; 32]);
    assert_eq!(result, Ok(()));
}

#[test]
fn production_check_software_verification_rejects_the_wrong_public_key() {
    let result = run_check([0x61; 32], [0x62; 32]);
    assert_eq!(result, Err(ProductionLocatorCheckError::Signature));
}

#[test]
fn production_check_rejects_a_valid_policy_with_the_wrong_expected_digest() {
    let result = run_check_with_inputs([0x61; 32], [0x61; 32], POLICY, "0".repeat(64), false);
    assert_eq!(result, Err(ProductionLocatorCheckError::Policy));
}

#[test]
fn production_check_rejects_a_rewritten_signature_with_a_matching_digest() {
    let mut policy: serde_json::Value = serde_json::from_slice(POLICY).expect("policy fixture");
    policy["signature"] = json!(
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    let rewritten_policy = serde_json::to_vec(&policy).expect("policy JSON");
    let expected_policy_sha256 = format!("{:x}", Sha256::digest(&rewritten_policy));
    let result = run_check_with_inputs(
        [0x61; 32],
        [0x61; 32],
        &rewritten_policy,
        expected_policy_sha256,
        false,
    );
    assert_eq!(result, Err(ProductionLocatorCheckError::Policy));
}

#[test]
fn production_check_errors_are_safe_categories() {
    assert_eq!(
        ProductionLocatorCheckError::Configuration.to_string(),
        "configuration"
    );
    assert_eq!(
        ProductionLocatorCheckError::Signature.to_string(),
        "HSM signature verification"
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
            Err(ProductionLocatorCheckError::Configuration)
        );
    }
    assert_eq!(
        parse_expected_policy_sha256(Some(OsString::from("a".repeat(64)))),
        Ok("a".repeat(64))
    );
}

#[test]
fn expected_policy_digest_parser_rejects_non_unicode() {
    use std::os::unix::ffi::OsStringExt as _;

    assert_eq!(
        parse_expected_policy_sha256(Some(OsString::from_vec(vec![0xff; 64]))),
        Err(ProductionLocatorCheckError::Configuration)
    );
}

fn run_check(
    signer_seed: [u8; 32],
    published_seed: [u8; 32],
) -> Result<(), ProductionLocatorCheckError> {
    run_check_with_inputs(signer_seed, published_seed, POLICY, policy_sha256(), true)
}

fn run_check_with_inputs(
    signer_seed: [u8; 32],
    published_seed: [u8; 32],
    policy_body: &[u8],
    expected_policy_sha256: String,
    exercise_hsm: bool,
) -> Result<(), ProductionLocatorCheckError> {
    let directory = workspace_tempdir();
    let policy_path = directory.path().join("policy.json");
    let public_keys_path = directory.path().join("locator-public-keys.json");
    let socket_path = directory.path().join("signer.sock");
    write_public_fixture(&policy_path, policy_body);
    write_public_keys(&public_keys_path, published_seed);

    let hsm = exercise_hsm.then(|| {
        let listener = UnixListener::bind(&socket_path).expect("HSM socket");
        thread::spawn(move || serve_one_signature(listener, signer_seed))
    });
    let config = ProductionLocatorConfig {
        server: LocatorServerConfig::new(
            "127.0.0.1:8092".parse::<SocketAddr>().expect("listen"),
            LocatorHttpLimits::default(),
        )
        .expect("server config"),
        home_region: RelayRegion::Cn,
        ticket_policy_file: policy_path,
        policy_max_age_seconds: std::num::NonZeroU64::new(60).expect("non-zero"),
        hsm_socket: socket_path,
        hsm_key_id: LocatorKeyId::new(KEY_ID).expect("key id"),
        hsm_peer: current_peer(),
        hsm_timeout: Duration::from_secs(1),
    };
    let result = check_production_config_at(
        &config,
        &public_keys_path,
        POLICY_NOW,
        &expected_policy_sha256,
    );
    if let Some(hsm) = hsm {
        hsm.join().expect("HSM thread");
    }
    result
}

fn policy_sha256() -> String {
    format!("{:x}", Sha256::digest(POLICY))
}

fn serve_one_signature(listener: UnixListener, signer_seed: [u8; 32]) {
    listener
        .set_nonblocking(true)
        .expect("HSM listener non-blocking");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return,
        }
    };
    stream.set_nonblocking(false).expect("HSM stream blocking");
    let mut request = Vec::new();
    stream.read_to_end(&mut request).expect("HSM request");
    assert_eq!(request.len(), HSM_SIGN_REQUEST_BYTES);
    assert_eq!(&request[..4], b"OPLS");
    assert_eq!(request[4], 1);
    assert_eq!(request[5], 1);
    let key_length = usize::from(request[6]);
    assert_eq!(&request[7..7 + key_length], KEY_ID.as_bytes());
    let canonical = &request[71..];
    assert_eq!(canonical.len(), 268);
    assert_eq!(canonical[0], 1);
    assert_eq!(canonical[1], RelayRegion::Cn as u8);
    assert_eq!(&canonical[2..18], &[0x51; 16]);
    assert_eq!(&canonical[18..26], &1_u64.to_be_bytes());
    assert_eq!(&canonical[26..58], &[0x52; 32]);
    assert_eq!(&canonical[187..195], &1_u64.to_be_bytes());
    assert_eq!(&canonical[195..203], &2_u64.to_be_bytes());

    let signature = SigningKey::from_bytes(&signer_seed)
        .sign(canonical)
        .to_bytes();
    let mut response = [0_u8; HSM_SIGN_RESPONSE_BYTES];
    response[..4].copy_from_slice(b"OPLR");
    response[4] = 1;
    response[5] = 0;
    response[6..].copy_from_slice(&signature);
    stream.write_all(&response).expect("HSM response");
}

fn write_public_keys(path: &Path, seed: [u8; 32]) {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    let verifying_key = SigningKey::from_bytes(&seed).verifying_key();
    let body = json!({
        "version": 1,
        "keys": [{
            "kid": KEY_ID,
            "public_key_ed25519": URL_SAFE_NO_PAD.encode(verifying_key.as_bytes()),
        }],
    });
    write_public_fixture(path, &serde_json::to_vec(&body).expect("public keys JSON"));
}

fn write_public_fixture(path: &Path, body: &[u8]) {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::write(path, body).expect("public fixture file");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644))
        .expect("public fixture permissions");
}

fn current_peer() -> ExpectedUnixPeer {
    ExpectedUnixPeer {
        uid: unsafe { libc::geteuid() },
        gid: unsafe { libc::getegid() },
    }
}

fn workspace_tempdir() -> tempfile::TempDir {
    let system_temp = std::env::temp_dir()
        .canonicalize()
        .expect("canonical system temp directory");
    tempfile::tempdir_in(system_temp).expect("system temp directory")
}
