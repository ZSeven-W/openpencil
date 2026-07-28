#![cfg(feature = "test-issuer")]

use std::time::Instant;

use op_auth_bridge::{
    CollabJwksCacheLimits, CollabJwksError, CollabTicketVerifier, CollabUnionPolicyError,
    CollabVerifyError, StaticTestJwksFetcher, TestCollabIssuer, TestCollabTicketSpec,
    TEST_COLLAB_ISSUER,
};

const NOW: u64 = 2_000_000_000;
const CHANNEL_BINDING: [u8; 32] = [0x42; 32];

fn verifier(issuer: &TestCollabIssuer) -> CollabTicketVerifier<StaticTestJwksFetcher> {
    CollabTicketVerifier::new(
        TestCollabIssuer::verifier_config().unwrap(),
        StaticTestJwksFetcher::new(issuer.jwks_json().unwrap(), 300),
        CollabJwksCacheLimits::default(),
    )
    .unwrap()
}

#[test]
fn public_fixtures_are_deterministic_and_cover_rotation_overlap() {
    let spec = TestCollabTicketSpec::valid_at(NOW, CHANNEL_BINDING);
    let initial = TestCollabIssuer::initial();
    let first = initial.issue(&spec).unwrap();
    let second = initial.issue(&spec).unwrap();
    assert_eq!(first.expose(), second.expose());

    let rotated = TestCollabIssuer::rotated();
    let rotated_ticket = rotated.issue(&spec).unwrap();
    let verifier = verifier(&rotated);
    assert_eq!(
        verifier
            .verify_at(first.expose(), &CHANNEL_BINDING, NOW, Instant::now())
            .unwrap()
            .issuer(),
        TEST_COLLAB_ISSUER
    );
    assert_eq!(
        verifier
            .verify_at(
                rotated_ticket.expose(),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            )
            .unwrap()
            .issuer(),
        TEST_COLLAB_ISSUER
    );
}

#[test]
fn retired_rotation_key_and_test_issuer_fail_closed() {
    let spec = TestCollabTicketSpec::valid_at(NOW, CHANNEL_BINDING);
    let initial_ticket = TestCollabIssuer::initial().issue(&spec).unwrap();
    let retired = TestCollabIssuer::retired_a();
    assert_eq!(
        verifier(&retired).verify_at(
            initial_ticket.expose(),
            &CHANNEL_BINDING,
            NOW,
            Instant::now()
        ),
        Err(CollabVerifyError::Jwks(CollabJwksError::UnknownKey))
    );

    let test_issuer = TestCollabIssuer::initial();
    let production_verifier = CollabTicketVerifier::production(StaticTestJwksFetcher::new(
        test_issuer.jwks_json().unwrap(),
        300,
    ))
    .unwrap();
    assert_eq!(
        production_verifier.verify_at(
            initial_ticket.expose(),
            &CHANNEL_BINDING,
            NOW,
            Instant::now()
        ),
        Err(CollabVerifyError::Jwks(CollabJwksError::Policy(
            CollabUnionPolicyError::MalformedJson
        )))
    );
}
