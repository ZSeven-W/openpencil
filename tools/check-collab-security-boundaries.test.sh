#!/usr/bin/env bash
# Mutation tests for check-collab-security-boundaries.sh.

set -euo pipefail

if [[ "${OPENPENCIL_COLLAB_SECURITY_FAKE_CARGO:-}" == "1" ]]; then
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
    exit 0
fi

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
        "$fixture_root/crates/op-collab-relay-protocol/src" \
        "$fixture_root/crates/op-collab-relay-client/src" \
        "$fixture_root/crates/op-collab-relay-server/src" \
        "$fixture_root/crates/op-collab-relay-control-plane/src" \
        "$fixture_root/crates/op-collab-policy-file/src" \
        "$fixture_root/crates/op-collab-relay-locator-hsm/src" \
        "$fixture_root/crates/op-collab-relay-locator-hsm/tests" \
        "$fixture_root/crates/op-collab-relay-locator-server/src" \
        "$fixture_root/crates/op-collab-smoke/src" \
        "$fixture_root/crates/op-auth-bridge/src" \
        "$fixture_root/crates/op-auth-bridge/tests" \
        "$fixture_root/crates/op-util/src" \
        "$fixture_root/crates/op-editor-core/src" \
        "$fixture_root/crates/op-editor-host-core/src/collab" \
        "$fixture_root/crates/op-editor-ui/src" \
        "$fixture_root/crates/op-host-native/src" \
        "$fixture_root/crates/op-host-desktop/src" \
        "$fixture_root/crates/op-chat-agent/src" \
        "$fixture_root/crates/op-collab-host/src/runtime/network" \
        "$fixture_root/crates/op-host-services/src" \
        "$fixture_root/crates/op-i18n/src" \
        "$fixture_root/deploy/collab-relay" \
        "$fixture_root/deploy/collab-relay-edge" \
        "$fixture_root/deploy/collab-relay-locator" \
        "$fixture_root/deploy/collab-relay-locator-hsm" \
        "$fixture_root/deploy/collab-relay-locator-edge"

    cp "$gate_source" "$fixture_root/tools/check-collab-security-boundaries.sh"
    cp "$script_dir/check-collab-security-boundaries-cases.sh" \
        "$fixture_root/tools/check-collab-security-boundaries-cases.sh"
    cp "$script_dir/check-collab-deployment-boundaries.sh" \
        "$fixture_root/tools/check-collab-deployment-boundaries.sh"
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
    cat > "$fixture_root/.dockerignore" <<'EOF'
**/relay-x25519-keys*.json
**/*private-keys*.json
**/*private_keys*.json
**/locator-signing-key*.json
EOF
    cp "$fixture_root/.dockerignore" "$fixture_root/.gitignore"

    cat > "$fixture_root/docs/security/p2p-collaboration-threat-model.md" <<'EOF'
# Fixture threat model

This file exists so the executable boundary gate can verify its public contract.
EOF

    write_collab_security_workflow_fixture

    cat > "$fixture_root/deploy/collab-relay-edge/global-nginx.conf" <<'EOF'
stream {
    access_log off;
    upstream cn_federation_listener { server 192.0.2.10:9443; }
    server {
        listen 8443;
        limit_conn global_clients 32;
        proxy_ssl on;
        proxy_ssl_verify on;
        proxy_ssl_certificate /run/secrets/global-edge-client-cert.pem;
        proxy_ssl_certificate_key /run/secrets/global-edge-client-key.pem;
        proxy_ssl_trusted_certificate /run/secrets/cn-federation-ca.pem;
        proxy_ssl_session_reuse off;
        proxy_next_upstream off;
        proxy_pass cn_federation_listener;
    }
}
EOF

    cat > "$fixture_root/deploy/collab-relay/nginx.conf" <<'EOF'
