# Mutation cases for check-collab-security-boundaries.test.sh.
# Sourced after the fixture and assertion helpers have been initialized.

write_collab_security_workflow_fixture() {
    cat > "$fixture_root/.github/workflows/collab-security.yml" <<'EOF'
pull_request:
  paths:
    - '.dockerignore'
    - '.gitignore'
    - 'crates/op-collab-smoke/**'
    - 'crates/op-collab-relay-protocol/**'
    - 'crates/op-collab-relay-client/**'
    - 'crates/op-collab-relay-server/**'
    - 'crates/op-collab-relay-control-plane/**'
    - 'crates/op-collab-policy-file/**'
    - 'crates/op-collab-relay-locator-hsm/**'
    - 'crates/op-collab-relay-locator-server/**'
    - 'crates/op-util/**'
    - 'crates/op-editor-core/**'
    - 'crates/op-editor-host-core/**'
    - 'crates/op-editor-ui/**'
    - 'crates/op-host-native/**'
    - 'crates/op-host-desktop/**'
    - 'crates/op-chat-agent/src/provider_dial.rs'
    - 'crates/op-host-services/**'
    - 'crates/op-i18n/**'
    - 'deploy/collab-relay/**'
    - 'deploy/collab-relay-edge/**'
    - 'deploy/collab-relay-locator/**'
    - 'deploy/collab-relay-locator-hsm/**'
    - 'deploy/collab-relay-locator-edge/**'
    - 'tools/check-collab-security-boundaries-cases.sh'
    - 'tools/check-collab-deployment-boundaries.sh'
    - 'tools/check-op-auth-prebuilt.sh'
    - 'tools/check-op-auth-prebuilt.test.sh'
    - 'tools/package-op-auth-prebuilt.sh'
push:
  paths:
    - '.dockerignore'
    - '.gitignore'
    - 'crates/op-collab-smoke/**'
    - 'crates/op-collab-relay-protocol/**'
    - 'crates/op-collab-relay-client/**'
    - 'crates/op-collab-relay-server/**'
    - 'crates/op-collab-relay-control-plane/**'
    - 'crates/op-collab-policy-file/**'
    - 'crates/op-collab-relay-locator-hsm/**'
    - 'crates/op-collab-relay-locator-server/**'
    - 'crates/op-util/**'
    - 'crates/op-editor-core/**'
    - 'crates/op-editor-host-core/**'
    - 'crates/op-editor-ui/**'
    - 'crates/op-host-native/**'
    - 'crates/op-host-desktop/**'
    - 'crates/op-chat-agent/src/provider_dial.rs'
    - 'crates/op-host-services/**'
    - 'crates/op-i18n/**'
    - 'deploy/collab-relay/**'
    - 'deploy/collab-relay-edge/**'
    - 'deploy/collab-relay-locator/**'
    - 'deploy/collab-relay-locator-hsm/**'
    - 'deploy/collab-relay-locator-edge/**'
    - 'tools/check-collab-security-boundaries-cases.sh'
    - 'tools/check-collab-deployment-boundaries.sh'
    - 'tools/check-op-auth-prebuilt.sh'
    - 'tools/check-op-auth-prebuilt.test.sh'
    - 'tools/package-op-auth-prebuilt.sh'
steps:
  - run: bash tools/check-op-auth-prebuilt.sh
  - run: bash tools/check-op-auth-prebuilt.test.sh
  - run: bash -n tools/package-op-auth-prebuilt.sh
  - run: cargo test --locked -p op-auth-bridge --test prebuilt_provenance
  - run: cargo test --locked -p op-collab-transport
  - run: cargo test --locked -p op-collab-transport config::tests
  - run: cargo test --locked -p op-collab-transport frame::tests
  - run: cargo test --locked -p op-collab-relay-locator-hsm
  - run: |
      docker build --target test \
        -f deploy/collab-relay-locator-hsm/Dockerfile .
  - run: bash deploy/collab-relay-edge/validate.sh
  - run: bash deploy/collab-relay-locator/validate.sh
  - run: bash deploy/collab-relay-locator-edge/validate.sh
EOF
}

