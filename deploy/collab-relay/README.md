# OpenPencil collaboration relay

The relay is an untrusted, blind WebSocket byte forwarder. TLS terminates at
the edge and the Rust process listens on `127.0.0.1:8091` by default. The
existing Noise session still encrypts document traffic end to end, so the
relay never receives plaintext document operations.

Only `GET /v1/tunnel` upgrades are accepted. Routing uses a domain-separated
BLAKE3 map key; capabilities, tickets, challenges, proofs, and payloads are
never logged.

## Production authentication

The standalone binary fails closed unless an explicit mode is selected.
`--production` enables signed-ticket, signed-locator, region, owner-key, and
challenge-bound X25519 proof verification.

For every successful authenticated WebSocket upgrade the relay sends exactly
one fresh:

```text
OpenPencil-Relay-Challenge: oprc1_<canonical-base64url>
```

Nginx must preserve this `101` response header.
`nginx.conf` sets `proxy_pass_header OpenPencil-Relay-Challenge` explicitly.
Strict clients reject a missing, duplicate, malformed, unknown-key, or
noncanonical challenge before sending a hello.

The checked-in public and federation TLS servers use bounded 64 KiB client
header buffers so the valid 48 KiB ticket ceiling fits in `Authorization`
without making request headers unbounded.

The challenge contains a pinned relay key id and a fresh 32-byte nonce. The
client retains the exact bearer used on that upgrade, performs X25519 with its
existing device private key and the pinned relay public key, derives a key with
HKDF-SHA256, and sends a fixed 33-byte HMAC-SHA256 proof in authentication mode
2. The proof binds:

- the complete challenge and protocol versions;
- SHA-256 of the exact bearer-token bytes;
- role and caller device public key;
- route capability and complete signed locator.

The per-connection challenge is non-cloneable server state, expires with the
bounded hello deadline, and is consumed by the first authentication attempt.
A proof replayed on another connection, used with a refreshed bearer, or moved
to another role/route/locator fails as `AuthenticationFailed`.

`RelayServerX25519ProofBoundary` is the HSM-capable verification boundary.
An HSM adapter can retain the private key and derived secret internally. The
packaged CLI also provides a sealed-file adapter for deployments without that
adapter; no private key belongs in source control or an image layer.

Blocking policy, signature, and proof operations run outside Tokio workers
behind `OPENPENCIL_COLLAB_RELAY_MAX_AUTH_IN_FLIGHT`. Its permit remains held
through challenge issuance, WebSocket upgrade, hello receipt, and actual
verification.

## Required settings

- `OPENPENCIL_COLLAB_RELAY_HOME_REGION=cn|global`
- `OPENPENCIL_COLLAB_RELAY_TICKET_POLICY_FILE`: absolute path to the pinned
  signed union-policy JSON
- `OPENPENCIL_COLLAB_RELAY_LOCATOR_KEYS_FILE`: absolute path to locator
  Ed25519 public keys
- `OPENPENCIL_COLLAB_RELAY_X25519_KEYS_FILE`: absolute path to the sealed relay
  X25519 key set
- optional `OPENPENCIL_COLLAB_RELAY_POLICY_MAX_AGE_SECONDS` (`1..=3600`,
  default `60`)
- optional `OPENPENCIL_COLLAB_RELAY_LOG_LEVEL=error|warn|info|debug` (default
  `info`); arbitrary `RUST_LOG` dependency filters and `trace` are not accepted
- optional `OPENPENCIL_COLLAB_RELAY_WAITING_TIMEOUT_SECS` (`55..=900`, default
  `60`): how long an un-paired peer may sit in the waiting queue before the
  relay rejects it with `PairingTimeout`. With the waiting lease on (the
  default) this is the length of **one lease**, not a countdown from
  registration. Both bounds come from
  `op_collab_relay_protocol::pairing_window`, shared with the client, and the
  relay refuses to start outside them rather than silently inverting the
  pairing contract described below.
- optional `OPENPENCIL_COLLAB_RELAY_WAITING_LEASE` (`0` or `1`, default `1`):
  renew an un-paired peer's waiting slot for as long as it answers the relay's
  WebSocket pings. Set it to `0` to restore the pre-lease fixed countdown.

## The pairing contract, and the waiting lease

An un-paired peer holds a slot in the relay's waiting queue. Two clocks decide
when that slot is released, and **the client's must expire first**:

- the relay's `waiting_timeout`, after which it rejects with `PairingTimeout`;
- the owner client's lane recycle budget,
  `op_collab_relay_client::RelayLimits::owner_pair` (45 s).

