use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signer as _, SigningKey};
use serde_json::{json, Value};

use super::*;

const ISSUER: &str = "https://collab.example.com";
const NOW: u64 = 1_800_000_000;
const PRODUCTION_FIXTURE_SIGNATURE: &str = "aeno37t6xdvD-UX4JnBXn4TyV3mh\
    ZY8FWD2dkMtmTCVXwobarKlRDaXIsRxrf1O4MMYh_QggcwOBUUMDqd9hAw";

fn sequence_key(first_byte: u8) -> SigningKey {
    let mut seed = [0_u8; 32];
    for (index, byte) in seed.iter_mut().enumerate() {
        *byte = first_byte + u8::try_from(index).unwrap();
    }
    SigningKey::from_bytes(&seed)
}

fn public_x(first_byte: u8) -> String {
    URL_SAFE_NO_PAD.encode(sequence_key(first_byte).verifying_key().to_bytes())
}

fn key(region: &str, kid: &str, first_byte: u8, activated: i64) -> Value {
    json!({
        "region": region,
        "kid": kid,
        "x": public_x(first_byte),
        "published_at_unix": 1_700_000_000,
        "activated_at_unix": activated,
        "retired_at_unix": 0,
        "not_after_unix": 0,
    })
}

fn production_fixture() -> Value {
    json!({
        "version": 1,
        "generation": 7,
        "issuer": ISSUER,
        "not_before_unix": 1_799_900_000,
        "not_after_unix": 1_800_500_000,
        "required_regions": ["cn", "global"],
        "keys": [
            key("cn", "active_key", 1, 1_700_000_300),
            key("cn", "next_key", 41, 0),
            key("global", "remote_active_key", 81, 1_700_000_300),
            key("global", "remote_next_key", 121, 0),
        ],
        "signature": PRODUCTION_FIXTURE_SIGNATURE,
    })
}

fn test_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[0x5a; 32])
}

fn sign_value(mut value: Value, signing_key: &SigningKey) -> Vec<u8> {
    value["signature"] = json!("");
    let wire: PolicyWire = serde_json::from_value(value.clone()).unwrap();
    let canonical = canonicalize(wire, value["issuer"].as_str().unwrap()).unwrap();
    let unsigned = serde_json::to_vec(&canonical.unsigned).unwrap();
    let mut message = POLICY_DOMAIN.to_vec();
    message.extend_from_slice(&unsigned);
    value["signature"] = json!(URL_SAFE_NO_PAD.encode(signing_key.sign(&message).to_bytes()));
    serde_json::to_vec(&value).unwrap()
}

fn parse_test_policy(value: Value, now: u64) -> Result<CollabUnionPolicy, CollabUnionPolicyError> {
    let signing_key = test_signing_key();
    let body = sign_value(value, &signing_key);
    CollabUnionPolicy::from_json_with_root(
        &body,
        64 * 1024,
        ISSUER,
        now,
        signing_key.verifying_key().to_bytes(),
    )
}

fn policy_with_retired() -> Value {
    let mut value = production_fixture();
    let mut retired = key("cn", "retired_key", 5, 1_700_000_100);
    retired["retired_at_unix"] = json!(1_700_000_200);
    retired["not_after_unix"] = json!(1_800_000_100);
    value["keys"].as_array_mut().unwrap().push(retired);
    value["signature"] = json!("");
    value
}

#[test]
fn verifies_the_frozen_go_production_root_fixture() {
    let body = serde_json::to_vec(&production_fixture()).unwrap();
    let policy = CollabUnionPolicy::from_json(&body, 64 * 1024, ISSUER, NOW).unwrap();
    assert_eq!(policy.generation(), 7);
    assert_eq!(policy.issuer(), ISSUER);
    assert_eq!(policy.key_count(), 4);
    assert_eq!(
        policy.verification_key_at("active_key", NOW),
        Some(sequence_key(1).verifying_key().to_bytes())
    );
    assert_eq!(policy.verification_key_at("next_key", NOW), None);
}

#[test]
fn canonical_sorting_matches_go_for_reordered_wire_arrays() {
    let mut fixture = production_fixture();
    fixture["required_regions"] = json!(["global", "cn"]);
    fixture["keys"].as_array_mut().unwrap().reverse();
    let body = serde_json::to_vec(&fixture).unwrap();
    assert!(CollabUnionPolicy::from_json(&body, 64 * 1024, ISSUER, NOW).is_ok());
}

#[test]
fn next_keys_never_verify_and_retired_overlap_expires_at_not_after() {
    let mut retired = key("cn", "retired", 3, (NOW - 200) as i64);
    retired["retired_at_unix"] = json!(NOW - 100);
    retired["not_after_unix"] = json!(NOW + 50);
    let value = json!({
        "version": 1,
        "generation": 2,
        "issuer": ISSUER,
        "not_before_unix": NOW - 100,
        "not_after_unix": NOW + 300,
        "required_regions": ["cn"],
        "keys": [
            key("cn", "active", 1, (NOW - 200) as i64),
            key("cn", "next", 2, 0),
            retired,
        ],
        "signature": "",
    });
    let policy = parse_test_policy(value, NOW).unwrap();
    assert!(policy.verification_key_at("active", NOW).is_some());
    assert!(policy.verification_key_at("retired", NOW).is_some());
    assert_eq!(policy.verification_key_at("retired", NOW + 50), None);
    assert_eq!(policy.verification_key_at("next", NOW), None);
}

