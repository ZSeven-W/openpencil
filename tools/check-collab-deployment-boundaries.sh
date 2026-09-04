# Deployment-specific collaboration security boundaries.
# Sourced by check-collab-security-boundaries.sh after its shared assertions
# and failure accumulator have been initialized.
#
# This file is a FRAGMENT, not a runnable gate. Every assertion below is a
# function the parent defines, so running this file directly used to print
# "require_file: command not found" once per assertion and then exit 0 — a
# green result from a script that checked nothing. Anyone who ran it by hand,
# or wired it into CI as its own step, got that silent pass.
#
# The guard below makes the mistake loud. It also refuses to run when the
# parent has not initialized the failure accumulator, so a future reordering
# of the parent cannot reintroduce the same hole.

for _boundary_helper in \
    require_file \
    require_executable \
    require_literal \
    require_literal_count \
    record_failure; do
    if ! command -v "$_boundary_helper" >/dev/null 2>&1; then
        printf '%s\n' \
            "check-collab-deployment-boundaries.sh is not a standalone gate." \
            "It must be sourced by check-collab-security-boundaries.sh, which" \
            "defines the assertions it uses (missing: $_boundary_helper)." \
            "Run instead:  bash tools/check-collab-security-boundaries.sh" >&2
        exit 2
    fi
done
unset _boundary_helper

for required in \
    .dockerignore \
    .gitignore \
    Cargo.toml \
    crates/op-collab/Cargo.toml \
    crates/op-collab/LICENSE \
    crates/op-collab/src/ticket_json.rs \
    crates/op-collab/tests/credential_ownership.rs \
    crates/op-collab-transport/Cargo.toml \
    crates/op-collab-transport/LICENSE \
    crates/op-collab-relay-protocol/Cargo.toml \
    crates/op-collab-relay-protocol/LICENSE \
    crates/op-collab-relay-client/Cargo.toml \
    crates/op-collab-relay-client/LICENSE \
    crates/op-collab-relay-server/Cargo.toml \
    crates/op-collab-relay-server/LICENSE \
    crates/op-collab-relay-control-plane/Cargo.toml \
    crates/op-collab-relay-control-plane/LICENSE \
    crates/op-collab-policy-file/Cargo.toml \
    crates/op-collab-policy-file/LICENSE \
    crates/op-collab-relay-locator-hsm/Cargo.toml \
    crates/op-collab-relay-locator-hsm/LICENSE \
    crates/op-collab-relay-locator-hsm/tests/softhsm.rs \
    crates/op-collab-relay-locator-server/Cargo.toml \
    crates/op-collab-relay-locator-server/LICENSE \
    crates/op-collab-smoke/LICENSE \
    crates/op-auth-bridge/Cargo.toml \
    crates/op-auth-bridge/LICENSE \
    crates/op-auth-bridge/prebuilt_provenance.rs \
    deploy/collab-relay/Dockerfile \
    deploy/collab-relay/README.md \
    deploy/collab-relay/compose.yaml \
    deploy/collab-relay/compose.production.yaml \
    deploy/collab-relay/compose.reduced-assurance.yaml \
    deploy/collab-relay/nginx.conf \
    deploy/collab-relay-edge/README.md \
    deploy/collab-relay-edge/global-nginx.conf \
    deploy/collab-relay-edge/cn-federation-nginx.conf \
    deploy/collab-relay-edge/compose.global.yaml \
    deploy/collab-relay-edge/compose.cn.yaml \
    deploy/collab-relay-edge/global-new-connection-rate.nft \
    deploy/collab-relay-edge/install-global-new-connection-rate.sh \
    deploy/collab-relay-edge/verify-global-new-connection-rate.sh \
    deploy/collab-relay-edge/verify-rate-rules.py \
    deploy/collab-relay-edge/deploy-global.sh \
    deploy/collab-relay-edge/openpencil-collab-relay-global.service.example \
    deploy/collab-relay-edge/rotate-cn-crl.sh \
    deploy/collab-relay-edge/validate.sh \
    deploy/collab-relay-locator/Dockerfile \
    deploy/collab-relay-locator/README.md \
    deploy/collab-relay-locator/compose.yaml \
    deploy/collab-relay-locator/nginx-http-limits.conf \
    deploy/collab-relay-locator/nginx-location.conf \
    deploy/collab-relay-locator/validate.sh \
    deploy/collab-relay-locator-hsm/Dockerfile \
    deploy/collab-relay-locator-hsm/README.md \
    deploy/collab-relay-locator-hsm/compose.yaml \
    deploy/collab-relay-locator-hsm/config.example.json \
    deploy/collab-relay-locator-hsm/openpencil-locator-hsm.conf \
    deploy/collab-relay-locator-hsm/softhsm2.conf \
    deploy/collab-relay-locator-edge/README.md \
    deploy/collab-relay-locator-edge/global-nginx.conf \
    deploy/collab-relay-locator-edge/cn-federation-nginx.conf \
    deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf \
    deploy/collab-relay-locator-edge/compose.global.yaml \
    deploy/collab-relay-locator-edge/compose.cn.yaml \
    deploy/collab-relay-locator-edge/compose.cn-https.yaml \
    deploy/collab-relay-locator-edge/global-new-connection-rate.nft \
    deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh \
    deploy/collab-relay-locator-edge/verify-global-new-connection-rate.sh \
    deploy/collab-relay-locator-edge/deploy-global.sh \
    deploy/collab-relay-locator-edge/openpencil-collab-locator-global.service.example \
    deploy/collab-relay-locator-edge/rotate-cn-crl.sh \
    deploy/collab-relay-locator-edge/validate.sh \
    tools/check-collab-security-boundaries-cases.sh \
    tools/check-collab-deployment-boundaries.sh \
    .github/workflows/collab-security.yml \
    docs/security/p2p-collaboration-threat-model.md; do
    require_file "$required"
