#!/usr/bin/env bash
# Enforces the public collaboration security and ownership boundaries.
#
# Run from any directory inside a normal checkout. The script intentionally
# uses only Bash, Cargo, and standard Unix text tools so the dedicated CI job
# does not depend on an unpinned scanner.

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)
cd "$repo_root"

failures=()

# Collaboration authorization and mutation gates intentionally live in ordinary
# editor, keyboard, save, widget, and locale modules. Scanning only paths whose
# names contain "collab" would miss those integration boundaries.
collab_scan_roots=(
    crates/op-collab
    crates/op-collab-transport
    crates/op-collab-smoke
    crates/op-auth-bridge
    crates/op-util/src
    crates/op-editor-core/src
    crates/op-editor-host-core/src
    crates/op-editor-ui/src
    crates/op-host-native/src
    crates/op-host-desktop/src
    crates/op-host-services/src/public_https_client.rs
    crates/op-host-services/src/provider_dial.rs
    crates/op-host-services/src/web_credentials.rs
    crates/op-i18n/src
)

collab_boundary_files() {
    local root
    for root in "${collab_scan_roots[@]}"; do
        if [[ -d "$root" ]]; then
            find "$root" -type f ! -path '*/prebuilt/*' -print
        elif [[ -f "$root" ]]; then
            printf '%s\n' "$root"
        fi
    done | LC_ALL=C sort -u
}

collab_rust_source_files() {
    collab_boundary_files | grep -E '\.rs$' || true
}

record_failure() {
    failures+=("$1")
}

require_file() {
    file=$1
    if [[ ! -f "$file" ]]; then
        record_failure "missing required file: $file"
    fi
}

require_literal() {
    file=$1
    literal=$2
    label=$3
    if [[ ! -f "$file" ]] || ! grep -Fq -- "$literal" "$file"; then
        record_failure "$label: expected '$literal' in $file"
    fi
}

require_literal_count() {
    local file=$1
    local literal=$2
    local minimum=$3
    local label=$4
    local count=0
    if [[ -f "$file" ]]; then
        count=$(grep -Fc -- "$literal" "$file" || true)
    fi
    if [[ "$count" -lt "$minimum" ]]; then
        record_failure "$label: expected '$literal' at least $minimum times in $file"
    fi
}

for required in \
    Cargo.toml \
    crates/op-collab/Cargo.toml \
    crates/op-collab/LICENSE \
    crates/op-collab/src/ticket_json.rs \
    crates/op-collab/tests/credential_ownership.rs \
    crates/op-collab-transport/Cargo.toml \
    crates/op-collab-transport/LICENSE \
    crates/op-collab-smoke/LICENSE \
    crates/op-auth-bridge/Cargo.toml \
    crates/op-auth-bridge/LICENSE \
    crates/op-auth-bridge/prebuilt_provenance.rs \
    .github/workflows/collab-security.yml \
    docs/security/p2p-collaboration-threat-model.md; do
    require_file "$required"
done

for auth_tool_path in \
    "tools/check-op-auth-prebuilt.sh" \
    "tools/check-op-auth-prebuilt.test.sh" \
    "tools/package-op-auth-prebuilt.sh"; do
    require_file "$auth_tool_path"
    require_literal_count .github/workflows/collab-security.yml \
        "$auth_tool_path" 3 "authentication artifact workflow gate"
done

for workflow_path in \
    "crates/op-collab-smoke/**" \
    "crates/op-util/**" \
    "crates/op-editor-core/**" \
    "crates/op-editor-host-core/**" \
    "crates/op-editor-ui/**" \
    "crates/op-host-native/**" \
    "crates/op-host-desktop/**" \
    "crates/op-host-services/**" \
    "crates/op-i18n/**"; do
    require_literal_count .github/workflows/collab-security.yml \
        "$workflow_path" 2 "collaboration security workflow path trigger"
done

# The protocol core is allowed in browser/wasm dependency graphs. Native
# transport, authentication, key generation, and HTTP/TLS stacks are not.
wasm_tree=
if ! wasm_tree=$(cargo tree \
    -p op-collab \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --edges normal,build \
    --prefix none \
    --locked 2>&1); then
    record_failure "WASM dependency closure could not be resolved: $wasm_tree"