new_fixture baseline
expect_pass "accepts the minimal safe collaboration boundary"

new_fixture wasm-native-dependency
: > "$fixture_root/.fake-wasm-forbidden"
expect_failure "rejects native dependencies in the wasm closure" \
    "WASM boundary includes native/auth dependencies"

new_fixture credential-clone-assertion-removed
sed '/assert_not_impl_any!(OpaqueTicket: Clone);/d' \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs" \
    > "$fixture_root/crates/op-collab/tests/credential_ownership.rs.next"
mv \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs.next" \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs"
expect_failure "requires compile-time non-Clone credential assertions" \
    "credential-bearing protocol type must remain non-Clone"

new_fixture dedicated-ticket-codec-removed
: > "$fixture_root/crates/op-collab/src/error.rs"
expect_failure "requires the dedicated credential codec failure" \
    "dedicated credential codec failure"

new_fixture credential-preflight-moved-after-value
awk '
    index($0, "    declared_kind_rejecting_renew_ticket(bytes)?;") == 1 {
        held = $0
        next
    }
    held != "" && index($0, "    let mut value = decode_json_value(bytes, limits)?;") == 1 {
        print
        print held
        held = ""
        next
    }
    { print }
' \
    "$fixture_root/crates/op-collab/src/codec.rs" \
    > "$fixture_root/crates/op-collab/src/codec.rs.next"
mv \
    "$fixture_root/crates/op-collab/src/codec.rs.next" \
    "$fixture_root/crates/op-collab/src/codec.rs"
expect_failure "requires credential classification before generic Value decoding" \
    "generic credential discriminator must run before JSON Value decoding"

new_fixture inbound-direction-budget-moved-after-discriminator
awk '
    index($0, "    enforce_inbound_envelope_limit(inbound_direction, bytes.len(), limits)?;") == 1 {
        held = $0
        next
    }
    held != "" && index($0, "    declared_kind_rejecting_renew_ticket(bytes)?;") == 1 {
        print
        print held
        held = ""
        next
    }
    { print }
' \
    "$fixture_root/crates/op-collab/src/codec.rs" \
    > "$fixture_root/crates/op-collab/src/codec.rs.next"
mv \
    "$fixture_root/crates/op-collab/src/codec.rs.next" \
    "$fixture_root/crates/op-collab/src/codec.rs"
expect_failure "requires trusted direction budgeting before wire discrimination" \
    "trusted per-direction inbound envelope limit must run before discriminator and JSON Value decoding"

new_fixture dedicated-ticket-zeroizing-decoder-removed
sed '/Zeroizing::new(String::with_capacity/d' \
    "$fixture_root/crates/op-collab/src/ticket_json.rs" \
    > "$fixture_root/crates/op-collab/src/ticket_json.rs.next"
mv \
    "$fixture_root/crates/op-collab/src/ticket_json.rs.next" \
    "$fixture_root/crates/op-collab/src/ticket_json.rs"
expect_failure "requires direct zeroizing ticket string decoding" \
    "direct zeroizing ticket string decoder"

new_fixture dedicated-ticket-ordinary-string-deserializer
printf '%s\n' \
    'fn bad() { let _ = String::deserialize(deserializer); }' \
    >> "$fixture_root/crates/op-collab/src/ticket_json.rs"
expect_failure "rejects ordinary String deserialization in the ticket decoder" \
    "dedicated ticket decoder must not materialize ordinary strings or Values"

new_fixture opaque-ticket-generic-string-deserializer
printf '%s\n' \
    'fn bad() { let _ = String::deserialize(deserializer); }' \
    >> "$fixture_root/crates/op-collab/src/protocol.rs"
expect_failure "rejects ordinary String deserialization in OpaqueTicket" \
    "OpaqueTicket must not deserialize through an ordinary String"

new_fixture generic-renewal-deserialize-assertion-removed
sed '/assert_not_impl_any!(RenewTicket: serde::de::DeserializeOwned);/d' \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs" \
    > "$fixture_root/crates/op-collab/tests/credential_ownership.rs.next"