server {
    listen 443 ssl;
    client_header_buffer_size 64k;
    large_client_header_buffers 2 64k;
    location = /v1/tunnel {
        proxy_set_header Authorization $http_authorization;
        proxy_pass_header OpenPencil-Relay-Challenge;
    }
}
server {
    listen 8444 ssl;
    client_header_buffer_size 64k;
    large_client_header_buffers 2 64k;
    location = /v1/tunnel {
        limit_req zone=relay_federation_handshakes burst=500 nodelay;
        limit_conn relay_federation_connections 512;
        proxy_set_header Authorization $http_authorization;
        proxy_pass_header OpenPencil-Relay-Challenge;
    }
}
EOF
    for relay_deploy_file in \
        Dockerfile \
        README.md \
        compose.yaml \
        compose.production.yaml \
        compose.reduced-assurance.yaml; do
        : > "$fixture_root/deploy/collab-relay/$relay_deploy_file"
    done
    cat > "$fixture_root/deploy/collab-relay/Dockerfile" <<'EOF'
FROM rust:1.94-bookworm@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa AS build
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
EOF
    cat > "$fixture_root/deploy/collab-relay-edge/cn-federation-nginx.conf" <<'EOF'
stream {
    access_log off;
    upstream cn_inner_wss { server 192.0.2.20:8444; }
    server {
        listen 9443 ssl;
        ssl_verify_client on;
        ssl_client_certificate /run/secrets/global-edge-client-ca.pem;
        ssl_crl /run/secrets/global-edge-client-crl.pem;
        ssl_session_cache off;
        proxy_next_upstream off;
        proxy_pass cn_inner_wss;
    }
}
EOF
    : > "$fixture_root/deploy/collab-relay-edge/README.md"
    cat > "$fixture_root/deploy/collab-relay-edge/compose.global.yaml" <<'EOF'
ports:
  - published: 443
restart: "no"
EOF
    : > "$fixture_root/deploy/collab-relay-edge/compose.cn.yaml"
    cat > "$fixture_root/deploy/collab-relay-edge/rotate-cn-crl.sh" <<'EOF'
echo "candidate CRLNumber must be strictly greater"
echo "candidate CRL drops an existing revoked certificate serial"
echo "CRL activation requires root"
echo "CRL/CA files must be root:101 mode 0440"
OPENPENCIL_RELAY_EDGE_VALIDATION_MODE=production
# Recheck the final staged inode after ownership and mode changes.
docker compose up -d --no-deps --force-recreate cn-federation
docker compose exec -T cn-federation nginx -t
EOF
    cat > "$fixture_root/deploy/collab-relay-edge/validate.sh" <<'EOF'
#!/bin/sh
set -eu
EOF
    cat > "$fixture_root/deploy/collab-relay-edge/global-new-connection-rate.nft" <<'EOF'
ct state new meter relay_edge_new_v4 { ip saddr timeout 2m limit rate over 60/minute burst 20 packets } counter drop
EOF
    cat > "$fixture_root/deploy/collab-relay-edge/install-global-new-connection-rate.sh" <<'EOF'
#!/bin/sh
meter_name=relay_edge_new_v4
ct state new meter $meter_name { ip saddr timeout 2m limit rate over 60/minute burst 20 packets } counter drop
EOF
    cat > "$fixture_root/deploy/collab-relay-edge/verify-global-new-connection-rate.sh" <<'EOF'
#!/bin/sh
exit 0
EOF
    cat > "$fixture_root/deploy/collab-relay-edge/verify-rate-rules.py" <<'EOF'