fi

forbidden_wasm_pattern='(^|[[:space:]])(op-collab-transport|op-auth-bridge|tokio|mio|socket2|snow|mdns-sd|x25519-dalek|getrandom|reqwest|ureq|native-tls|openssl|rustls|ring|libc)([[:space:]]|$)'
forbidden_wasm=$(printf '%s\n' "$wasm_tree" \
    | grep -E "$forbidden_wasm_pattern" \
    | LC_ALL=C sort -u || true)
if [[ -n "$forbidden_wasm" ]]; then
    record_failure "WASM boundary includes native/auth dependencies:
$forbidden_wasm"
fi

# Collaboration crates are open MIT code. Future op-collab-* crates are picked
# up automatically instead of relying on a hand-maintained allowlist.
if ! grep -Eq '^[[:space:]]*license[[:space:]]*=[[:space:]]*"MIT"[[:space:]]*$' Cargo.toml; then
    record_failure "workspace.package license must remain exactly MIT"
fi

found_collab=0
found_transport=0
found_auth=0
while IFS= read -r manifest; do
    package_name=$(awk '
        /^\[package\][[:space:]]*$/ { in_package = 1; next }
        /^\[/ && in_package { exit }
        in_package && /^[[:space:]]*name[[:space:]]*=/ {
            line = $0
            sub(/^[[:space:]]*name[[:space:]]*=[[:space:]]*"/, "", line)
            sub(/".*$/, "", line)
            print line
            exit
        }
    ' "$manifest")
    case "$package_name" in
        op-collab)
            found_collab=1
            ;;
        op-collab-*)
            found_transport=1
            ;;
        op-auth-bridge)
            found_auth=1
            ;;
        *)
            continue
            ;;
    esac
    if ! grep -Eq '^[[:space:]]*license\.workspace[[:space:]]*=[[:space:]]*true[[:space:]]*$' \
        "$manifest" \
        && ! grep -Eq '^[[:space:]]*license[[:space:]]*=[[:space:]]*"MIT"[[:space:]]*$' \
            "$manifest"; then
        record_failure "$package_name must inherit or declare the MIT license: $manifest"
    fi
done < <(find crates -mindepth 2 -maxdepth 2 -name Cargo.toml -print | LC_ALL=C sort)

[[ "$found_collab" -eq 1 ]] \
    || record_failure "op-collab was not found by the collaboration license gate"
[[ "$found_transport" -eq 1 ]] \
    || record_failure "no op-collab-* transport/integration crate was found"
[[ "$found_auth" -eq 1 ]] \
    || record_failure "op-auth-bridge was not found by the collaboration license gate"

# Committed security artifacts remain inspectable client inputs, but silent
# corruption or substitution must fail both CI and the build-script check.
while IFS= read -r auth_artifact; do
    checksum_file=$(dirname "$auth_artifact")/SHA256
    if [[ ! -f "$checksum_file" ]]; then
        record_failure "missing authentication artifact checksum: $checksum_file"
        continue
    fi
    expected_checksum=$(tr -d '[:space:]' < "$checksum_file")
    if command -v sha256sum >/dev/null 2>&1; then
        actual_checksum=$(sha256sum "$auth_artifact" | awk '{ print $1 }')
    elif command -v shasum >/dev/null 2>&1; then
        actual_checksum=$(shasum -a 256 "$auth_artifact" | awk '{ print $1 }')
    else
        record_failure "no SHA-256 tool available for authentication artifact verification"
        break
    fi
    if [[ "$expected_checksum" != "$actual_checksum" ]]; then
        record_failure "authentication artifact checksum mismatch: $auth_artifact"
    fi
done < <(
    find crates/op-auth-bridge/prebuilt \
        -type f \( -name '*.a' -o -name '*.lib' \) \
        -print 2>/dev/null | LC_ALL=C sort
)

# Required typed resource contracts. These anchors deliberately name public
# types and hard caps rather than attempting to infer semantics from numbers.
require_literal crates/op-collab/src/protocol.rs \
    "pub struct WireLimits" "protocol typed limits"