done

for executable_deploy_file in \
    deploy/collab-relay-edge/install-global-new-connection-rate.sh \
    deploy/collab-relay-edge/verify-global-new-connection-rate.sh \
    deploy/collab-relay-edge/verify-rate-rules.py \
    deploy/collab-relay-edge/deploy-global.sh \
    deploy/collab-relay-edge/rotate-cn-crl.sh \
    deploy/collab-relay-edge/validate.sh \
    deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh \
    deploy/collab-relay-locator-edge/verify-global-new-connection-rate.sh \
    deploy/collab-relay-locator-edge/deploy-global.sh \
    deploy/collab-relay-locator-edge/rotate-cn-crl.sh \
    deploy/collab-relay-locator-edge/validate.sh; do
    require_executable "$executable_deploy_file" "deployment gate executable boundary"
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
    ".dockerignore" \
    ".gitignore" \
    "crates/op-collab-smoke/**" \
    "crates/op-collab-relay-protocol/**" \
    "crates/op-collab-relay-client/**" \
    "crates/op-collab-relay-server/**" \
    "crates/op-collab-relay-control-plane/**" \
    "crates/op-collab-policy-file/**" \
    "crates/op-collab-relay-locator-hsm/**" \
    "crates/op-collab-relay-locator-server/**" \
    "crates/op-util/**" \
    "crates/op-editor-core/**" \
    "crates/op-editor-host-core/**" \
    "crates/op-editor-ui/**" \
    "crates/op-host-native/**" \
    "crates/op-host-desktop/**" \
    "crates/op-chat-agent/src/provider_dial.rs" \
    "crates/op-host-services/**" \
    "crates/op-i18n/**" \
    "deploy/collab-relay/**" \
    "deploy/collab-relay-edge/**" \
    "deploy/collab-relay-locator/**" \
    "deploy/collab-relay-locator-hsm/**" \
    "deploy/collab-relay-locator-edge/**" \
    "tools/check-collab-security-boundaries-cases.sh" \
    "tools/check-collab-deployment-boundaries.sh"; do
    require_literal_count .github/workflows/collab-security.yml \
        "$workflow_path" 2 "collaboration security workflow path trigger"