mv \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs.next" \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs"
expect_failure "requires the generic renewal Deserialize compile-time boundary" \
    "credential-bearing protocol type must not implement generic Deserialize"

new_fixture derived-collab-message-deserializer
printf '%s\n' \
    '#[derive(PartialEq, Serialize, Deserialize)]' \
    >> "$fixture_root/crates/op-collab/src/protocol.rs"
expect_failure "rejects derived adjacent-tag CollabMessage deserialization" \
    "CollabMessage must not use derived Deserialize"

new_fixture direct-serde-renewal-serialization-regression-removed
sed '/direct_serde_renewal_serialization_is_fail_closed/d' \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs" \
    > "$fixture_root/crates/op-collab/tests/credential_ownership.rs.next"
mv \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs.next" \
    "$fixture_root/crates/op-collab/tests/credential_ownership.rs"
expect_failure "requires the direct serde renewal serialization regression" \
    "direct serde credential serialization rejection test"

new_fixture mislabeled-renewal-regression-removed
: > "$fixture_root/crates/op-collab-transport/src/frame.rs"
expect_failure "requires the mislabeled renewal transport regression" \
    "mislabeled credential transport regression test"

new_fixture credential-transport-workflow-test-removed
sed '/cargo test --locked -p op-collab-transport frame::tests/d' \
    "$fixture_root/.github/workflows/collab-security.yml" \
    > "$fixture_root/.github/workflows/collab-security.yml.next"
mv \
    "$fixture_root/.github/workflows/collab-security.yml.next" \
    "$fixture_root/.github/workflows/collab-security.yml"
expect_failure "requires the credential transport codec workflow test" \
    "credential transport codec workflow test"

new_fixture complete-transport-workflow-test-removed
sed '/cargo test --locked -p op-collab-transport$/d' \
    "$fixture_root/.github/workflows/collab-security.yml" \
    > "$fixture_root/.github/workflows/collab-security.yml.next"
mv \
    "$fixture_root/.github/workflows/collab-security.yml.next" \
    "$fixture_root/.github/workflows/collab-security.yml"
expect_failure "requires the complete transport resource-limit test suite" \
    "complete transport resource-limit workflow test"

new_fixture locator-hsm-workflow-tests-removed
sed \
    -e '/cargo test --locked -p op-collab-relay-locator-hsm/d' \
    -e '/docker build --target test/d' \
    "$fixture_root/.github/workflows/collab-security.yml" \
    > "$fixture_root/.github/workflows/collab-security.yml.next"
mv \
    "$fixture_root/.github/workflows/collab-security.yml.next" \
    "$fixture_root/.github/workflows/collab-security.yml"
expect_failure "requires locator HSM unit and real SoftHSM workflow tests" \
    "locator HSM crate workflow test"

new_fixture locator-hsm-soft-token-target-removed
sed 's/docker build --target test/docker build/' \
    "$fixture_root/.github/workflows/collab-security.yml" \
    > "$fixture_root/.github/workflows/collab-security.yml.next"
mv \
    "$fixture_root/.github/workflows/collab-security.yml.next" \
    "$fixture_root/.github/workflows/collab-security.yml"
expect_failure "requires the real SoftHSM Docker test stage" \
    "real SoftHSM workflow test target"

new_fixture locator-hsm-production-seed
printf '%s\n' \
    'const PRODUCTION_SIGNING_SEED: [u8; 32] = [9; 32];' \
    >> "$fixture_root/crates/op-collab-relay-locator-hsm/src/lib.rs"
expect_failure "scans the locator HSM crate for deterministic production keys" \
    "deterministic signing/key seed leaked"

new_fixture desktop-renewal-vec-copy
printf '%s\n' \
    'fn bad(ticket: Ticket) { let _ = ticket.expose().as_bytes().to_vec(); }' \
    >> "$fixture_root/crates/op-collab-host/src/runtime/types.rs"
expect_failure "rejects ordinary Vec copies in desktop renewal commands" \
    "desktop renewal commands must move OpaqueTicket"