require_literal crates/op-collab/src/apply_context.rs \
    "pub struct ApplyLimits" "apply typed limits"
require_literal crates/op-collab/src/codec.rs \
    "to_json_vec_with_limits" "bounded outbound protocol encoding"
require_literal crates/op-collab/src/codec.rs \
    "from_json_slice_with_limits" "bounded inbound protocol decoding"
require_literal crates/op-collab/src/codec.rs \
    "pub struct SensitiveFrameJson" "redacted zeroizing credential JSON wrapper"
require_literal crates/op-collab/src/codec.rs \
    "serde_json::to_writer(&mut *encoded, &raw)" "direct credential JSON serializer"
require_literal crates/op-collab/src/codec.rs \
    "serializer.serialize_str(self.0.expose())" \
    "dedicated opaque-ticket serializer wrapper"
require_literal crates/op-collab/src/codec.rs \
    "enum RawNonSensitiveMessage" "credential-free generic wire enum"
require_literal crates/op-collab/src/codec.rs \
    "body: RawNonSensitiveMessage" "credential-free generic frame body"
require_literal crates/op-collab/src/ticket_json.rs \
    "serde_json::value::RawValue" "borrowed inbound credential discriminator"
require_literal crates/op-collab/src/ticket_json.rs \
    "opaque_ticket: &'a RawValue" "borrowed raw opaque-ticket payload"
require_literal crates/op-collab/src/ticket_json.rs \
    "Zeroizing::new(String::with_capacity" "direct zeroizing ticket string decoder"
require_literal crates/op-collab/src/ticket_json.rs \
    "OpaqueTicket::from_zeroizing(decoded)" "zeroizing opaque-ticket construction"
require_literal crates/op-collab/src/protocol.rs \
    "opaque tickets require the dedicated renewal encoder" \
    "fail-closed generic opaque-ticket serialization"
if grep -qF '#[derive(PartialEq, Serialize, Deserialize)]' \
    crates/op-collab/src/protocol.rs; then
    record_failure \
        "CollabMessage must not use derived Deserialize because adjacent payloads can buffer credentials"
fi
ordinary_opaque_ticket_deserializer=$(grep -nF \
    'String::deserialize' crates/op-collab/src/protocol.rs 2>/dev/null || true)
if [[ -n "$ordinary_opaque_ticket_deserializer" ]]; then
    record_failure "OpaqueTicket must not deserialize through an ordinary String:
$ordinary_opaque_ticket_deserializer"
fi
ordinary_ticket_deserializer=$(grep -nE \
    'String::deserialize|serde_json::from_value|serde_json::from_(str|slice)[[:space:]]*::<[[:space:]]*String' \
    crates/op-collab/src/ticket_json.rs 2>/dev/null || true)
if [[ -n "$ordinary_ticket_deserializer" ]]; then
    record_failure "dedicated ticket decoder must not materialize ordinary strings or Values:
$ordinary_ticket_deserializer"
fi
credential_probe_line=$(grep -nE \
    '^[[:space:]]*reject_renew_ticket_before_generic_value_decode\(bytes\)\?;' \
    crates/op-collab/src/codec.rs | head -1 | cut -d: -f1 || true)
generic_value_decode_line=$(grep -nF \
    'let mut value = decode_json_value(bytes, limits)?;' \
    crates/op-collab/src/codec.rs | head -1 | cut -d: -f1 || true)
if [[ -z "$credential_probe_line" || -z "$generic_value_decode_line" ]] \
    || [[ "$credential_probe_line" -ge "$generic_value_decode_line" ]]; then
    record_failure \
        "generic credential discriminator must run before JSON Value decoding"
fi
require_literal crates/op-collab/src/error.rs \
    "SensitiveCredentialRequiresDedicatedCodec" "dedicated credential codec failure"
require_literal crates/op-collab/tests/credential_ownership.rs \
    "generic_raw_codecs_reject_credential_frames" "generic credential encoder rejection test"
for non_deserializable_type in OpaqueTicket RenewTicket CollabMessage; do
    require_literal crates/op-collab/tests/credential_ownership.rs \
        "assert_not_impl_any!($non_deserializable_type: serde::de::DeserializeOwned);" \
        "credential-bearing protocol type must not implement generic Deserialize"