done

for edge_anchor in \
    "listen 8443;" \
    "limit_conn global_clients 32;" \
    "proxy_ssl on;" \
    "proxy_ssl_verify on;" \
    "proxy_ssl_certificate /run/secrets/global-edge-client-cert.pem;" \
    "proxy_ssl_certificate_key /run/secrets/global-edge-client-key.pem;" \
    "proxy_ssl_trusted_certificate /run/secrets/cn-federation-ca.pem;" \
    "proxy_ssl_session_reuse off;" \
    "proxy_next_upstream off;" \
    "access_log off;"; do
    require_literal deploy/collab-relay-edge/global-nginx.conf \
        "$edge_anchor" "Global-to-CN inner-TLS passthrough boundary"
done
for cn_federation_anchor in \
    "listen 9443 ssl;" \
    "ssl_verify_client on;" \
    "ssl_client_certificate /run/secrets/global-edge-client-ca.pem;" \
    "ssl_crl /run/secrets/global-edge-client-crl.pem;" \
    "ssl_session_cache off;" \
    "proxy_next_upstream off;" \
    "access_log off;"; do
    require_literal deploy/collab-relay-edge/cn-federation-nginx.conf \
        "$cn_federation_anchor" "CN outer-mTLS federation boundary"
done
require_literal .github/workflows/collab-security.yml \
    "bash deploy/collab-relay-edge/validate.sh" \
    "nested-TLS relay edge workflow validation"
for relay_edge_rate_anchor in \
    "ct state new" \
    "meter_name=relay_edge_new_v4" \
    'meter $meter_name' \
    "ip saddr timeout 2m" \
    "limit rate over 60/minute burst 20 packets" \
    "counter drop"; do
    require_literal deploy/collab-relay-edge/install-global-new-connection-rate.sh \
        "$relay_edge_rate_anchor" "overseas relay per-source connection-rate boundary"
done
for relay_edge_supervision_anchor in \
    'published: 443' \
    'restart: "no"'; do
    require_literal deploy/collab-relay-edge/compose.global.yaml \
        "$relay_edge_supervision_anchor" "overseas relay supervised fixed-port boundary"
done
for relay_edge_deploy_anchor in \
    '"$script_dir/install-global-new-connection-rate.sh"' \
    'OPENPENCIL_RELAY_EDGE_VALIDATION_MODE=production' \
    '--abort-on-container-exit --exit-code-from global-edge'; do
    require_literal deploy/collab-relay-edge/deploy-global.sh \
        "$relay_edge_deploy_anchor" "overseas relay supervised startup boundary"
done
for relay_edge_service_anchor in \
    'Requires=docker.service nftables.service' \
    'After=network-online.target nftables.service docker.service' \
    'ExecStart=/opt/openpencil/deploy/collab-relay-edge/deploy-global.sh'; do
    require_literal deploy/collab-relay-edge/openpencil-collab-relay-global.service.example \
        "$relay_edge_service_anchor" "overseas relay boot-order boundary"
done
for relay_edge_rotation_anchor in \
    "candidate CRLNumber must be strictly greater" \
    "candidate CRL drops an existing revoked certificate serial" \
    "CRL activation requires root" \
    "CRL/CA files must be root:101 mode 0440" \
    "OPENPENCIL_RELAY_EDGE_VALIDATION_MODE=production" \
    "Recheck the final staged inode after ownership and mode changes." \
    "up -d --no-deps --force-recreate cn-federation" \
    "exec -T cn-federation nginx -t"; do
    require_literal deploy/collab-relay-edge/rotate-cn-crl.sh \
        "$relay_edge_rotation_anchor" "relay federation CRL activation boundary"