#[test]
fn enforces_generation_monotonicity_and_same_generation_immutability() {
    let mut current = production_fixture();
    current["generation"] = json!(2);
    current["signature"] = json!("");
    let current = parse_test_policy(current, NOW).unwrap();

    let mut older = production_fixture();
    older["generation"] = json!(1);
    older["signature"] = json!("");
    let older = parse_test_policy(older, NOW).unwrap();
    assert_eq!(
        older.ensure_successor_of(&current),
        Err(CollabUnionPolicyError::GenerationRollback)
    );

    let mut rewritten = production_fixture();
    rewritten["generation"] = json!(2);
    rewritten["keys"][0]["published_at_unix"] = json!(1_700_000_001);
    rewritten["signature"] = json!("");
    let rewritten = parse_test_policy(rewritten, NOW).unwrap();
    assert_eq!(
        rewritten.ensure_successor_of(&current),
        Err(CollabUnionPolicyError::GenerationRewrite)
    );

    let mut newer = production_fixture();
    newer["generation"] = json!(3);
    newer["signature"] = json!("");
    assert!(parse_test_policy(newer, NOW)
        .unwrap()
        .ensure_successor_of(&current)
        .is_ok());
}

#[test]
fn rejects_invalid_authority_profile_and_resource_bounds() {
    let valid_body = serde_json::to_vec(&production_fixture()).unwrap();
    assert_eq!(
        CollabUnionPolicy::from_json(&valid_body, valid_body.len() - 1, ISSUER, NOW),
        Err(CollabUnionPolicyError::InvalidBodySize)
    );
    assert_eq!(
        CollabUnionPolicy::from_json(&valid_body, 64 * 1024, "https://other.example", NOW),
        Err(CollabUnionPolicyError::InvalidIssuer)
    );
    assert_eq!(
        CollabUnionPolicy::from_json(&valid_body, 64 * 1024, ISSUER, 1_799_899_999),
        Err(CollabUnionPolicyError::Inactive)
    );
    assert_eq!(
        CollabUnionPolicy::from_json(&valid_body, 64 * 1024, ISSUER, 1_800_500_000),
        Err(CollabUnionPolicyError::Inactive)
    );

    let mut unknown = production_fixture();
    unknown["unexpected"] = json!(true);
    assert_eq!(
        CollabUnionPolicy::from_json(
            &serde_json::to_vec(&unknown).unwrap(),
            64 * 1024,
            ISSUER,
            NOW
        ),
        Err(CollabUnionPolicyError::MalformedJson)
    );

    let mut tampered = production_fixture();
    tampered["generation"] = json!(8);
    assert_eq!(
        CollabUnionPolicy::from_json(
            &serde_json::to_vec(&tampered).unwrap(),
            64 * 1024,
            ISSUER,
            NOW
        ),
        Err(CollabUnionPolicyError::InvalidSignature)
    );
}

#[test]
fn rejects_any_union_key_outside_its_signed_active_time() {
    let mut future_published = production_fixture();
    future_published["keys"][0]["published_at_unix"] = json!(NOW + 1);
    future_published["signature"] = json!("");

    let mut future_activated = production_fixture();
    future_activated["keys"][0]["activated_at_unix"] = json!(NOW + 1);
    future_activated["signature"] = json!("");

    let mut future_retired = policy_with_retired();
    future_retired["keys"][4]["retired_at_unix"] = json!(NOW + 1);
    future_retired["keys"][4]["not_after_unix"] = json!(NOW + 100);

    let mut expired_retired = policy_with_retired();
    expired_retired["keys"][4]["not_after_unix"] = json!(NOW);

    for value in [
        future_published,
        future_activated,
        future_retired,
        expired_retired,
    ] {
        assert_eq!(
            parse_test_policy(value, NOW),
            Err(CollabUnionPolicyError::Inactive)
        );
    }
}

#[test]
fn rejects_incomplete_or_ambiguous_regional_unions() {
    let cases = [
        (
            {
                let mut value = production_fixture();
                value["required_regions"] = json!(["cn", "cn"]);
                value
            },
            CollabUnionPolicyError::InvalidRegions,
        ),
        (
            {
                let mut value = production_fixture();
                value["keys"].as_array_mut().unwrap().remove(1);
                value
            },
            CollabUnionPolicyError::InvalidRotationPhase,
        ),
        (
            {
                let mut value = production_fixture();
                value["keys"][1]["x"] = value["keys"][0]["x"].clone();
                value
            },
            CollabUnionPolicyError::InvalidKeys,
        ),
        (
            {
                let mut value = production_fixture();
                value["keys"][1]["not_after_unix"] = json!(1_900_000_000_i64);
                value
            },
            CollabUnionPolicyError::InvalidKeyLifecycle,
        ),
    ];
    for (value, expected) in cases {
        let body = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            CollabUnionPolicy::from_json(&body, 64 * 1024, ISSUER, NOW),
            Err(expected)
        );
    }
}

#[test]
fn accepts_the_maximum_eight_region_twenty_four_key_union() {
    let mut regions = Vec::new();
    let mut keys = Vec::new();
    for index in 0_u8..8 {
        let region = format!("region-{index}");
        regions.push(region.clone());
        keys.push(key(
            &region,
            &format!("{region}-active"),
            1 + index * 3,
            (NOW - 200) as i64,
        ));
        keys.push(key(&region, &format!("{region}-next"), 2 + index * 3, 0));
        let mut retired = key(
            &region,
            &format!("{region}-retired"),
            3 + index * 3,
            (NOW - 300) as i64,
        );
        retired["retired_at_unix"] = json!(NOW - 100);
        retired["not_after_unix"] = json!(NOW + 200);
        keys.push(retired);
    }
    let value = json!({
        "version": 1,
        "generation": 9,
        "issuer": ISSUER,
        "not_before_unix": NOW - 100,
        "not_after_unix": NOW + 300,
        "required_regions": regions,
        "keys": keys,
        "signature": "",
    });
    assert_eq!(parse_test_policy(value, NOW).unwrap().key_count(), 24);
}