new_fixture non-mit-crate
cat > "$fixture_root/crates/op-collab-transport/Cargo.toml" <<'EOF'
[package]
name = "op-collab-transport"
version = "0.0.0"
license = "Apache-2.0"
EOF
expect_failure "rejects a non-MIT collaboration crate" \
    "must inherit or declare the MIT license"

new_fixture deterministic-production-seed
printf '%s\n' \
    'const PRODUCTION_SIGNING_SEED: [u8; 32] = [9; 32];' \
    >> "$fixture_root/crates/op-collab/src/protocol.rs"
expect_failure "rejects deterministic key material in production source" \
    "deterministic signing/key seed leaked"

new_fixture deterministic-production-seed-after-test-module
printf '%s\n' \
    'const PRODUCTION_SIGNING_SEED: [u8; 32] = [9; 32];' \
    >> "$fixture_root/crates/op-auth-bridge/src/collab_verifier.rs"
expect_failure "scans production items after an inline cfg(test) module" \
    "deterministic signing/key seed leaked"

new_fixture deterministic-external-test-without-cfg
sed '/#!\[cfg(test)\]/d' \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs" \
    > "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs.next"
mv \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs.next" \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs"
expect_failure "requires an explicit cfg(test) boundary for external unit tests" \
    "deterministic signing/key seed leaked"

new_fixture deterministic-path-test-without-parent-cfg
sed '/#\[cfg(test)\]/d' \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache.rs" \
    > "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache.rs.next"
mv \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache.rs.next" \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache.rs"
expect_failure "requires cfg(test) on path-based external unit-test modules" \
    "deterministic signing/key seed leaked"

new_fixture production-root-fixture-regression-removed
sed '/verifies_the_frozen_go_production_root_fixture/d' \
    "$fixture_root/crates/op-auth-bridge/src/collab_union_policy_tests.rs" \
    > "$fixture_root/crates/op-auth-bridge/src/collab_union_policy_tests.rs.next"
mv \
    "$fixture_root/crates/op-auth-bridge/src/collab_union_policy_tests.rs.next" \
    "$fixture_root/crates/op-auth-bridge/src/collab_union_policy_tests.rs"
expect_failure "requires the split production root fixture regression" \
    "production trust-root fixture regression test"

new_fixture production-policy-fail-closed-regression-removed
sed '/production_signed_policy_path_never_falls_back_to_raw_jwks/d' \
    "$fixture_root/crates/op-auth-bridge/src/collab_verifier.rs" \
    > "$fixture_root/crates/op-auth-bridge/src/collab_verifier.rs.next"
mv \
    "$fixture_root/crates/op-auth-bridge/src/collab_verifier.rs.next" \
    "$fixture_root/crates/op-auth-bridge/src/collab_verifier.rs"
expect_failure "requires the split production policy fail-closed regression" \
    "production/test issuer isolation regression test"

new_fixture sensitive-key-file
: > "$fixture_root/crates/op-collab-transport/peer.key"
expect_failure "rejects key-shaped repository fixtures" \
    "sensitive key/token-shaped files are forbidden"

new_fixture sealed-relay-key-json
: > "$fixture_root/deploy/collab-relay/relay-x25519-keys.json"
expect_failure "rejects sealed Relay private-key JSON in the repository" \
    "sensitive key/token-shaped files are forbidden"

new_fixture relay-key-dockerignore-removed
sed '/relay-x25519-keys/d' \
    "$fixture_root/.dockerignore" \
    > "$fixture_root/.dockerignore.next"
mv "$fixture_root/.dockerignore.next" "$fixture_root/.dockerignore"
expect_failure "requires private Relay key JSON exclusion from Docker builds" \
    "Docker build-context private-key exclusion"

new_fixture relay-key-gitignore-removed
sed '/relay-x25519-keys/d' \
    "$fixture_root/.gitignore" \
    > "$fixture_root/.gitignore.next"