done
for exact_rate_verifier_anchor in \
    'not address.is_global' \
    'len(expressions) != 7' \
    '"rate": 60' \
    '"burst": 20' \
    '"per": "minute"' \
    'expect_equal(expressions[6], {"drop": None}'; do
    require_literal deploy/collab-relay-edge/verify-rate-rules.py \
        "$exact_rate_verifier_anchor" "kernel nftables semantic verification boundary"
done
require_literal_count deploy/collab-relay/nginx.conf \
    "client_header_buffer_size 64k;" 2 \
    "48 KiB relay bearer ingress header boundary"
require_literal_count deploy/collab-relay/nginx.conf \
    "large_client_header_buffers 2 64k;" 2 \
    "48 KiB relay bearer ingress header boundary"
for docker_secret_pattern in \
    '**/relay-x25519-keys*.json' \
    '**/*private-keys*.json' \
    '**/*private_keys*.json' \
    '**/locator-signing-key*.json'; do
    require_literal .dockerignore "$docker_secret_pattern" \
        "Docker build-context private-key exclusion"
done
for git_secret_pattern in \
    '**/relay-x25519-keys*.json' \
    '**/*private-keys*.json' \
    '**/*private_keys*.json' \
    '**/locator-signing-key*.json'; do
    require_literal .gitignore "$git_secret_pattern" \
        "Git private-key exclusion"
done

for cn_wss_anchor in \
    "location = /v1/tunnel {" \
    'proxy_set_header Authorization $http_authorization;' \
    "proxy_pass_header OpenPencil-Relay-Challenge;" \
    "listen 8444 ssl;" \
    "limit_req zone=relay_federation_handshakes burst=500 nodelay;" \
    "limit_conn relay_federation_connections 512;"; do
    require_literal deploy/collab-relay/nginx.conf \
        "$cn_wss_anchor" "CN WSS/federation ingress boundary"
done

for locator_ingress_anchor in \
    "location = /v1/locator {" \
    'if ($request_uri != "/v1/locator") {' \
    "location = /v1/pairing-code {" \
    'if ($request_uri != "/v1/pairing-code") {' \
    "client_max_body_size 624;" \
    "client_body_buffer_size 624;" \
    'application/vnd.openpencil.relay-pairing-publish-v1' \
    "proxy_pass http://locator:8092/v1/pairing-code;" \
    "location = /v1/pairing-code/claim {" \
    'if ($request_uri != "/v1/pairing-code/claim") {' \
    "client_max_body_size 49;" \
    "client_body_buffer_size 49;" \
    'if ($content_length != "49") {' \
    'application/vnd.openpencil.relay-pairing-claim-v1' \
    'application/vnd.openpencil.relay-sealed-invite-v1' \
    "proxy_pass http://locator:8092/v1/pairing-code/claim;" \
    'if ($http_host = "") {' \
    "client_max_body_size 191;" \
    'proxy_set_header Authorization $http_authorization;' \
    "location / {" \
    "return 404;"; do
    require_literal deploy/collab-relay-locator/nginx-location.conf \
        "$locator_ingress_anchor" "locator exact-route ingress boundary"
done
require_literal_count deploy/collab-relay-locator/nginx-location.conf \
    'limit_except POST {' 2 \
    "locator pairing ingress POST-only boundary"
require_literal_count deploy/collab-relay-locator/nginx-location.conf \
    'proxy_pass_request_headers off;' 2 \
    "locator pairing ingress header isolation boundary"
for locator_limit_anchor in \
    'limit_req_zone $binary_remote_addr zone=openpencil_locator_per_source:10m rate=10r/s;' \
    'limit_conn_zone $binary_remote_addr zone=openpencil_locator_connections:10m;' \
    'large_client_header_buffers 2 64k;'; do
    require_literal deploy/collab-relay-locator/nginx-http-limits.conf \
        "$locator_limit_anchor" "locator per-source ingress boundary"
done
for locator_location_limit_anchor in \
    "limit_req zone=openpencil_locator_per_source burst=20 nodelay;" \
    "limit_conn openpencil_locator_connections 16;"; do
    require_literal deploy/collab-relay-locator/nginx-location.conf \
        "$locator_location_limit_anchor" "locator per-source ingress boundary"