If the relay wins that race, the owner lane learns about the close late — or,
behind a NAT that silently reaps the idle flow, not at all — and the owner is
absent from the queue for the length of a re-dial. A guest that joins in that
window finds no counterpart, which is how a room becomes unjoinable while both
ends believe they are healthy.

Making the client always re-dial first is a mitigation, not a fix: an idle
four-lane room re-dials roughly five times a minute forever. The **waiting
lease** removes the cause. The relay pings each un-paired peer every
`min(waiting_timeout, idle_timeout) / 3`, and the peer's pong restarts the
waiting countdown. A live owner therefore just stays in the queue. The
renewal is bounded by the connection's authentication deadline and
`tunnel_lifetime` (`RelayAuthState::effective_deadline`), so a lease can never
outlive the credential that opened it; `idle_timeout` still reaps a peer that
stops answering, and `max_waiting_per_route` still bounds abuse.

### Mixed-fleet migration

**No client change is required, and none is gated on.** Every OpenPencil relay
client answers a WebSocket ping while it is reading — that is how tungstenite
behaves — so an already-shipped client renews its lease without knowing the
lease exists. Enabling the lease makes old clients *more* correct: their
five-minute pair budget stops being cut short at 60 s.

Newer clients additionally read an optional capability header on the upgrade
response, `openpencil-relay-waiting`, e.g.:

```text
openpencil-relay-waiting: oprw1 window=43200 renew=1
```

A client never adopts that value verbatim. It computes
`min(advertised - safety_margin, local_cap)`, where `local_cap` widens from the
45 s recycle budget to the ordinary pair window only when `renew=1`. A relay
that omits the header, and an intermediary that strips it, both leave the
client on its compiled-in fallback. The shared constants remain the fallback,
never the contract.

Rollout order therefore does not matter, and the three mixed states are all
safe:

| Relay | Client | Behaviour |
|---|---|---|
| lease on | new | Lane parks for the full pair window; ~0.8 dials/min per idle 4-lane room. |
| lease on | old | Lane keeps its own 5-minute budget and is no longer cut short at 60 s. |
| lease off | new | Header says `renew=0`; lane stays on the 45 s recycle budget. |
| lease off | old | Unchanged from today. |

The one operator action that is *not* safe is lowering
`OPENPENCIL_COLLAB_RELAY_WAITING_TIMEOUT_SECS` below the validated floor; the
relay refuses to start rather than accept it.

## Operational logging

At the default `info` level the relay emits one `relay_event=census` line per
minute — and none at all while idle — carrying the last window's counters plus
the live waiting-queue depth:

```text
INFO op_collab_relay_server::observe: relay census relay_event="census"
  window_secs=60 accepted=6 admission_refused=0 handshake_failed=1
  registered=5 paired=4 rejected_pairing_timeout=1 rejected_authentication=0
  rejected_capacity=0 rejected_other=0 closed_orderly=3 closed_peer_eof=1
  closed_peer_reset=0 waiting_owners=4 waiting_guests=0 waiting_routes=1
  max_route_queue_depth=4 routes_at_waiting_capacity=0 active_pairs=2
```

Read it as three questions. *Is anyone arriving?* `accepted`,
`admission_refused`, `handshake_failed`. *Is pairing working?* `registered`
versus `paired`, and which `rejected_*` bucket is moving. *Is the queue the
right shape?* `waiting_owners` versus `waiting_guests` separates "owners are
missing" from "guests are piling up", and `max_route_queue_depth` /
`routes_at_waiting_capacity` catch one wedged route while the relay as a whole
looks healthy. `closed_orderly` is kept apart from `closed_peer_eof` and
`closed_peer_reset` so a relay retiring lanes on schedule is never confused
with an intermediary severing tunnels.

Setting `OPENPENCIL_COLLAB_RELAY_LOG_LEVEL=debug` adds one line per connection
lifecycle transition (`accepted` / `handshake-failed` / `registered` /
`paired` / `rejected` / `closed`), so no connection can disappear without a
reason:

```text
DEBUG op_collab_relay_server::observe: relay rejected a peer
  relay_event="rejected" conn=1 role=owner route_ref=3d6d1666d4868b6f0aa50f02
  reason="pairing-timeout" elapsed_ms=123
```

`route_ref` is a one-way 24-hex digest of the route map key, mirroring the SSO
backend's `security_event=` reference shape. No token, invite code, session
key, bearer, or peer address is ever logged.

Locator public keys use:

