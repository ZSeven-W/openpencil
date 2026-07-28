#!/usr/bin/env bash
# Mutation tests for check-collab-security-boundaries.sh.

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
gate_source="$script_dir/check-collab-security-boundaries.sh"
test_root=$(mktemp -d "${TMPDIR:-/tmp}/collab-security-gate.XXXXXX")
trap 'rm -rf "$test_root"' EXIT

fixture_root=
gate_output=
gate_status=0
test_index=0
failure_count=0

new_fixture() {
    fixture_root="$test_root/$1"
    mkdir -p \
        "$fixture_root/docs/security" \
        "$fixture_root/tools" \
        "$fixture_root/fake-bin" \
        "$fixture_root/.github/workflows" \
        "$fixture_root/crates/op-collab/src" \
        "$fixture_root/crates/op-collab/tests" \
        "$fixture_root/crates/op-collab-transport/src" \
        "$fixture_root/crates/op-collab-smoke/src" \
        "$fixture_root/crates/op-auth-bridge/src" \
        "$fixture_root/crates/op-auth-bridge/tests" \
        "$fixture_root/crates/op-util/src" \
        "$fixture_root/crates/op-editor-core/src" \
        "$fixture_root/crates/op-editor-host-core/src/collab" \
        "$fixture_root/crates/op-editor-ui/src" \
        "$fixture_root/crates/op-host-native/src" \
        "$fixture_root/crates/op-host-desktop/src/collab_runtime" \
        "$fixture_root/crates/op-host-services/src" \
        "$fixture_root/crates/op-i18n/src"

    cp "$gate_source" "$fixture_root/tools/check-collab-security-boundaries.sh"
    cp "$script_dir/check-op-auth-prebuilt.sh" "$fixture_root/tools/check-op-auth-prebuilt.sh"
    cp "$script_dir/check-op-auth-prebuilt.test.sh" "$fixture_root/tools/check-op-auth-prebuilt.test.sh"
    cp "$script_dir/package-op-auth-prebuilt.sh" "$fixture_root/tools/package-op-auth-prebuilt.sh"

    cat > "$fixture_root/Cargo.toml" <<'EOF'
[workspace]
members = [
    "crates/op-collab",
    "crates/op-collab-transport",
    "crates/op-auth-bridge",
]

[workspace.package]
license = "MIT"
EOF

    cat > "$fixture_root/docs/security/p2p-collaboration-threat-model.md" <<'EOF'
# Fixture threat model

This file exists so the executable boundary gate can verify its public contract.
EOF

    cat > "$fixture_root/.github/workflows/collab-security.yml" <<'EOF'
pull_request:
  paths:
    - 'crates/op-collab-smoke/**'
    - 'crates/op-util/**'
    - 'crates/op-editor-core/**'
    - 'crates/op-editor-host-core/**'
    - 'crates/op-editor-ui/**'
    - 'crates/op-host-native/**'
    - 'crates/op-host-desktop/**'
    - 'crates/op-host-services/**'
    - 'crates/op-i18n/**'
    - 'tools/check-op-auth-prebuilt.sh'
    - 'tools/check-op-auth-prebuilt.test.sh'
    - 'tools/package-op-auth-prebuilt.sh'
push:
  paths:
    - 'crates/op-collab-smoke/**'
    - 'crates/op-util/**'
    - 'crates/op-editor-core/**'
    - 'crates/op-editor-host-core/**'
    - 'crates/op-editor-ui/**'
    - 'crates/op-host-native/**'
    - 'crates/op-host-desktop/**'
    - 'crates/op-host-services/**'
    - 'crates/op-i18n/**'
    - 'tools/check-op-auth-prebuilt.sh'
    - 'tools/check-op-auth-prebuilt.test.sh'
    - 'tools/package-op-auth-prebuilt.sh'
steps:
  - run: bash tools/check-op-auth-prebuilt.sh
  - run: bash tools/check-op-auth-prebuilt.test.sh
  - run: bash -n tools/package-op-auth-prebuilt.sh
  - run: cargo test --locked -p op-auth-bridge --test prebuilt_provenance
  - run: cargo test --locked -p op-collab-transport frame::tests
EOF

    cat > "$fixture_root/crates/op-collab/Cargo.toml" <<'EOF'
[package]
name = "op-collab"
version = "0.0.0"
license.workspace = true
EOF
    : > "$fixture_root/crates/op-collab/LICENSE"

    cat > "$fixture_root/crates/op-collab-transport/Cargo.toml" <<'EOF'
[package]
name = "op-collab-transport"
version = "0.0.0"
license.workspace = true
EOF
    : > "$fixture_root/crates/op-collab-transport/LICENSE"
    : > "$fixture_root/crates/op-collab-smoke/LICENSE"

    cat > "$fixture_root/crates/op-auth-bridge/Cargo.toml" <<'EOF'
[package]
name = "op-auth-bridge"
version = "0.0.0"
license.workspace = true

[features]
test-issuer = []
EOF
    : > "$fixture_root/crates/op-auth-bridge/LICENSE"

    cat > "$fixture_root/crates/op-collab/src/protocol.rs" <<'EOF'
pub const MAX_ENVELOPE_BYTES: u32 = 1024;
pub const MAX_TXN_BYTES: u32 = 1024;
pub const MAX_OPS_PER_TXN: u32 = 8;
pub const MAX_DOCUMENT_NODES: u32 = 32;
pub const MAX_TREE_DEPTH: u32 = 8;
pub const MAX_IDENTIFIER_BYTES: u32 = 128;
pub const MAX_OPAQUE_TICKET_BYTES: usize = 1024;
pub const MAX_VALIDATION_NODE_VISITS_PER_TXN: u32 = 256;
pub struct WireLimits;
const _: &str = "opaque tickets require the dedicated renewal encoder";
EOF

    cat > "$fixture_root/crates/op-collab/src/apply_context.rs" <<'EOF'
pub struct ApplyLimits;
EOF

    cat > "$fixture_root/crates/op-collab/src/codec.rs" <<'EOF'
use serde_json::value::RawValue;
pub fn to_json_vec_with_limits() {}
enum RawNonSensitiveMessage {}
struct RawFrameEnvelope {
    body: RawNonSensitiveMessage,
}
pub fn from_json_slice_with_limits(bytes: &[u8], limits: ()) {
    reject_renew_ticket_before_generic_value_decode(bytes)?;
    let mut value = decode_json_value(bytes, limits)?;
}
fn reject_renew_ticket_before_generic_value_decode(_bytes: &[u8]) -> Result<(), ()> {
    Ok(())
}
fn decode_json_value(_bytes: &[u8], _limits: ()) -> Result<(), ()> {
    Ok(())
}
pub struct SensitiveFrameJson;
fn sensitive(raw: ()) {
    let mut encoded = Vec::new();
    serde_json::to_writer(&mut *encoded, &raw);
}
struct DedicatedOpaqueTicketRef<'a>(&'a OpaqueTicket);
impl DedicatedOpaqueTicketRef<'_> {
    fn serialize(&self) {
        serializer.serialize_str(self.0.expose());
    }
}
EOF

    cat > "$fixture_root/crates/op-collab/src/ticket_json.rs" <<'EOF'