done
for locator_container_anchor in \
    "OPENPENCIL_COLLAB_LOCATOR_HSM_SOCKET: /run/openpencil-hsm/signer.sock" \
    "read_only: true" \
    "cap_drop:" \
    "no-new-privileges:true"; do
    require_literal deploy/collab-relay-locator/compose.yaml \
        "$locator_container_anchor" "locator production container boundary"
done
require_literal deploy/collab-relay-locator/Dockerfile \
    'ENTRYPOINT ["/usr/local/bin/op-collab-relay-locator-server", "--production"]' \
    "locator production-only image entrypoint"
for relay_dockerfile in \
    deploy/collab-relay/Dockerfile \
    deploy/collab-relay-locator/Dockerfile; do
    if ! grep -Eq \
        '^FROM rust:[^[:space:]@]+@sha256:[0-9a-f]{64}([[:space:]]+AS[[:space:]]+build)?$' \
        "$relay_dockerfile" ||
        ! grep -Eq \
            '^FROM gcr\.io/distroless/cc-debian12:nonroot@sha256:[0-9a-f]{64}$' \
            "$relay_dockerfile"
    then
        record_failure \
            "relay container base images must use reviewed immutable SHA-256 digests: $relay_dockerfile"
    fi
done
require_literal .github/workflows/collab-security.yml \
    "bash deploy/collab-relay-locator/validate.sh" \
    "locator deployment workflow validation"
require_literal .github/workflows/collab-security.yml \
    "cargo test --locked -p op-collab-relay-locator-hsm" \
    "locator HSM crate workflow test"
require_literal .github/workflows/collab-security.yml \
    "docker build --target test" \
    "real SoftHSM workflow test target"
require_literal .github/workflows/collab-security.yml \
    "-f deploy/collab-relay-locator-hsm/Dockerfile ." \
    "real SoftHSM workflow test target"
for locator_hsm_test_anchor in \
    "FROM build AS test" \
    "apt-get install -y --no-install-recommends softhsm2" \
    "cargo test --locked -p op-collab-relay-locator-hsm --test softhsm -- --nocapture"; do
    require_literal deploy/collab-relay-locator-hsm/Dockerfile \
        "$locator_hsm_test_anchor" "real SoftHSM image test boundary"
done

for locator_edge_global_anchor in \
    "listen 8443;" \
    "limit_conn locator_global_clients 32;" \
    "proxy_ssl on;" \
    "proxy_ssl_verify on;" \
    "proxy_ssl_certificate /run/secrets/global-locator-edge-client-cert.pem;" \
    "proxy_ssl_certificate_key /run/secrets/global-locator-edge-client-key.pem;" \
    "proxy_ssl_trusted_certificate /run/secrets/cn-locator-federation-ca.pem;" \
    "proxy_ssl_session_reuse off;" \
    "proxy_next_upstream off;" \
    "access_log off;"; do
    require_literal deploy/collab-relay-locator-edge/global-nginx.conf \
        "$locator_edge_global_anchor" "overseas locator inner-TLS passthrough boundary"
done
for locator_edge_cn_anchor in \
    "listen 9543 ssl;" \
    "ssl_verify_client on;" \
    "ssl_client_certificate /run/secrets/global-locator-edge-client-ca.pem;" \
    "ssl_crl /run/secrets/global-locator-edge-client-crl.pem;" \
    "ssl_session_cache off;" \
    "proxy_next_upstream off;" \
    "access_log off;"; do
    require_literal deploy/collab-relay-locator-edge/cn-federation-nginx.conf \
        "$locator_edge_cn_anchor" "CN locator outer-mTLS federation boundary"
