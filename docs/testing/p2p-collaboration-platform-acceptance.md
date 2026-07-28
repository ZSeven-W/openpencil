# P2P collaboration platform acceptance

This runbook records evidence that cannot be replaced by protocol unit tests or
same-process mocks. It deliberately distinguishes a real network-path check
from a two-physical-device acceptance result.

## Evidence rules

- Use the exact same OpenPencil revision on both devices.
- Record the OS, architecture, revision, commands, exit status, and complete
  machine-readable result.
- A two-device result requires two different physical devices. Two processes,
  VMs, or addresses on one host do not qualify.
- Test credentials use the public `.invalid` issuer only. Production ticket
  timing and sign-out are a separate private integration gate.
- Do not include tickets, device tokens, account identifiers, document content,
  or private keys in evidence.

## Frozen Go issuer interoperability

The public `op-auth-bridge` fixture is a permanent, non-production
interoperability vector emitted by the real Go issuer service with deterministic
test-only data. It uses `https://sso.example.invalid`, stores the compact JWS as
three separate segments, and locks
`SHA-256(join(ticket_segments, ".") || 0x00 || jwks_json)`. The Rust test joins
the segments only in memory and verifies the ticket against the frozen JWKS.
An independent test in the private issuer repository calls the actual Go
`Service.Issue` and `Service.KeySet` paths and locks the same segments, JWKS,
and digest. This is a producer/verifier compatibility gate, not production HSM
or account-policy evidence.

## Renewal cancellation regression

The production native JWKS path races both the HTTPS send and each response-body
read against cancellation, dropping the unfinished async request future rather
than detaching work until the five-second network timeout. Cancellation also
interrupts cache-lock waiting without advancing refresh or unknown-key
backoff. Dropping a pending renewal closes its result channel, signals
cancellation, and then briefly joins the worker so no late result or request is
left behind. Deterministic tests cancel a never-completing fetch, observe the
join in under one second, and immediately start replacement work. This is
source lifecycle evidence; production ticket timing remains a separate gate.
During renewal, only an unknown key confirmed against a successfully refreshed
keyset rejects the ticket immediately. Transport, cache, and invalid authority
responses leave the unverified renewal unpublished and back off while the old
verified ticket is valid; persistent failure still closes before its expiry.

## Domestic and overseas trust-domain acceptance

Configure each client with its regional credential origin and the same
explicit collaboration trust root. For example:

```text
# Domestic client
OPENPENCIL_SSO_URL=https://login.cn.example
OPENPENCIL_COLLAB_ISSUER=https://collab.example
OPENPENCIL_COLLAB_POLICY_ENDPOINT=https://login.cn.example/api/v1/collab/policy

# Overseas client
OPENPENCIL_SSO_URL=https://login.global.example
OPENPENCIL_COLLAB_ISSUER=https://collab.example
OPENPENCIL_COLLAB_POLICY_ENDPOINT=https://login.global.example/api/v1/collab/policy
```

Before network testing, prove all of the following without recording live
tickets or identities:

- both regional ticket endpoints emit the exact configured `iss`;
- one account has the same immutable global `sub` in both regions;
- each region signs only inside its own HSM, using globally unique `kid`
  values;
- both broker configs load the same offline-signed policy containing every
  region's active/next pair and any live retired overlap;
- both pinned regional mirrors return the same canonical policy body and ETag,
  and the OpenPencil verifier accepts its pinned-root signature;
- a domestic ticket verifies on the overseas client and an overseas ticket
  verifies on the domestic client, including Noise static-key binding;
- a different issuer, region-local account id, missing regional key, rewritten
  `kid`, remote private object, bad policy signature, generation rollback,
  same-generation rewrite, premature next-key use, or expired overlap is
  rejected;
- setting either endpoint without `OPENPENCIL_COLLAB_ISSUER`, setting policy
  and JWKS endpoints together, or supplying an HTTP/path-bearing issuer makes
  collaboration startup fail closed without falling back to raw JWKS.

Run renewal while rotating one region at a time. The new public key must be
visible across the complete union for the configured prepublication window
before activation, and the old key must remain through ticket lifetime,
verifier skew, and cache overlap. Also test a stale or unavailable policy
endpoint: no unverified renewal may be published, and the connection must
close before the previously verified ticket expires.

## Native mDNS on each platform

The manual workflow
`.github/workflows/collab-platform-acceptance.yml` targets dedicated physical
macOS, Linux, and Windows runners labelled `openpencil-mdns`. Each runner must
have multicast access to a real LAN. Run the equivalent command locally when
diagnosing a runner:

```bash
cargo test --locked -p op-collab-transport --features mdns \
  --test mdns_smoke -- \
  --ignored \
  --exact publisher_is_discovered_and_unregisters_cleanly \
  --nocapture
```

The test must discover the ephemeral `_openpencil-collab._tcp.local.` service,
verify its port, then observe its removal after unregister. A compile-only
result or a hosted network that suppresses multicast is inconclusive, not a
pass.