use serde_json::value::RawValue;
use zeroize::Zeroizing;
struct BorrowedRenewTicketPayload<'a> {
    opaque_ticket: &'a RawValue,
}
fn decode(raw: &str) {
    let decoded = Zeroizing::new(String::with_capacity(raw.len()));
    let _ = OpaqueTicket::from_zeroizing(decoded);
}
EOF

    cat > "$fixture_root/crates/op-collab/src/error.rs" <<'EOF'
pub enum ProtocolError {
    SensitiveCredentialRequiresDedicatedCodec,
}
EOF

    cat > "$fixture_root/crates/op-collab/tests/credential_ownership.rs" <<'EOF'
assert_not_impl_any!(OpaqueTicket: Clone);
assert_not_impl_any!(RenewTicket: Clone);
assert_not_impl_any!(CollabMessage: Clone);
assert_not_impl_any!(FrameEnvelope: Clone);
assert_not_impl_any!(OpaqueTicket: serde::de::DeserializeOwned);
assert_not_impl_any!(RenewTicket: serde::de::DeserializeOwned);
assert_not_impl_any!(CollabMessage: serde::de::DeserializeOwned);
fn generic_raw_codecs_reject_credential_frames() {}
fn direct_serde_renewal_serialization_is_fail_closed() {}
fn generic_decoder_rejects_renewal_before_payload_deserialization() {}
fn sensitive_discriminator_rejects_duplicate_message_fields() {}
fn dedicated_codec_round_trips_and_debug_redacts_the_secret() {}
fn dedicated_decoder_unescapes_directly_into_zeroizing_storage() {}
fn dedicated_decoder_rejects_malformed_escapes_and_surrogates() {}
fn dedicated_decoder_rejects_duplicate_and_unknown_ticket_fields() {}
fn dedicated_decoder_enforces_decoded_ticket_bounds() {}
EOF

    cat > "$fixture_root/crates/op-collab/tests/outbound_limits.rs" <<'EOF'