done
for locator_edge_https_anchor in \
    'if ($ssl_server_name != locator.example.cn) {' \
    'if ($http_host != locator.example.cn) {' \
    'if ($request_uri != "/v1/locator") {' \
    'if ($request_uri != "/v1/pairing-code") {' \
    'if ($request_uri != "/v1/pairing-code/claim") {' \
    'location = /v1/pairing-code {' \
    'location = /v1/pairing-code/claim {' \
    'client_max_body_size 624;' \
    'client_max_body_size 49;' \
    'client_body_buffer_size 624;' \
    'client_body_buffer_size 49;' \
    'if ($content_length != "49") {' \
    'application/vnd.openpencil.relay-pairing-publish-v1' \
    'application/vnd.openpencil.relay-pairing-claim-v1' \
    'application/vnd.openpencil.relay-sealed-invite-v1' \
    'proxy_pass http://openpencil_locator/v1/pairing-code;' \
    'proxy_pass http://openpencil_locator/v1/pairing-code/claim;' \
    'keepalive_requests 1;' \
    'keepalive_timeout 0;' \
    'client_header_buffer_size 64k;' \
    'large_client_header_buffers 2 64k;' \
    'proxy_pass_request_headers off;' \
    'proxy_set_header Authorization $http_authorization;' \
    'proxy_set_header Transfer-Encoding "";' \
    'proxy_set_header Content-Encoding "";'; do
    require_literal deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf \
        "$locator_edge_https_anchor" "CN locator exact inner-HTTPS boundary"
done
require_literal_count deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf \
    'limit_except POST {' 3 \
    "CN locator exact inner-HTTPS POST-only boundary"
require_literal_count deploy/collab-relay-locator-edge/cn-locator-https-nginx.conf \
    'proxy_pass_request_headers off;' 3 \
    "CN locator exact inner-HTTPS header isolation boundary"
for locator_edge_rate_anchor in \
    "ct state new" \
    "meter locator_edge_new_v4" \
    "ip saddr timeout 2m" \
    "limit rate over 60/minute burst 20 packets" \
    "counter drop"; do
    require_literal deploy/collab-relay-locator-edge/install-global-new-connection-rate.sh \
        "$locator_edge_rate_anchor" "overseas locator per-source connection-rate boundary"
done
for locator_edge_supervision_anchor in \
    'published: 443' \
    'restart: "no"'; do
    require_literal deploy/collab-relay-locator-edge/compose.global.yaml \
        "$locator_edge_supervision_anchor" "overseas locator supervised fixed-port boundary"
done
for locator_edge_deploy_anchor in \
    '"$script_dir/install-global-new-connection-rate.sh"' \
    'OPENPENCIL_LOCATOR_EDGE_VALIDATION_MODE=production' \
    '--abort-on-container-exit --exit-code-from global-locator-edge'; do
    require_literal deploy/collab-relay-locator-edge/deploy-global.sh \
        "$locator_edge_deploy_anchor" "overseas locator supervised startup boundary"
done
for locator_edge_service_anchor in \
    'Requires=docker.service nftables.service' \
    'After=network-online.target nftables.service docker.service' \
    'ExecStart=/opt/openpencil/deploy/collab-relay-locator-edge/deploy-global.sh'; do
    require_literal \
        deploy/collab-relay-locator-edge/openpencil-collab-locator-global.service.example \
        "$locator_edge_service_anchor" "overseas locator boot-order boundary"
done
for locator_edge_rotation_anchor in \
    "candidate CRLNumber must be strictly greater" \
    "candidate CRL drops an existing revoked certificate serial" \
    "CRL activation requires root" \
    "CRL/CA files must be root:101 mode 0440" \
    "OPENPENCIL_LOCATOR_EDGE_VALIDATION_MODE=production" \
    "Revalidate the exact staged inode after its final ownership/mode changes." \
    "up -d --no-deps --force-recreate" \
    "exec -T locator-cn-federation nginx -t"; do
    require_literal deploy/collab-relay-locator-edge/rotate-cn-crl.sh \
        "$locator_edge_rotation_anchor" "locator federation CRL activation boundary"
done
require_literal .github/workflows/collab-security.yml \
    "bash deploy/collab-relay-locator-edge/validate.sh" \
    "nested-TLS locator edge workflow validation"