mv "$fixture_root/.gitignore.next" "$fixture_root/.gitignore"
expect_failure "requires private Relay key JSON exclusion from Git staging" \
    "Git private-key exclusion"

new_fixture compact-token
mkdir -p "$fixture_root/crates/op-collab/fixtures"
printf '%s\n' \
    '"abcdefghijklmnop.qrstuvwxyzABCDEF.abcdefghijklmnopqrstuvwxyzABCDEF0123456789"' \
    > "$fixture_root/crates/op-collab/fixtures/captured-ticket.txt"
expect_failure "rejects compact bearer tokens in non-source fixtures" \
    "high-signal credential/private-key material detected"

new_fixture smoke-compact-token
printf '%s\n' \
    '"abcdefghijklmnop.qrstuvwxyzABCDEF.abcdefghijklmnopqrstuvwxyzABCDEF0123456789"' \
    > "$fixture_root/crates/op-collab-smoke/captured-ticket.txt"
expect_failure "rejects compact bearer tokens in the smoke crate" \
    "high-signal credential/private-key material detected"

new_fixture desktop-sensitive-file
: > "$fixture_root/crates/op-collab-host/src/runtime/runtime-ticket.token"
expect_failure "rejects sensitive files in desktop collaboration integration" \
    "sensitive key/token-shaped files are forbidden"

new_fixture avatar-redirect-limit-removed
sed '/MAX_REDIRECTS/d' \
    "$fixture_root/crates/op-host-services/src/profile_avatar_fetch.rs" \
    > "$fixture_root/crates/op-host-services/src/profile_avatar_fetch.rs.next"
mv \
    "$fixture_root/crates/op-host-services/src/profile_avatar_fetch.rs.next" \
    "$fixture_root/crates/op-host-services/src/profile_avatar_fetch.rs"
expect_failure "requires the shared avatar redirect limit" \
    "bounded collaboration avatar fetch"

new_fixture desktop-public-avatar-delegation-removed
sed '/fetch_profile_avatar_blocking(request.url())/d' \
    "$fixture_root/crates/op-host-desktop/src/collab_avatar_host.rs" \
    > "$fixture_root/crates/op-host-desktop/src/collab_avatar_host.rs.next"
mv \
    "$fixture_root/crates/op-host-desktop/src/collab_avatar_host.rs.next" \
    "$fixture_root/crates/op-host-desktop/src/collab_avatar_host.rs"
expect_failure "requires public-only desktop collaboration avatar delegation" \
    "desktop avatar security-policy delegation"

new_fixture avatar-proxy-bypass-removed
: > "$fixture_root/crates/op-chat-agent/src/provider_dial.rs"
cat > "$fixture_root/crates/op-host-services/src/provider_dial.rs" <<'EOF'
fn fake_pinned_client() {
    let _ = ".no_proxy()";
    let _ = ".resolve_to_addrs";
}
EOF
expect_failure "requires proxy-free pinned avatar dialing" \
    "public HTTPS proxy bypass prevention"

new_fixture avatar-dns-pinning-removed
sed '/\.resolve_to_addrs/d' \
    "$fixture_root/crates/op-chat-agent/src/provider_dial.rs" \
    > "$fixture_root/crates/op-chat-agent/src/provider_dial.rs.next"
mv \
    "$fixture_root/crates/op-chat-agent/src/provider_dial.rs.next" \
    "$fixture_root/crates/op-chat-agent/src/provider_dial.rs"
expect_failure "requires connect-time DNS pinning for public avatar dialing" \
    "public HTTPS DNS pinning"

new_fixture auth-artifact-integrity-removed
: > "$fixture_root/crates/op-auth-bridge/build.rs"
expect_failure "requires authentication artifact integrity verification" \
    "authentication artifact integrity gate"

new_fixture auth-artifact-signature-removed
: > "$fixture_root/crates/op-auth-bridge/prebuilt_provenance.rs"
expect_failure "requires authentication artifact signature verification" \
    "authentication artifact signature verification"

new_fixture auth-matrix-test-removed
sed \
    '/cargo test --locked -p op-auth-bridge --test prebuilt_provenance/d' \
    "$fixture_root/.github/workflows/collab-security.yml" \
    > "$fixture_root/.github/workflows/collab-security.yml.next"