On macOS, an accepted package must also contain the canonical
`NSLocalNetworkUsageDescription` and exactly one `NSBonjourServices` entry,
`_openpencil-collab._tcp`. The checked-in plist and both macOS bundle scripts
use one canonical helper and validate the final bundle plist. The release
workflow checks the exact `OpenPencil.app/Contents/Info.plist` path that it
submits before app notarization. Missing, wrong, empty, or extra values fail the
package gate.

### 2026-07-28 macOS Bonjour diagnostic

A physical Apple Silicon Mac on a real `en0` LAN completed the following
diagnostic against the working tree based on revision
`878a393517b1ffd9575d5c534f16af0a069d44a1`:

- the machine's configured `.local` name resolved through mDNS to the active
  interface's IPv4 address and scoped link-local IPv6 address;
- a system diagnostic service was visible through Browse and Resolve with the
  expected SRV and TXT records, then emitted its removal event;
- the Rust publisher was discovered and removed by the Rust browser both with
  automatic interface selection and with the `en0` address selected explicitly;
- the Rust browser discovered and removed a service published by the macOS
  system `dns-sd` tool; and
- process cleanup left no diagnostic `dns-sd` publisher running.

The diagnostic output was inspected locally and intentionally does not record
the personal host label or interface address here. OpenPencil advertised a
fresh random service label and a fresh random `op-*.local.` hostname, not the
macOS user-visible Computer Name or LocalHostName.

This is positive macOS/Bonjour evidence for one machine and one LAN. It is not
Linux or Windows evidence, a hostname-conflict/automatic-suffix test, or a
two-physical-device result.

### Bonjour browser lifetime and backpressure

The native macOS browser treats a live `DNSServiceGetAddrInfo` operation as
authoritative until removal or cancellation. It exposes a bounded 30-second
cache lease and refreshes that lease every 10 seconds, so a long-lived query
does not disappear merely because its first address TTL elapsed. Explicit
removal clears the entry; a dead worker naturally ages out within the bounded
lease.

The browser event lane remains bounded. If it is full, the current notification
is dropped without terminating the worker, and a later heartbeat republishes
the latest aggregate state. Only a disconnected receiver stops the worker.
Deterministic tests cover both lease renewal/removal and an over-capacity burst
followed by heartbeat recovery.

## Two-device TCP, Noise, and canonical convergence

Choose an explicit unicast address on the owner's LAN interface. The owner
command rejects wildcard, loopback, and multicast addresses.

```bash
# Device A
cargo run --locked -p op-collab-smoke --features test-issuer -- \
  lan-owner 192.168.1.20:45123

# Device B
cargo run --locked -p op-collab-smoke --features test-issuer -- \
  lan-guest 192.168.1.20:45123
```

Both commands must exit successfully and emit
`openpencil-p2p-lan-smoke/v1` JSON with identical `canonical_hash` values.
Repeat once after joining through mDNS and once after disabling mDNS and
entering the owner address manually.

### 2026-07-28 same-host LAN rehearsal

The `lan-owner` and `lan-guest` commands completed over the Mac's real `en0`
unicast address with manual address entry. Both emitted
`openpencil-p2p-lan-smoke/v1` and the same canonical document hash:

```text
f3bebaedd7fe0c29c2b1ff766e2a3f663609e0aea2aa7f6caa69da2ed4bfa3e9
```

This proves that wildcard/loopback rejection still permits the real interface
path and that the TCP, Noise, signed-ticket, edit, and convergence flow works
over that path. Both processes ran on the same physical Mac, so this is only a
rehearsal and does not satisfy the two-device acceptance requirement.

## Cross-site network path

mDNS discovery stops at the local network and is not part of a domestic ↔
overseas test. Until M2 rendezvous/direct-connect and M3 relay are implemented,
the only cross-site path is a manually entered owner endpoint that is publicly
routable (or reachable through an operator-managed VPN), with inbound
firewall/NAT forwarding configured explicitly.

Use two physical devices on different site networks, repeat the
`lan-owner`/`lan-guest` convergence procedure with that routable endpoint, and
record only the redacted machine-readable result. This validates TCP, Noise,
ticket admission, and document convergence over that particular path. It does
not prove general NAT traversal, relay failover, censorship resistance, or
production Internet availability; those remain M2/M3 work.

## Desktop support matrix

On each supported owner/guest OS pairing, verify both directions for:

- position, size, rotation, name, text, solid fill, stroke, opacity, corner
  radius, and existing layout fields;
- basic shape, text, and frame creation and deletion;
- same-page reorder and reparent, group and ungroup;
- presence, editor/viewer admission, and conditional property undo;
- disconnect to read-only, same-epoch reconnect, owner exit, and Save As fork.

After each group, save or inspect both sides and record equal canonical document
hashes.

Verify that each unsupported path is visibly rejected without changing the
shared document:

- page create/delete/rename/reorder/background;
- variables, themes, component registry, and UIKit;
- image, SVG, HTML, import, clipboard paste, and external resource relinking;
- AI/MCP bulk writes, full document replacement, design metadata, app state,
  and routes;
- structural undo/redo and new edits while disconnected.

Finally exercise unauthenticated, wrong-account, viewer-write, expired-ticket,
unapproved, slow-client, crash, and network-jitter cases. No rejected peer may
receive a snapshot or document bytes, and logs/mDNS/errors must contain no PII,
ticket, key, or document content.