#[test]
fn presence_payload_limit_applies_to_encode_and_decode() {}
EOF

    cat > "$fixture_root/crates/op-collab-transport/src/config.rs" <<'EOF'
pub const MAX_CONTROL_TRANSFER_BYTES: usize = 1024;
pub const MAX_TICKET_BYTES: usize = 1024;
pub const MAX_TXN_TRANSFER_BYTES: usize = 4096;
pub const MAX_SNAPSHOT_TRANSFER_BYTES: usize = 8192;
pub struct TimeoutConfig;
pub struct ConnectionLimits;
pub struct RateLimitConfig;
pub struct TransportConfig;
pub struct ConfigError;
impl TransportConfig {
    pub fn validate(self) -> Result<Self, ConfigError> {
        Ok(self)
    }
}
#[cfg(test)]
fn invalid_resource_limits_fail_closed() {}
EOF

    cat > "$fixture_root/crates/op-collab-transport/src/frame.rs" <<'EOF'
#[test]
fn mislabeled_renewal_never_reaches_generic_payload_deserialization() {}
EOF

    cat > "$fixture_root/crates/op-collab-transport/src/queue.rs" <<'EOF'
pub(crate) struct QueueItem;
impl QueueItem {
    pub(crate) fn reliable(class: TransferClass) {
        if class == TransferClass::Ticket {}
    }
    pub(crate) fn coalescing(class: TransferClass) {
        if class == TransferClass::Ticket {}
    }
    pub(crate) fn sensitive_ticket_frame() {}
    pub(crate) fn sensitive_admission() {}
}
pub(crate) struct BoundedTransferQueue;
pub struct SharedQueueBudget;
pub struct TokenBucket;
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache.rs" <<'EOF'
pub struct CollabJwksCacheLimits;
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs" <<'EOF'
#![cfg(test)]
fn deterministic_test_key(seed: u8) {
    let _ = SigningKey::from_bytes(&[seed; 32]);
}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_ticket.rs" <<'EOF'
pub const MAX_COLLAB_TICKET_BYTES: usize = 1024;
EOF

    cat > "$fixture_root/crates/op-auth-bridge/build.rs" <<'EOF'
fn main() {
    prebuilt_provenance::validate_prebuilt();
}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/prebuilt_provenance.rs" <<'EOF'
const HARDENING_PROFILE_V1: &str = "op-auth-hardened-v1";
fn validate() {
    let _ = "Sha256::digest";
    let _ = "verify_strict";
}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/lib.rs" <<'EOF'
#[cfg(any(test, feature = "test-issuer"))]
mod collab_test_issuer;
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_test_issuer.rs" <<'EOF'
//! The seed below is public test material.
pub const TEST_ISSUER: &str = "https://collab.test.invalid";
pub const PUBLIC_TEST_SEED: [u8; 32] = [7; 32];
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_verifier.rs" <<'EOF'
#[cfg(test)]
fn production_trust_root_rejects_the_public_test_issuer() {}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/tests/collab_verifier.rs" <<'EOF'
#![cfg(feature = "test-issuer")]
#[test]
fn public_fixture_is_explicitly_enabled() {}
EOF

    cat > "$fixture_root/crates/op-host-desktop/src/collab_avatar_host.rs" <<'EOF'
const MAX_REDIRECTS: usize = 3;
const REQUEST_TIMEOUT: u64 = 5;
const MAX_AVATAR_ENCODED_BYTES: usize = 1024;
fn public_https_client() {}
EOF

    cat > "$fixture_root/crates/op-host-services/src/public_https_client.rs" <<'EOF'