mv \
    "$fixture_root/.github/workflows/collab-security.yml.next" \
    "$fixture_root/.github/workflows/collab-security.yml"
expect_failure "requires the committed authentication matrix test" \
    "committed authentication matrix test"

new_fixture integration-line-cap
awk 'BEGIN { for (line = 1; line <= 801; line++) print "// integration line" }' \
    > "$fixture_root/crates/op-editor-host-core/src/collab/oversized.rs"
expect_failure "enforces the line cap across collaboration integration source" \
    "has 801 lines; maximum is 800"

new_fixture missing-provider-dial-workflow-trigger
awk '
    !removed && index($0, "crates/op-chat-agent/src/provider_dial.rs") {
        removed = 1
        next
    }
    { print }
' \
    "$fixture_root/.github/workflows/collab-security.yml" \
    > "$fixture_root/.github/workflows/collab-security.yml.next"
mv \
    "$fixture_root/.github/workflows/collab-security.yml.next" \
    "$fixture_root/.github/workflows/collab-security.yml"
expect_failure "requires both canonical provider-dial workflow triggers" \
    "collaboration security workflow path trigger"

new_fixture relay-edge-mtls-verification-removed
sed '/proxy_ssl_verify on;/d' \
    "$fixture_root/deploy/collab-relay-edge/global-nginx.conf" \
    > "$fixture_root/deploy/collab-relay-edge/global-nginx.conf.next"
mv \
    "$fixture_root/deploy/collab-relay-edge/global-nginx.conf.next" \
    "$fixture_root/deploy/collab-relay-edge/global-nginx.conf"
expect_failure "requires Global-to-CN outer-mTLS server verification" \
    "Global-to-CN inner-TLS passthrough boundary"

new_fixture relay-edge-client-crl-removed
awk '!index($0, "ssl_crl /run/secrets/global-edge-client-crl.pem;")' \
    "$fixture_root/deploy/collab-relay-edge/cn-federation-nginx.conf" \
    > "$fixture_root/deploy/collab-relay-edge/cn-federation-nginx.conf.next"
mv \
    "$fixture_root/deploy/collab-relay-edge/cn-federation-nginx.conf.next" \
    "$fixture_root/deploy/collab-relay-edge/cn-federation-nginx.conf"
expect_failure "requires revocation checking for Global edge client certificates" \
    "CN outer-mTLS federation boundary"

new_fixture relay-edge-crl-production-validation-removed
sed '/OPENPENCIL_RELAY_EDGE_VALIDATION_MODE=production/d' \
    "$fixture_root/deploy/collab-relay-edge/rotate-cn-crl.sh" \
    > "$fixture_root/deploy/collab-relay-edge/rotate-cn-crl.sh.next"
mv \
    "$fixture_root/deploy/collab-relay-edge/rotate-cn-crl.sh.next" \
    "$fixture_root/deploy/collab-relay-edge/rotate-cn-crl.sh"
expect_failure "requires production validation before Relay CRL activation" \
    "relay federation CRL activation boundary"

new_fixture relay-edge-source-rate-removed
awk '!index($0, "limit rate over 60/minute burst 20 packets")' \
    "$fixture_root/deploy/collab-relay-edge/install-global-new-connection-rate.sh" \
    > "$fixture_root/deploy/collab-relay-edge/install-global-new-connection-rate.sh.next"
mv \
    "$fixture_root/deploy/collab-relay-edge/install-global-new-connection-rate.sh.next" \
    "$fixture_root/deploy/collab-relay-edge/install-global-new-connection-rate.sh"
expect_failure "requires the overseas relay per-source connection-rate gate" \
    "overseas relay per-source connection-rate boundary"

new_fixture relay-edge-auto-restart-enabled
sed 's/restart: "no"/restart: unless-stopped/' \
    "$fixture_root/deploy/collab-relay-edge/compose.global.yaml" \
    > "$fixture_root/deploy/collab-relay-edge/compose.global.yaml.next"