done
require_literal crates/op-collab/tests/credential_ownership.rs \
    "direct_serde_renewal_serialization_is_fail_closed" \
    "direct serde credential serialization rejection test"
require_literal crates/op-collab/tests/credential_ownership.rs \
    "generic_decoder_rejects_renewal_before_payload_deserialization" \
    "generic credential pre-deserialization rejection test"
require_literal crates/op-collab/tests/credential_ownership.rs \
    "sensitive_discriminator_rejects_duplicate_message_fields" \
    "credential discriminator duplicate-field regression test"
require_literal crates/op-collab/tests/credential_ownership.rs \
    "dedicated_codec_round_trips_and_debug_redacts_the_secret" \
    "dedicated credential codec roundtrip test"
for dedicated_ticket_test in \
    dedicated_decoder_unescapes_directly_into_zeroizing_storage \
    dedicated_decoder_rejects_malformed_escapes_and_surrogates \
    dedicated_decoder_rejects_duplicate_and_unknown_ticket_fields \
    dedicated_decoder_enforces_decoded_ticket_bounds; do
    require_literal crates/op-collab/tests/credential_ownership.rs \
        "$dedicated_ticket_test" "dedicated credential raw-decoder regression test"
done
require_literal crates/op-collab-transport/src/frame.rs \
    "mislabeled_renewal_never_reaches_generic_payload_deserialization" \
    "mislabeled credential transport regression test"
require_literal .github/workflows/collab-security.yml \
    "cargo test --locked -p op-collab-transport frame::tests" \
    "credential transport codec workflow test"
for non_clone_type in OpaqueTicket RenewTicket CollabMessage FrameEnvelope; do
    require_literal crates/op-collab/tests/credential_ownership.rs \
        "assert_not_impl_any!($non_clone_type: Clone);" \
        "credential-bearing protocol type must remain non-Clone"
done

for limit in \
    MAX_ENVELOPE_BYTES \
    MAX_TXN_BYTES \
    MAX_OPS_PER_TXN \
    MAX_DOCUMENT_NODES \
    MAX_TREE_DEPTH \
    MAX_IDENTIFIER_BYTES \
    MAX_OPAQUE_TICKET_BYTES \
    MAX_VALIDATION_NODE_VISITS_PER_TXN; do
    require_literal crates/op-collab/src/protocol.rs "$limit" "protocol hard limit"
done

for typed_config in \
    "pub struct TimeoutConfig" \
    "pub struct ConnectionLimits" \
    "pub struct RateLimitConfig" \
    "pub struct TransportConfig" \
    "pub fn validate(self) -> Result<Self, ConfigError>"; do
    require_literal crates/op-collab-transport/src/config.rs \
        "$typed_config" "transport typed configuration"
done

for transfer_limit in \
    MAX_CONTROL_TRANSFER_BYTES \
    MAX_TICKET_BYTES \
    MAX_TXN_TRANSFER_BYTES \
    MAX_SNAPSHOT_TRANSFER_BYTES; do
    require_literal crates/op-collab-transport/src/config.rs \
        "$transfer_limit" "transport hard limit"
done

for queue_type in \
    "pub(crate) struct QueueItem" \
    "pub(crate) struct BoundedTransferQueue" \
    "pub struct SharedQueueBudget" \
    "pub struct TokenBucket"; do
    require_literal crates/op-collab-transport/src/queue.rs \
        "$queue_type" "bounded queue/rate type"
done
require_literal crates/op-collab-transport/src/queue.rs \
    "pub(crate) fn sensitive_ticket_frame" "unique sensitive Ticket frame queue path"
require_literal crates/op-collab-transport/src/queue.rs \
    "pub(crate) fn sensitive_admission" "unique sensitive admission queue path"
require_literal_count crates/op-collab-transport/src/queue.rs \
    "if class == TransferClass::Ticket" 2 \
    "ordinary queue constructors reject Ticket storage"

require_literal crates/op-auth-bridge/src/collab_jwks_cache.rs \
    "pub struct CollabJwksCacheLimits" "JWKS cache typed limits"
require_literal crates/op-auth-bridge/src/collab_ticket.rs \
    "MAX_COLLAB_TICKET_BYTES" "opaque ticket hard limit"