#!/usr/bin/env python3
not address.is_global
len(expressions) != 7
"rate": 60
"burst": 20
"per": "minute"
expect_equal(expressions[6], {"drop": None}
EOF
    cat > "$fixture_root/deploy/collab-relay-edge/deploy-global.sh" <<'EOF'
#!/bin/sh
"$script_dir/install-global-new-connection-rate.sh"
OPENPENCIL_RELAY_EDGE_VALIDATION_MODE=production
--abort-on-container-exit --exit-code-from global-edge
EOF
    cat > "$fixture_root/deploy/collab-relay-edge/openpencil-collab-relay-global.service.example" <<'EOF'
Requires=docker.service nftables.service
After=network-online.target nftables.service docker.service
ExecStart=/opt/openpencil/deploy/collab-relay-edge/deploy-global.sh
EOF

    cat > "$fixture_root/deploy/collab-relay-locator/nginx-location.conf" <<'EOF'
location = /v1/locator {
    if ($request_uri != "/v1/locator") {
        return 404;
    }
    if ($http_host = "") {
        return 400;
    }
    limit_req zone=openpencil_locator_per_source burst=20 nodelay;
    limit_conn openpencil_locator_connections 16;
    client_max_body_size 191;
    proxy_set_header Authorization $http_authorization;
}
location = /v1/pairing-code {
    if ($request_uri != "/v1/pairing-code") { return 404; }
    limit_except POST { deny all; }
    client_max_body_size 624; client_body_buffer_size 624;
    if ($http_content_type != "application/vnd.openpencil.relay-pairing-publish-v1") { return 415; }
    proxy_pass_request_headers off;
    proxy_pass http://locator:8092/v1/pairing-code;
}
location = /v1/pairing-code/claim {
    if ($request_uri != "/v1/pairing-code/claim") { return 404; }
    limit_except POST { deny all; }
    client_max_body_size 49; client_body_buffer_size 49;
    if ($content_length != "49") { return 400; }
    if ($http_content_type != "application/vnd.openpencil.relay-pairing-claim-v1") { return 415; }
    if ($http_accept != "application/vnd.openpencil.relay-sealed-invite-v1") { return 406; }
    proxy_pass_request_headers off;
    proxy_pass http://locator:8092/v1/pairing-code/claim;
}
location / {
    return 404;
}
EOF
    cat > "$fixture_root/deploy/collab-relay-locator/nginx-http-limits.conf" <<'EOF'
limit_req_zone $binary_remote_addr zone=openpencil_locator_per_source:10m rate=10r/s;
limit_conn_zone $binary_remote_addr zone=openpencil_locator_connections:10m;
client_header_buffer_size 64k;
large_client_header_buffers 2 64k;
EOF
    cat > "$fixture_root/deploy/collab-relay-locator/compose.yaml" <<'EOF'
services:
  locator:
    environment:
      OPENPENCIL_COLLAB_LOCATOR_HSM_SOCKET: /run/openpencil-hsm/signer.sock
    read_only: true
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
EOF
    cat > "$fixture_root/deploy/collab-relay-locator/Dockerfile" <<'EOF'
FROM rust:1.94-bookworm@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa AS build
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
ENTRYPOINT ["/usr/local/bin/op-collab-relay-locator-server", "--production"]
EOF
    : > "$fixture_root/deploy/collab-relay-locator/README.md"
    cat > "$fixture_root/deploy/collab-relay-locator/validate.sh" <<'EOF'
#!/bin/sh
set -eu
EOF

    cat > "$fixture_root/deploy/collab-relay-locator-hsm/Dockerfile" <<'EOF'
FROM rust:1.94-bookworm@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa AS build
FROM build AS test
RUN apt-get update && apt-get install -y --no-install-recommends softhsm2
RUN cargo test --locked -p op-collab-relay-locator-hsm --test softhsm -- --nocapture
FROM debian:bookworm-slim@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
EOF
    for locator_hsm_file in \
        README.md \
        compose.yaml \
        config.example.json \
        openpencil-locator-hsm.conf \
        softhsm2.conf; do
        : > "$fixture_root/deploy/collab-relay-locator-hsm/$locator_hsm_file"
    done

    cat > "$fixture_root/deploy/collab-relay-locator-edge/global-nginx.conf" <<'EOF'
stream {
    access_log off;
    upstream cn_locator_federation { server 192.0.2.30:9543; }
    server {
        listen 8443;
        limit_conn locator_global_clients 32;
        proxy_ssl on;
        proxy_ssl_verify on;
        proxy_ssl_certificate /run/secrets/global-locator-edge-client-cert.pem;
        proxy_ssl_certificate_key /run/secrets/global-locator-edge-client-key.pem;
        proxy_ssl_trusted_certificate /run/secrets/cn-locator-federation-ca.pem;
        proxy_ssl_session_reuse off;
        proxy_next_upstream off;
        proxy_pass cn_locator_federation;
    }
}
EOF
    cat > "$fixture_root/deploy/collab-relay-locator-edge/cn-federation-nginx.conf" <<'EOF'
stream {
    access_log off;
    upstream cn_locator_inner_https { server 192.0.2.40:8445; }
    server {
        listen 9543 ssl;
        ssl_verify_client on;
        ssl_client_certificate /run/secrets/global-locator-edge-client-ca.pem;
        ssl_crl /run/secrets/global-locator-edge-client-crl.pem;
        ssl_session_cache off;
        proxy_next_upstream off;
        proxy_pass cn_locator_inner_https;
    }
}
EOF
    cat > "$fixture_root/deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf" <<'EOF'
server {
    client_header_buffer_size 64k;
    large_client_header_buffers 2 64k;
    keepalive_requests 1;
    keepalive_timeout 0;
    if ($ssl_server_name != locator.example.cn) {
        return 421;
    }
    if ($http_host != locator.example.cn) {
        return 421;
    }
    location = /v1/locator {
        if ($request_uri != "/v1/locator") {
            return 404;
        }
        limit_except POST {
            deny all;
        }
        proxy_pass_request_headers off;
        proxy_set_header Authorization $http_authorization;
        proxy_set_header Transfer-Encoding "";
        proxy_set_header Content-Encoding "";
    }
    location = /v1/pairing-code {
        if ($request_uri != "/v1/pairing-code") { return 404; }
        limit_except POST { deny all; }
        client_max_body_size 624; client_body_buffer_size 624;
        if ($http_content_type != "application/vnd.openpencil.relay-pairing-publish-v1") { return 415; }
        proxy_pass_request_headers off;
        proxy_pass http://openpencil_locator/v1/pairing-code;
    }
    location = /v1/pairing-code/claim {
        if ($request_uri != "/v1/pairing-code/claim") { return 404; }
        limit_except POST { deny all; }
        client_max_body_size 49; client_body_buffer_size 49;
        if ($content_length != "49") { return 400; }
        if ($http_content_type != "application/vnd.openpencil.relay-pairing-claim-v1") { return 415; }
        if ($http_accept != "application/vnd.openpencil.relay-sealed-invite-v1") { return 406; }
        proxy_pass_request_headers off;
        proxy_pass http://openpencil_locator/v1/pairing-code/claim;
    }
    location / { return 404; }
}
EOF
    for locator_edge_file in \
        README.md \
        compose.global.yaml \
        compose.cn.yaml \
        compose.cn-https.yaml \
        global-new-connection-rate.nft \
        verify-global-new-connection-rate.sh \
        deploy-global.sh \
        validate.sh; do
        : > "$fixture_root/deploy/collab-relay-locator-edge/$locator_edge_file"
    done
    cat > "$fixture_root/deploy/collab-relay-locator-edge/compose.global.yaml" <<'EOF'
ports:
  - published: 443
restart: "no"
EOF
    cat > "$fixture_root/deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh" <<'EOF'
ct state new meter locator_edge_new_v4 { ip saddr timeout 2m limit rate over 60/minute burst 20 packets } counter drop
EOF
    cat > "$fixture_root/deploy/collab-relay-locator-edge/deploy-global.sh" <<'EOF'
#!/bin/sh
"$script_dir/install-global-new-connection-rate.sh"
OPENPENCIL_LOCATOR_EDGE_VALIDATION_MODE=production
--abort-on-container-exit --exit-code-from global-locator-edge
EOF
    cat > "$fixture_root/deploy/collab-relay-locator-edge/openpencil-collab-locator-global.service.example" <<'EOF'
Requires=docker.service nftables.service
After=network-online.target nftables.service docker.service
ExecStart=/opt/openpencil/deploy/collab-relay-locator-edge/deploy-global.sh
EOF
    cat > "$fixture_root/deploy/collab-relay-locator-edge/rotate-cn-crl.sh" <<'EOF'
echo "candidate CRLNumber must be strictly greater"
echo "candidate CRL drops an existing revoked certificate serial"
echo "CRL activation requires root"
echo "CRL/CA files must be root:101 mode 0440"
OPENPENCIL_LOCATOR_EDGE_VALIDATION_MODE=production
# Revalidate the exact staged inode after its final ownership/mode changes.
docker compose up -d --no-deps --force-recreate
docker compose exec -T locator-cn-federation nginx -t
EOF
    chmod +x \
        "$fixture_root/deploy/collab-relay-edge/install-global-new-connection-rate.sh" \
        "$fixture_root/deploy/collab-relay-edge/verify-global-new-connection-rate.sh" \
        "$fixture_root/deploy/collab-relay-edge/verify-rate-rules.py" \
        "$fixture_root/deploy/collab-relay-edge/deploy-global.sh" \
        "$fixture_root/deploy/collab-relay-edge/rotate-cn-crl.sh" \
        "$fixture_root/deploy/collab-relay-edge/validate.sh" \
        "$fixture_root/deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh" \
        "$fixture_root/deploy/collab-relay-locator-edge/verify-global-new-connection-rate.sh" \
        "$fixture_root/deploy/collab-relay-locator-edge/deploy-global.sh" \
        "$fixture_root/deploy/collab-relay-locator-edge/rotate-cn-crl.sh" \
        "$fixture_root/deploy/collab-relay-locator-edge/validate.sh"

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
    for relay_crate in \
        op-collab-relay-protocol \
        op-collab-relay-client \
        op-collab-relay-server \
        op-collab-relay-control-plane \
        op-collab-policy-file \
        op-collab-relay-locator-hsm \
        op-collab-relay-locator-server; do
        cat > "$fixture_root/crates/$relay_crate/Cargo.toml" <<EOF
[package]
name = "$relay_crate"
version = "0.0.0"
license.workspace = true
EOF
        : > "$fixture_root/crates/$relay_crate/LICENSE"
    done
    : > "$fixture_root/crates/op-collab-relay-locator-hsm/tests/softhsm.rs"
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
pub fn from_json_slice_with_limits_for_direction(
    bytes: &[u8],
    limits: (),
    inbound_direction: InboundFrameDirection,
) {
    enforce_inbound_envelope_limit(inbound_direction, bytes.len(), limits)?;
    declared_kind_rejecting_renew_ticket(bytes)?;
    let mut value = decode_json_value(bytes, limits)?;
}
fn declared_kind_rejecting_renew_ticket(_bytes: &[u8]) -> Result<(), ()> {
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

    cat > "$fixture_root/crates/op-collab/src/frame_direction.rs" <<'EOF'
pub enum InboundFrameDirection {
    GuestToOwner,
    OwnerToGuest,
}
fn enforce_inbound_envelope_limit(
    direction: InboundFrameDirection,
    actual: usize,
    limits: WireLimits,
) {}
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
#[test]
fn oversized_snapshot_kind_cannot_raise_the_owner_inbound_ceiling() {}
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

    cat > "$fixture_root/crates/op-collab-transport/src/connection_limit_tests.rs" <<'EOF'
#[test]
fn live_silent_guards_stay_charged_until_the_socket_worker_drops_them() {}
EOF

    cat > "$fixture_root/crates/op-collab-transport/src/tcp.rs" <<'EOF'
#[test]
fn silent_guarded_accept_exits_at_first_message_deadline_before_releasing_its_seat() {}
EOF

    cat > "$fixture_root/crates/op-collab-transport/src/chunk_tests.rs" <<'EOF'
#[test]
fn completed_transfer_holds_the_declared_reservation_until_drop() {}
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

    cat > "$fixture_root/crates/op-collab-transport/src/admission.rs" <<'EOF'
pub enum PeerIdentityPolicy {
    ThisAccount,
    AnyIssuedAccount,
}
EOF

    cat > "$fixture_root/crates/op-collab-transport/src/admission_tests.rs" <<'EOF'
#[test]
fn any_issued_account_admits_a_foreign_subject_but_keeps_every_other_check() {}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache.rs" <<'EOF'
pub struct CollabJwksCacheLimits;
#[cfg(test)]
#[path = "collab_policy_cache_tests.rs"]
mod policy_tests;
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_jwks_cache_cancellation_tests.rs" <<'EOF'
#![cfg(test)]
fn deterministic_test_key(seed: u8) {
    let _ = SigningKey::from_bytes(&[seed; 32]);
}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_policy_cache_tests.rs" <<'EOF'
fn deterministic_policy_test_key(seed: u8) {
    let _ = SigningKey::from_bytes(&[seed; 32]);
}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_ticket.rs" <<'EOF'
pub const MAX_COLLAB_TICKET_BYTES: usize = 1024;
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_relay_token.rs" <<'EOF'
pub struct VerifiedRelayTokenClaims;
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
mod tests {
    #[test]
    fn production_signed_policy_path_never_falls_back_to_raw_jwks() {}
}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_union_policy.rs" <<'EOF'
#[cfg(test)]
#[path = "collab_union_policy_tests.rs"]
mod tests;
EOF

    cat > "$fixture_root/crates/op-auth-bridge/src/collab_union_policy_tests.rs" <<'EOF'
fn deterministic_union_policy_test_key(seed: u8) {
    let _ = SigningKey::from_bytes(&[seed; 32]);
}

#[test]
fn verifies_the_frozen_go_production_root_fixture() {}
EOF

    cat > "$fixture_root/crates/op-auth-bridge/tests/collab_verifier.rs" <<'EOF'
#![cfg(feature = "test-issuer")]
#[test]
fn public_fixture_is_explicitly_enabled() {}
EOF

    cat > "$fixture_root/crates/op-host-services/src/profile_avatar_fetch.rs" <<'EOF'
const MAX_REDIRECTS: usize = 3;
const REQUEST_TIMEOUT: u64 = 5;
const MAX_AVATAR_ENCODED_BYTES: usize = 1024;
fn public_https_client() {}
EOF

    cat > "$fixture_root/crates/op-host-desktop/src/collab_avatar_host.rs" <<'EOF'
fn dispatch(request: AvatarRequest) {
    if request.is_current_account() {
        fetch_account_avatar_blocking(request.url());
    } else {
        fetch_profile_avatar_blocking(request.url());
    }
}
EOF

    cat > "$fixture_root/crates/op-host-services/src/public_https_client.rs" <<'EOF'
pub fn public_https_client() {}
EOF

    cat > "$fixture_root/crates/op-chat-agent/src/provider_dial.rs" <<'EOF'
fn pinned_client(builder: reqwest::ClientBuilder, host: &str, addrs: &[SocketAddr]) {
    let _client = builder
        .no_proxy()
        .resolve_to_addrs(host, addrs)
        .build();
}
EOF

    cat > "$fixture_root/crates/op-host-services/src/provider_dial.rs" <<'EOF'
pub(crate) use op_chat_agent::provider_dial::client_for;
EOF

    cat > "$fixture_root/crates/op-host-services/src/web_credentials.rs" <<'EOF'
pub fn is_restricted_ip() -> bool { true }
EOF

    cat > "$fixture_root/crates/op-editor-ui/src/collab_avatar_runtime.rs" <<'EOF'
pub const MAX_AVATAR_SOURCE_PIXELS: u64 = 1_048_576;
EOF

    cat > "$fixture_root/crates/op-collab-host/src/runtime/types.rs" <<'EOF'
assert_not_impl_any!(OwnerNetworkCommand: Clone);
assert_not_impl_any!(GuestNetworkCommand: Clone);
assert_not_impl_any!(PeerNetworkCommand: Clone);
fn verification_commands_move_the_original_ticket_allocation() {}
EOF

    cat > "$fixture_root/crates/op-collab-host/src/runtime/relay_bootstrap_tests.rs" <<'EOF'
#[test]
fn payload_rejects_exact_cross_region_key_reuse() {}

#[cfg(test)]
fn an_unpinned_join_without_confirmation_still_requires_this_account() {}

#[cfg(test)]
fn an_unpinned_join_admits_a_foreign_account_only_behind_the_confirmation_gate() {}
EOF

    cat > "$fixture_root/crates/op-collab-host/src/runtime/network/owner.rs" <<'EOF'
fn owner_policy() {
    let _ = PeerIdentityPolicy::AnyIssuedAccount;
}
EOF

    ln -s "$script_dir/check-collab-security-boundaries.test.sh" \
        "$fixture_root/fake-bin/cargo"

}

run_gate() {
    set +e
    gate_output=$(
        cd "$fixture_root"
        PATH="$fixture_root/fake-bin:$PATH" \
            OPENPENCIL_COLLAB_SECURITY_FAKE_CARGO=1 \
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

source "$script_dir/check-collab-security-boundaries-cases.sh"