mv \
    "$fixture_root/deploy/collab-relay-edge/compose.global.yaml.next" \
    "$fixture_root/deploy/collab-relay-edge/compose.global.yaml"
expect_failure "requires supervised relay startup after the nftables gate" \
    "overseas relay supervised fixed-port boundary"

new_fixture relay-edge-verifier-not-executable
chmod -x "$fixture_root/deploy/collab-relay-edge/verify-rate-rules.py"
expect_failure "requires executable deployment gate helpers" \
    "deployment gate executable boundary"

new_fixture relay-bearer-header-buffer-removed
awk '
    !removed && /client_header_buffer_size 64k;/ {
        removed = 1
        next
    }
    { print }
' \
    "$fixture_root/deploy/collab-relay/nginx.conf" \
    > "$fixture_root/deploy/collab-relay/nginx.conf.next"
mv \
    "$fixture_root/deploy/collab-relay/nginx.conf.next" \
    "$fixture_root/deploy/collab-relay/nginx.conf"
expect_failure "requires 48 KiB bearer buffers on both relay ingresses" \
    "48 KiB relay bearer ingress header boundary"

new_fixture cn-federation-aggregate-limit-removed
sed '/limit_conn relay_federation_connections 512;/d' \
    "$fixture_root/deploy/collab-relay/nginx.conf" \
    > "$fixture_root/deploy/collab-relay/nginx.conf.next"
mv \
    "$fixture_root/deploy/collab-relay/nginx.conf.next" \
    "$fixture_root/deploy/collab-relay/nginx.conf"
expect_failure "keeps the trusted federation backhaul off the public per-IP ceiling" \
    "CN WSS/federation ingress boundary"

new_fixture locator-hsm-boundary-removed
sed '/OPENPENCIL_COLLAB_LOCATOR_HSM_SOCKET:/d' \
    "$fixture_root/deploy/collab-relay-locator/compose.yaml" \
    > "$fixture_root/deploy/collab-relay-locator/compose.yaml.next"
mv \
    "$fixture_root/deploy/collab-relay-locator/compose.yaml.next" \
    "$fixture_root/deploy/collab-relay-locator/compose.yaml"
expect_failure "requires the external locator HSM socket boundary" \
    "locator production container boundary"

new_fixture locator-per-source-limit-removed
sed '/limit_req_zone.*openpencil_locator_per_source/d' \
    "$fixture_root/deploy/collab-relay-locator/nginx-http-limits.conf" \
    > "$fixture_root/deploy/collab-relay-locator/nginx-http-limits.conf.next"
mv \
    "$fixture_root/deploy/collab-relay-locator/nginx-http-limits.conf.next" \
    "$fixture_root/deploy/collab-relay-locator/nginx-http-limits.conf"
expect_failure "requires per-source locator ingress throttling" \
    "locator per-source ingress boundary"

new_fixture locator-pairing-route-removed
awk '!index($0, "location = /v1/pairing-code {")' \
    "$fixture_root/deploy/collab-relay-locator/nginx-location.conf" \
    > "$fixture_root/deploy/collab-relay-locator/nginx-location.conf.next"
mv \
    "$fixture_root/deploy/collab-relay-locator/nginx-location.conf.next" \
    "$fixture_root/deploy/collab-relay-locator/nginx-location.conf"
expect_failure "requires the pairing publish route at the locator ingress" \
    "locator exact-route ingress boundary"

new_fixture locator-edge-crl-secure-ownership-removed
awk '!index($0, "CRL/CA files must be root:101 mode 0440")' \
    "$fixture_root/deploy/collab-relay-locator-edge/rotate-cn-crl.sh" \
    > "$fixture_root/deploy/collab-relay-locator-edge/rotate-cn-crl.sh.next"
mv \
    "$fixture_root/deploy/collab-relay-locator-edge/rotate-cn-crl.sh.next" \
    "$fixture_root/deploy/collab-relay-locator-edge/rotate-cn-crl.sh"