```json
{
  "version": 1,
  "keys": [
    {
      "kid": "locator-key-2026-07",
      "public_key_ed25519": "canonical-base64url-no-padding"
    }
  ]
}
```

The sealed relay X25519 file uses:

```json
{
  "version": 1,
  "active_kid": "relay-pop-2026-07",
  "keys": [
    {
      "kid": "relay-pop-2026-07",
      "private_key_x25519": "canonical-base64url-no-padding",
      "public_key_x25519": "canonical-base64url-no-padding"
    }
  ]
}
```

The public value and key id are distributed to clients as pinned relay keys.
On Unix, the sealed file must be a regular non-symlink file with no group or
other permission bits. Reads are bounded and use `O_NOFOLLOW | O_CLOEXEC`.
Private encodings, shared secrets, and derived keys are zeroized.

## Deployment

On Unix, the public signed-policy file may be owned either by the relay's
effective UID or by root. It must be a regular non-symlink file and must not be
group- or world-writable. Because the policy contains public verification
material, root-owned mode `0440` (when the container identity has group read)
or `0444` is accepted; writable modes fail closed.

Run full challenge-bound production mode:

```sh
export OPENPENCIL_COLLAB_POLICY_HOST_FILE=/secure/public/collab-policy.json
export OPENPENCIL_RELAY_LOCATOR_KEYS_HOST_FILE=/secure/public/locator-keys.json
export OPENPENCIL_RELAY_X25519_KEYS_HOST_FILE=/secure/private/relay-x25519-keys.json
sudo chown 65532:65532 "$OPENPENCIL_RELAY_X25519_KEYS_HOST_FILE"
sudo chmod 0400 "$OPENPENCIL_RELAY_X25519_KEYS_HOST_FILE"
docker compose \
  -f deploy/collab-relay/compose.yaml \
  -f deploy/collab-relay/compose.production.yaml \
  -f deploy/collab-relay/compose.production.global.yaml \
  up --build
```

That command is for Global. On CN, replace only the final overlay with
`compose.production.cn.yaml`. Never combine the two regional overlays.

The secret is mounted read-only under `/run/secrets`. The packaged image runs
as UID/GID `65532`, so the host file must actually be owner-readable by that
identity and inaccessible to group/other users. Docker Compose silently
ignores `uid`, `gid`, and `mode` on file-backed secrets because they are bind
mounts; the overlay therefore does not claim those ineffective attributes.
Verify the effective ownership, mode, and readability on the target Linux
host before starting production. A key-read or permission failure must stop
the relay. Prefer a platform-managed secret or an HSM-backed
`RelayServerX25519ProofBoundary` for a long-lived Internet deployment.

Both Dockerfile base images are pinned by multi-architecture manifest digest,
verified from their upstream registries on 2026-07-29. For an update, resolve
the intended Rust and distroless tags from the upstream registries, review the
new manifests/platforms and security delta, replace both tag-plus-digest
`FROM` values, rebuild with `--pull --no-cache`, and rerun the deployment and
workspace security gates.

Clients not yet wired for authentication mode 2 may use only the separate,
explicit reduced-assurance overlay:

```sh
export OPENPENCIL_COLLAB_POLICY_HOST_FILE=/secure/public/collab-policy.json
export OPENPENCIL_RELAY_LOCATOR_KEYS_HOST_FILE=/secure/public/locator-keys.json
docker compose \
  -f deploy/collab-relay/compose.yaml \
  -f deploy/collab-relay/compose.reduced-assurance.yaml \
  up --build
```

That mode still verifies ticket-to-device-DH binding, locator signature,
region, expiry, and Owner static binding, but has no challenge proof. It
accepts only authentication mode 1 without proof and never becomes the
default.

For local capability-only development:

```sh
docker compose \
  -f deploy/collab-relay/compose.yaml \
  -f deploy/collab-relay/compose.dev.yaml \
  up --build
```

Never expose that development mode to the Internet.

## Direct regional public paths

The default two-region deployment uses one public application host per region,
with both collaboration paths on that host:

```text
CN       https://<cn-public-host>/v1/locator
         wss://<cn-public-host>/v1/tunnel
Global   https://<global-public-host>/v1/locator
         wss://<global-public-host>/v1/tunnel
```

Keep the concrete public hosts in the private deployment inventory and signed
desktop bootstrap, not in this public repository. The two Compose projects use
separate networks, so host Nginx must never use Docker service names such as
`relay` or `locator` as upstreams. The common Compose files publish no host
ports. Immutable regional overlays bind both the home region and the only
permitted host address; there is no host-bind variable or wildcard default.
They also fix `OPENPENCIL_COLLAB_RELAY_MAX_PENDING_PER_SOURCE=1024`, equal to
the process-wide pending ceiling. The common Compose files do not set this
production override.