pub fn public_https_client() {}
EOF

    cat > "$fixture_root/crates/op-host-services/src/provider_dial.rs" <<'EOF'
fn pinned_client() {
    let _ = ".no_proxy()";
    let _ = ".resolve_to_addrs";
}
EOF

    cat > "$fixture_root/crates/op-host-services/src/web_credentials.rs" <<'EOF'
pub fn is_restricted_ip() -> bool { true }
EOF

    cat > "$fixture_root/crates/op-editor-ui/src/collab_avatar_runtime.rs" <<'EOF'
pub const MAX_AVATAR_SOURCE_PIXELS: u64 = 1_048_576;
EOF

    cat > "$fixture_root/crates/op-host-desktop/src/collab_runtime/types.rs" <<'EOF'
assert_not_impl_any!(OwnerNetworkCommand: Clone);
assert_not_impl_any!(GuestNetworkCommand: Clone);
assert_not_impl_any!(PeerNetworkCommand: Clone);
fn verification_commands_move_the_original_ticket_allocation() {}
EOF

    cat > "$fixture_root/fake-bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "tree" ]]; then
    printf 'unexpected fake cargo invocation: %s\n' "$*" >&2
    exit 2
fi
printf '%s\n' \
    'op-collab v0.0.0' \
    'serde v1.0.0'
if [[ -f "$PWD/.fake-wasm-forbidden" ]]; then
    printf '%s\n' 'tokio v1.0.0'
fi
EOF
    chmod +x "$fixture_root/fake-bin/cargo"

}

run_gate() {
    set +e
    gate_output=$(
        cd "$fixture_root"
        PATH="$fixture_root/fake-bin:$PATH" \
            bash tools/check-collab-security-boundaries.sh 2>&1
    )
    gate_status=$?
    set -e
}

pass_case() {
    test_index=$((test_index + 1))
    printf 'ok %s - %s\n' "$test_index" "$1"
}

fail_case() {
    test_index=$((test_index + 1))
    failure_count=$((failure_count + 1))
    printf 'not ok %s - %s\n' "$test_index" "$1"
    printf '%s\n' "$gate_output" | sed 's/^/# /'
}

expect_pass() {
    label=$1
    run_gate
    if [[ "$gate_status" -eq 0 ]]; then
        pass_case "$label"
    else
        fail_case "$label"
    fi
}

expect_failure() {
    label=$1
    expected=$2
    run_gate
    if [[ "$gate_status" -ne 0 && "$gate_output" == *"$expected"* ]]; then
        pass_case "$label"
    else
        fail_case "$label (expected failure containing '$expected')"
    fi
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
    index($0, "    reject_renew_ticket_before_generic_value_decode(bytes)?;") == 1 {
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

new_fixture desktop-renewal-vec-copy
printf '%s\n' \
    'fn bad(ticket: Ticket) { let _ = ticket.expose().as_bytes().to_vec(); }' \
    >> "$fixture_root/crates/op-host-desktop/src/collab_runtime/types.rs"
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

new_fixture deterministic-external-test-without-cfg
sed '/#!\[cfg(test)\]/d' \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs" \
    > "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs.next"
mv \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs.next" \
    "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs"
expect_failure "requires an explicit cfg(test) boundary for external unit tests" \
    "deterministic signing/key seed leaked"

new_fixture sensitive-key-file
: > "$fixture_root/crates/op-collab-transport/peer.key"
expect_failure "rejects key-shaped repository fixtures" \
    "sensitive key/token-shaped files are forbidden"

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
: > "$fixture_root/crates/op-host-desktop/src/collab_runtime/runtime-ticket.token"
expect_failure "rejects sensitive files in desktop collaboration integration" \
    "sensitive key/token-shaped files are forbidden"

new_fixture avatar-proxy-bypass-removed
: > "$fixture_root/crates/op-host-services/src/provider_dial.rs"
expect_failure "requires proxy-free pinned avatar dialing" \
    "public HTTPS proxy bypass prevention"

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

new_fixture missing-workflow-trigger
awk '
    !removed && index($0, "crates/op-host-desktop/**") {
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
expect_failure "rejects removal of either integration workflow trigger" \
    "collaboration security workflow path trigger"

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