expect_failure "requires secure ownership before locator CRL activation" \
    "locator federation CRL activation boundary"

new_fixture mutable-relay-container-base
awk '
    /^FROM rust:/ {
        print "FROM rust:1.94-bookworm AS build"
        next
    }
    { print }
' \
    "$fixture_root/deploy/collab-relay/Dockerfile" \
    > "$fixture_root/deploy/collab-relay/Dockerfile.next"
mv \
    "$fixture_root/deploy/collab-relay/Dockerfile.next" \
    "$fixture_root/deploy/collab-relay/Dockerfile"
expect_failure "requires immutable digests for relay container base images" \
    "relay container base images must use reviewed immutable SHA-256 digests"

new_fixture locator-edge-mtls-verification-removed
sed '/proxy_ssl_verify on;/d' \
    "$fixture_root/deploy/collab-relay-locator-edge/global-nginx.conf" \
    > "$fixture_root/deploy/collab-relay-locator-edge/global-nginx.conf.next"
mv \
    "$fixture_root/deploy/collab-relay-locator-edge/global-nginx.conf.next" \
    "$fixture_root/deploy/collab-relay-locator-edge/global-nginx.conf"
expect_failure "requires outer-mTLS verification on the overseas locator ingress" \
    "overseas locator inner-TLS passthrough boundary"

new_fixture locator-edge-exact-host-removed
awk '!index($0, "if ($http_host != locator.example.cn) {")' \
    "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf" \
    > "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf.next"
mv \
    "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf.next" \
    "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf"
expect_failure "requires an exact inner HTTPS Host at the CN locator terminator" \
    "CN locator exact inner-HTTPS boundary"

new_fixture locator-edge-pairing-claim-route-removed
awk '!index($0, "location = /v1/pairing-code/claim {")' \
    "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf" \
    > "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf.next"
mv \
    "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf.next" \
    "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf"
expect_failure "requires the pairing claim route at the CN locator terminator" \
    "CN locator exact inner-HTTPS boundary"

new_fixture locator-edge-source-rate-removed
awk '!index($0, "limit rate over 60/minute burst 20 packets")' \
    "$fixture_root/deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh" \
    > "$fixture_root/deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh.next"
mv \
    "$fixture_root/deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh.next" \
    "$fixture_root/deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh"
expect_failure "requires the overseas locator per-source connection-rate gate" \
    "overseas locator per-source connection-rate boundary"

new_fixture missing-hard-limit
awk '!/MAX_OPS_PER_TXN/' \
    "$fixture_root/crates/op-collab/src/protocol.rs" \
    > "$fixture_root/crates/op-collab/src/protocol.rs.next"
mv \
    "$fixture_root/crates/op-collab/src/protocol.rs.next" \
    "$fixture_root/crates/op-collab/src/protocol.rs"
expect_failure "rejects removal of a protocol hard-limit anchor" \
    "protocol hard limit"

new_fixture public-transport-queue
sed \
    's/pub(crate) struct BoundedTransferQueue/pub struct BoundedTransferQueue/' \
    "$fixture_root/crates/op-collab-transport/src/queue.rs" \
    > "$fixture_root/crates/op-collab-transport/src/queue.rs.next"
mv \
    "$fixture_root/crates/op-collab-transport/src/queue.rs.next" \
    "$fixture_root/crates/op-collab-transport/src/queue.rs"
expect_failure "rejects exposing the transport queue implementation" \
    "bounded queue/rate type"

new_fixture untyped-boundary-error
printf '%s\n' \
    'fn bad_boundary() -> Result<(), String> { Ok(()) }' \
    >> "$fixture_root/crates/op-collab/src/protocol.rs"
expect_failure "rejects untyped public boundary errors" \
    "untyped Result<_, String/&str>"

if [[ "$failure_count" -ne 0 ]]; then
    printf '%s\n' \
        "check-collab-security-boundaries.test.sh: $failure_count mutation test(s) failed." \
        >&2
    exit 1
fi

printf '%s\n' \
    "check-collab-security-boundaries.test.sh: all $test_index mutation tests pass."