That equality is intentional. A connection crossing a Docker-published host
port reaches the relay with a Docker NAT peer address, so independent Internet
clients can collapse into one application-visible source. The relay must not
apply its normal per-source bucket to that shared address. Host Nginx is the
trusted real-source boundary: its handshake-rate and connection zones enforce
per-client limits before proxying, while the relay still enforces the global
1024-pending ceiling. If another load balancer precedes Nginx, restore the
client address only from its fixed trusted addresses; never trust arbitrary
forwarded-address headers.

On the Global application host, use only:

```text
deploy/collab-relay/compose.production.global.yaml
deploy/collab-relay-locator/compose.production.global.yaml
```

They hard-code `home_region=global` and loopback ports `127.0.0.1:8091` and
`127.0.0.1:8092`. At Global Nginx `http` scope include
`../collab-relay-locator/nginx-http-limits.conf`,
`nginx-http-direct.conf`, and
`../collab-relay-locator/nginx-http-direct.conf` exactly once.

On the CN application service host, use only:

```text
deploy/collab-relay/compose.production.cn.yaml
deploy/collab-relay-locator/compose.production.cn.yaml
```

They hard-code `home_region=cn` and private ports `10.0.0.10:8091` and
`10.0.0.10:8092`. The CN front gateway includes the same locator limits plus
`nginx-http-direct-cn-gateway.conf` and
`../collab-relay-locator/nginx-http-direct-cn-gateway.conf` at `http` scope.
Those upstreams are explicitly `10.0.0.10:8091` and `10.0.0.10:8092`.
Restrict both ports on the service-host firewall to the configured front
gateway source addresses. Never bind either port to `0.0.0.0` or expose it on
a public interface. Do not copy a regional overlay to the other region, combine
the two overlays, or add a Compose `ports` override.

Inside each regional application TLS virtual host include
`../collab-relay-locator/nginx-location-direct.conf` and
`nginx-location.conf`. These files intentionally define only the two exact
collaboration paths, so they do not replace the application's normal routes.
The standalone relay `nginx.conf` and dedicated-host locator
`nginx-location.conf` must not be combined with the direct-host snippets.

A Global user who explicitly selects CN uses the CN application host directly
for both paths. The signed locator remains `home_region=cn`; the Global relay
is not a fallback and does not proxy that session. This topology does not
require an L4 federation edge. Confirm the two externally published URL pairs
and their certificates from the private inventory before rollout.

## Optional China relay for overseas peers

This optional topology is not part of the direct regional deployment above.
Use it only after separately deploying and reviewing the L4 edge. For a
locator with `home_region=cn`, domestic clients connect to the normal CN
WSS endpoint. Overseas clients may instead resolve a Global ingress that is
only an L4 passthrough:

```text
overseas client inner TLS/WSS
  -> Global L4 stream proxy
  -> fixed outer-mTLS backhaul
  -> private CN federation listener
  -> normal CN TLS/WSS terminator
  -> the same CN relay process
```

The Global hop never terminates or inspects the inner TLS stream and is not a
Global home relay or fallback. Both peers still reach the same CN pairing
table, and the CN authenticator remains authoritative for the signed
`home_region=cn`. See
[`deploy/collab-relay-edge`](../collab-relay-edge/README.md) for the fixed
nested-TLS topology, certificates, limits, and validation steps.

On an overseas desktop, configure the logical CN endpoint
`OPENPENCIL_COLLAB_RELAY_CN_URL` with the Global ingress WSS hostname, or use
GeoDNS/split DNS so that the same CN endpoint hostname resolves to Global
ingress overseas and the direct CN terminator domestically. The inner
certificate is still presented by the CN terminator and must cover that
hostname. `OPENPENCIL_COLLAB_RELAY_GLOBAL_URL` remains reserved for locators
whose signed home region is actually `global`; it is never an implicit
fallback for a CN locator. A domestic peer and an overseas peer can therefore
take different network paths while converging on the same CN relay
process/shard.

This intentionally creates a cross-border data path and requires the
applicable transfer, latency, reachability, and availability review. A CN
relay rejects a locator signed for the Global home region.

The server currently keeps pairing state in one process. Do not round-robin a
relay hostname across independent replicas without stable route-key sharding
or a shared rendezvous layer.