require_literal crates/op-auth-bridge/build.rs \
    "prebuilt_provenance::validate_prebuilt" "authentication artifact integrity gate"
require_literal crates/op-auth-bridge/prebuilt_provenance.rs \
    "Sha256::digest" "authentication artifact SHA-256 verification"
require_literal crates/op-auth-bridge/prebuilt_provenance.rs \
    "verify_strict" "authentication artifact signature verification"
require_literal crates/op-auth-bridge/prebuilt_provenance.rs \
    "HARDENING_PROFILE_V1" "authentication artifact hardening profile"
require_literal .github/workflows/collab-security.yml \
    "cargo test --locked -p op-auth-bridge --test prebuilt_provenance" \
    "committed authentication matrix test"
require_literal crates/op-collab/tests/outbound_limits.rs \
    "presence_payload_limit_applies_to_encode_and_decode" \
    "outbound/inbound limit regression test"
require_literal crates/op-collab-transport/src/config.rs \
    "invalid_resource_limits_fail_closed" \
    "transport invalid-limit regression test"

for command_type in OwnerNetworkCommand GuestNetworkCommand PeerNetworkCommand; do
    require_literal crates/op-host-desktop/src/collab_runtime/types.rs \
        "assert_not_impl_any!($command_type: Clone);" \
        "renewal verification command must remain non-Clone"
done
require_literal crates/op-host-desktop/src/collab_runtime/types.rs \
    "verification_commands_move_the_original_ticket_allocation" \
    "renewal command ownership regression test"
credential_vec_copies=$(grep -RInF \
    '.expose().as_bytes().to_vec()' \
    crates/op-host-desktop/src/collab_runtime 2>/dev/null || true)
if [[ -n "$credential_vec_copies" ]]; then
    record_failure "desktop renewal commands must move OpaqueTicket instead of copying into Vec:
$credential_vec_copies"
fi

# Verified profile avatars cross an untrusted network boundary. Every redirect
# is re-resolved and pinned without proxies; both encoded and decoded sizes are
# independently capped before UI decode.
for avatar_anchor in \
    "MAX_REDIRECTS" \
    "MAX_AVATAR_ENCODED_BYTES" \
    "public_https_client" \
    "REQUEST_TIMEOUT"; do
    require_literal crates/op-host-desktop/src/collab_avatar_host.rs \
        "$avatar_anchor" "bounded collaboration avatar fetch"
done
require_literal crates/op-host-services/src/provider_dial.rs \
    ".no_proxy()" "public HTTPS proxy bypass prevention"
require_literal crates/op-host-services/src/provider_dial.rs \
    ".resolve_to_addrs" "public HTTPS DNS pinning"
require_literal crates/op-editor-ui/src/collab_avatar_runtime.rs \
    "MAX_AVATAR_SOURCE_PIXELS" "decoded avatar pixel limit"

# Public boundary failures must use domain errors, not caller-visible strings.
untyped_errors=$(grep -RInE \
    --include='*.rs' \
    'Result<[^>]*,[[:space:]]*(String|&[[:space:]]*str)[[:space:]]*>' \
    crates/op-collab/src \
    crates/op-collab-transport/src \
    crates/op-auth-bridge/src 2>/dev/null || true)
if [[ -n "$untyped_errors" ]]; then
    record_failure "untyped Result<_, String/&str> at collaboration boundaries:
$untyped_errors"
fi

# Test signing material is permitted only in the explicitly gated issuer or
# behind an outer/inner cfg(test) boundary. Integration fixtures must opt into
# the test-issuer feature at the crate root.
require_literal crates/op-auth-bridge/Cargo.toml \
    "test-issuer = []" "test issuer feature"
if ! awk '
    previous == "#[cfg(any(test, feature = \"test-issuer\"))]" \
        && $0 == "mod collab_test_issuer;" { found = 1 }
    { previous = $0 }
    END { exit(found ? 0 : 1) }
' crates/op-auth-bridge/src/lib.rs; then
    record_failure "collab_test_issuer module must remain behind test/test-issuer cfg"
fi
require_literal crates/op-auth-bridge/src/collab_test_issuer.rs \
    "public test material" "public fixture warning"
require_literal crates/op-auth-bridge/src/collab_test_issuer.rs \
    "https://collab.test.invalid" "non-production fixture issuer"
if grep -Fq "https://sso.zseven.cn" crates/op-auth-bridge/src/collab_test_issuer.rs; then
    record_failure "test issuer fixture must not contain the production issuer"
fi
require_literal crates/op-auth-bridge/src/collab_verifier.rs \
    "production_trust_root_rejects_the_public_test_issuer" \
    "production/test trust-root isolation test"

if [[ -f crates/op-auth-bridge/tests/collab_verifier.rs ]] \
    && ! sed -n '1,5p' crates/op-auth-bridge/tests/collab_verifier.rs \
        | grep -Fq '#![cfg(feature = "test-issuer")]'; then
    record_failure "auth integration fixtures must require feature = \"test-issuer\""
fi

production_fixture_hits=
while IFS= read -r source_file; do
    case "$source_file" in
        */tests/*|*/collab_test_issuer.rs)
            continue
            ;;
    esac
    hits=$(awk '
        /^[[:space:]]*#\[cfg\(test\)\][[:space:]]*$/ { exit }
        /^[[:space:]]*#!\[cfg\(test\)\][[:space:]]*$/ { exit }
        { print FNR ":" $0 }
    ' "$source_file" \
        | grep -E 'SigningKey::from_bytes|[A-Z][A-Z0-9_]*_SEED[[:space:]]*:' \
        || true)
    if [[ -n "$hits" ]]; then
        production_fixture_hits+="$source_file:
$hits
"
    fi
done < <(collab_rust_source_files)
if [[ -n "$production_fixture_hits" ]]; then
    record_failure "deterministic signing/key seed leaked into production source:
$production_fixture_hits"
fi

# High-signal committed-secret patterns. This is intentionally conservative;
# comprehensive secret scanning remains a repository-host responsibility.
sensitive_files=$(collab_boundary_files \
    | grep -E '\.(pem|key|p12|pfx|jwt|token)$' \
    || true)
if [[ -n "$sensitive_files" ]]; then
    record_failure "sensitive key/token-shaped files are forbidden:
$sensitive_files"
fi

high_signal_pattern="-----BEGIN[[:space:]]+([A-Z0-9]+[[:space:]]+)?PRIVATE[[:space:]]+KEY-----|(^|[^A-Za-z0-9])(AKIA|ASIA)[A-Z0-9]{16}([^A-Za-z0-9]|$)|(^|[^A-Za-z0-9])gh[pousr]_[A-Za-z0-9]{30,}([^A-Za-z0-9]|$)|(^|[^A-Za-z0-9])xox[baprs]-[A-Za-z0-9-]{20,}([^A-Za-z0-9]|$)|(^|[^A-Za-z0-9])sk-(proj-)?[A-Za-z0-9_-]{20,}([^A-Za-z0-9]|$)|[\"'][A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{32,}[\"']"
high_signal_hits=
while IFS= read -r scan_file; do
    hits=$(grep -InE -- "$high_signal_pattern" "$scan_file" || true)
    if [[ -n "$hits" ]]; then
        high_signal_hits+="$scan_file:
$hits
"
    fi
done < <(collab_boundary_files)
if [[ -n "$high_signal_hits" ]]; then
    record_failure "high-signal credential/private-key material detected:
$high_signal_hits"
fi

# Keep security-sensitive modules reviewable under the repository-wide cap.
while IFS= read -r source_file; do
    line_count=$(wc -l < "$source_file")
    line_count=${line_count//[[:space:]]/}
    if [[ "$line_count" -gt 800 ]]; then
        record_failure "$source_file has $line_count lines; maximum is 800"
    fi
done < <(collab_rust_source_files)

if [[ "${#failures[@]}" -ne 0 ]]; then
    printf 'check-collab-security-boundaries.sh: FAILED\n' >&2
    for failure in "${failures[@]}"; do
        printf '  - %s\n' "$failure" >&2
    done
    exit 1
fi

printf '%s\n' \
    "check-collab-security-boundaries.sh: all collaboration security boundaries pass."
