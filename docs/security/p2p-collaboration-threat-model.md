# P2P collaboration threat model and trust boundary

Status: public M1 implementation checkpoint, 2026-07-28; the private identity
source integration is implemented, while production provisioning and the full
real-platform acceptance matrix remain pending.

This document is the security contract for OpenPencil peer-to-peer
collaboration. It is intentionally public. The protocol, parsers, state
machines, cryptographic transport integration, resource limits, and security
tests do not rely on implementation secrecy.

## Scope and security goals

M1 is designed to connect native desktop peers on a local network by mDNS
discovery or a manually entered IP address. Document bytes travel directly
between peers.
Multicast DNS is LAN-scoped and is not a cross-region discovery mechanism.
A manually entered publicly routable address can exercise the current TCP and
Noise path across sites when firewall and NAT policy permit it, but M1 has no
rendezvous, NAT traversal, or relay and does not claim robust Internet
reachability.
ZSeven services authenticate accounts and are the production issuer boundary
for short-lived admission tickets. The private issuer and credential-bearing
provider source implementations now exist, including a strict JWKS profile,
persistent rotation ledger, Unix-HSM peer authentication, and an append-only
ABI-v2 client contract. Production HSM keys, protected static archives,
deployment policy, and hardened runners are separate provisioning gates and
are not claimed complete at this checkpoint. The services do not store or
relay document content.

The implementation must:

- authenticate both ends with a server-signed collaboration ticket;
- prove that each ticket is bound to the Noise static key used by that
  connection;
- admit only the expected issuer and account subject;
- encrypt and integrity-protect all admission and document traffic;
- prevent an unauthenticated peer from receiving document, thumbnail, session
  name, or presence data;
- enforce the owner-authoritative role and edit policy;
- reject malformed, oversized, stale, replayed-state, or unsupported
  operations without partially mutating the document;
- bound connections, handshakes, frames, transfers, queues, document
  validation, and cached metadata;
- keep tickets, private keys, device credentials, document content, and
  personally identifying account data out of logs and discovery metadata.

Availability against a party that can saturate the host's network link is not
guaranteed. The limits below are intended to contain CPU and memory use after
traffic reaches OpenPencil.

## Assets

| Asset | Required protection |
| --- | --- |
| Document and presence content | Confidentiality and integrity in transit; no disclosure before admission |
| ZSeven device credential | Private implementation and platform-protected storage; never exposed through the open collaboration API |
| Collaboration signing private key | Issuer/HSM boundary only; never shipped to a client or repository |
| Short-lived collaboration ticket | Treated as a bearer credential, redacted and zeroized where owned |
| Noise X25519 static private key | Local-only, zeroized in memory, stored with platform/file protections |
| Account subject and device id | Derived only from verified claims; omitted from mDNS and routine logs |
| Collaboration display name and avatar URL | Accepted only as bounded signed claims; disclosed only to admitted session participants |
| Owner role and commit sequence | Owner-authoritative and guarded against guest self-assertion or stale writes |
| Resource availability | Bounded before allocation, reassembly, parsing, or broadcast |

## Trust boundaries and data flow

1. The private authentication provider uses an already authenticated device
   session to request an opaque ticket. Its public request contains only the
   local X25519 public key.
2. The ticket issuer signs the fixed public claims profile. It does not return
   signing keys to the client.
3. mDNS advertises only an ephemeral discovery id, protocol version, and TCP
   port. Discovery is a locator, not an authentication statement.
4. Peers complete `Noise_XX_25519_ChaChaPoly_BLAKE2s`. The responder prelude is
   included in the Noise prologue.
5. Tickets are exchanged inside the encrypted Noise channel. The open verifier
   checks signature, issuer, audience, version, scope, time, identifiers, and
   the equality of `dh_pub_x25519` with the observed remote Noise static key.
   Optional `display_name` and `avatar_url` claims are accepted only from this
   signed payload; names are bounded and control-free, and avatar URLs must be
   bounded HTTPS URLs without credentials or fragments.
6. Only after both admission checks succeed may the owner send a welcome,
   snapshot, commit, or presence message.
7. The owner assigns the connection role and is the serialization point for
   accepted commits. A guest cannot acquire permissions by putting a role,
   author, subject, or device id in an untrusted message.

Neither an mDNS record, a local `SignedIn` flag, an email address, nor a
peer-supplied profile is an authentication source. Issuers that have not yet
added the optional signed profile claims remain compatible, but their peers
receive generic epoch-local labels rather than a fallback from local account
UI state.

Ticket-bearing protocol values are deliberately non-`Clone`. Generic JSON and
frame-transfer encoders reject `RenewTicket`; the dedicated sensitive encoder
serializes directly into uniquely owned zeroizing storage without first
materializing a `serde_json::Value` or ordinary output buffer. Admission,
renewal commands, chunking, decryption, reassembly, and per-peer queues retain
that ownership discipline so ticket plaintext is zeroized on every drop path.

### Domestic and overseas issuer topology

Regional credential origins are not collaboration trust roots. A domestic
client may use its local SSO for login and ticket requests while an overseas
client uses another SSO origin. Both deployments must issue the exact same
logical collaboration `iss` and the same immutable global account `sub`; email
address matching or region-local account ids are not a federation mechanism.

The desktop reads `OPENPENCIL_SSO_URL`,
`OPENPENCIL_COLLAB_ISSUER`, and
`OPENPENCIL_COLLAB_POLICY_ENDPOINT` only from trusted process-startup
configuration. Production fetches `/api/v1/collab/policy`; the envelope must
verify under the offline Ed25519 root pinned into the open client. Endpoint-only
configuration, conflicting policy/JWKS endpoints, signature or issuer
mismatch, inactive key metadata, generation rollback, and same-generation
rewrites fail closed without a raw-JWKS fallback. The old
`OPENPENCIL_COLLAB_JWKS_ENDPOINT` remains an explicit self-hosted compatibility
path. No ticket, provider response, mDNS record, invite, or peer message can
supply or replace the pinned trust values.

Each region owns an independent HSM signing key hierarchy and never exports or
copies private keys to the other region. Region-local mirrors publish one
canonical offline-signed policy containing the complete public union. The
client validates no more than 8 regions/24 keys, exact region membership,
unique key ids/public keys, one active and one next key per region, and at most
one retired overlap key. It excludes unactivated next keys from ticket
verification and rechecks policy/key times on every cache use. Key publication,
activation, overlap, retirement, and emergency removal must therefore be
authorized in a higher generation before either region changes signing state.
Mirror availability, consistency, HSM provisioning, and physical multi-region
timing tests remain production gates.

## Public and private ownership

The default is open source. Code stays private only when publishing it would
expose an account credential or a production signing secret.

| Component | Repository boundary | Rationale |
| --- | --- | --- |
| Wire protocol, exact diff/apply, canonical hash, owner/guest state machines | Public `openpencil/crates/op-collab` | Reviewable deterministic behavior; wasm-compatible |
| Noise/TCP framing, admission, limits, queues, discovery, key-store interface and safe fallback | Public `openpencil/crates/op-collab-transport` | Security comes from open protocol and audited libraries |
| Ticket claims/profile, signed-union-policy and legacy JWKS parser/cache, Ed25519 verifier, provider trait, stub, ABI declarations | Public `openpencil/crates/op-auth-bridge` | Trust decisions must remain reviewable |
| Deterministic issuer and rotation fixtures | Public, compiled only for tests or `test-issuer` | Contain deliberately public seeds and a `.invalid` issuer; production verifier rejects that issuer |
| Host integration, UI, recovery, diagnostics, and smoke tests | Public `openpencil` | No credential-handling reason to hide them |
| Device token and authenticated ticket request implementation | Private `op-platform` real provider | Holds the account credential and platform storage integration |
| Ticket issuance policy, production signing/HSM keys, rotation and revocation | Private `zseven-sso` | Contains production signing authority and account policy |
| Runtime tickets, Noise private keys, device tokens, HSM material | Never committed | Secrets are runtime data, not source assets |

Public test keys are not a backup or development production key. A build that
enables `test-issuer` must still use an explicitly pinned test verifier; the
production constructor defaults to `https://sso.zseven.cn`. The desktop may
use explicit startup-pinned collaboration issuer/JWKS values for a controlled
regional deployment, independently of its credential origin.

## Threats, controls, and residual risk

### LAN interception and active man-in-the-middle

Noise XX encrypts traffic and authenticates possession of both static private
keys. The signed ticket binds the authenticated account/device claims to the
remote static key observed in that handshake. A relayed or substituted static
key therefore fails admission.

Traffic endpoints, timing, and approximate volume remain visible to the local
network. M1 does not provide anonymity or traffic-shape concealment.

### Discovery spoofing and privacy

mDNS is untrusted and spoofable. Advertisements contain no account, email,
device name, document title, session title, or stable hostname. Consumers
strictly parse the `id`, `v`, and `p` fields, cap the discovery cache, honor
removal, and proceed to Noise plus ticket authentication before disclosure.
On macOS, a live `DNSServiceGetAddrInfo` operation refreshes a 30-second
upper-layer lease every 10 seconds; removal clears it, while a dead worker ages
out within that lease. A full bounded event lane drops only the current
notification and relies on the next heartbeat to republish current state;
receiver disconnection is the condition that stops the worker.

The macOS source plist declares the canonical local-network usage description
and exactly `_openpencil-collab._tcp` in `NSBonjourServices`. Both bundle paths
patch and validate the final plist through one helper, and the release workflow
validates the exact app plist before notarization. This is a packaging/TCC
precondition, not peer authentication.

An attacker can advertise many endpoints or make connection attempts. Cache,
pending-handshake, per-IP, and active-connection limits reduce impact but
cannot prevent network-link exhaustion.

### Forged, replayed, wrong-account, or downgraded tickets

The verifier accepts only compact JWS with protected `alg=Ed25519`,
`typ=openpencil-collab+jwt`, and a bounded `kid`. Claims are exact and include
the pinned issuer/audience/version/scope, canonical subject/device ids, channel
binding, `iat`, `nbf`, `exp`, and `jti`. JWKS data is fetched only from the
pinned HTTPS endpoint and is strictly parsed as public OKP/Ed25519 verification
keys.

A stolen ticket can be used only with the bound Noise private key and until its
signed expiry. M1's revocation service-level objective is the ticket lifetime,
currently at most 15 minutes; sign-out prevents renewal but does not
retroactively invalidate an issued ticket. If immediate revocation becomes a
product requirement, it needs an online revocation epoch or introspection
mechanism.

Unknown signing keys trigger a throttled refresh. Expired JWKS cache entries
fail closed when refresh fails. Key rotation must publish overlap keys for at
least the maximum ticket lifetime and cache interval.

In a multi-region deployment, a regional ticket is accepted only after its
signing key appears in the same logical union JWKS observed by every client.
The pinned endpoint URL may be a region-local mirror, but its canonical keyset
must match every other mirror. Routing clients to partial regional keysets
would create asymmetric admission and renewal failures and is a deployment
error, never a reason to fall back to a ticket-provided key or a second issuer.

Renewal distinguishes evidence that a ticket is invalid from an unavailable
trust source. A key id absent from a successfully refreshed keyset is rejected;
transport, cache, malformed-keyset, invalid-ETag, rejected-response, and
response-limit failures publish no renewed ticket and retry only while the
previously verified ticket remains valid. Persistent failure closes the
session before the old ticket expires.

The production native fetcher propagates cancellation through cache-lock
waiting, the async HTTPS send, and every streamed body chunk. Cancellation
drops the unfinished request future, rolls back refresh/unknown-key timing
markers, closes the pending result lane, and joins the worker without waiting
for the network timeout or publishing a late result. The trait default can only
check before and after a blocking third-party fetch; any other production
blocking adapter must override the cancellable method.

### Unauthorized edits and identity injection

Verified identity metadata is constructed only by the admission boundary.
Owner state assigns roles and rewrites authoritative author/sequence fields.
Viewer edits, counter gaps, stale bases, failed preconditions, unsupported
operations, and session/epoch mismatches are typed rejection paths.

The authenticated `Participant` roster wire projects only the verified display
name and HTTPS avatar URL alongside epoch-local participant/peer ids and role.
Persistent subject and device ids remain inside the non-serializable connection
principal and are never added to Welcome, presence, commit, or roster messages.
The desktop host copies the URL only into a process-local, generation-scoped
avatar registry. It is absent from the document, `EditorState`, narrowed
off-thread snapshots, and redacted `Debug`; fetched image bytes never enter the
collaboration protocol.

The owner can intentionally send document content or grant edit access; that is
the collaboration action, not an attacker bypass. M1 does not protect a
document from an authorized malicious participant after disclosure.

### Parser, memory, CPU, and queue exhaustion

Limits are typed configuration with hard maxima. They cover compact tickets,
JWKS bodies and key counts, identifiers, operation count, validation visits,
tree depth, document node count, presence, envelopes, transaction/snapshot
transfers, Noise records and handshakes, reassembly, connection counts,
per-peer/global queues, rate buckets, timeouts, commit history, and discovery
cache entries.

Lengths and counts are checked before allocating complete buffers or applying
mutations. Invalid refreshes do not replace a valid JWKS cache. Exact apply is
transactional: rejection must not leave partial document changes.

Every new externally controlled collection, string, transfer, parser, retry,
cache, or queue requires:

1. a named default and hard maximum;
2. a typed configuration or wrapper validated at construction;
3. a typed, log-safe error;
4. tests at the limit and one unit over it;
5. outbound enforcement as well as inbound enforcement.

### Local key theft and filesystem attacks

Private key buffers implement zeroization and redact `Debug`. The open
`OsKeyStore` uses macOS Keychain, Windows Credential Manager, or Linux Secret
Service for the device static key. A locked, inaccessible, ambiguous, or
malformed platform-store entry fails closed; it must not silently create a
replacement identity.

The Unix file store is a narrowly scoped fallback only when the selected
platform store has definitively reported that it is unavailable. It uses a
dedicated `0700` directory, `0600` files, no-follow opens, atomic installation,
length/all-zero checks, and reopens through the hardened read path. A locked or
temporarily inaccessible platform store is not “unavailable” and must not
trigger this fallback. The file store is not hardware-backed and cannot protect
against a compromised user account or process; platforms without the required
filesystem guarantees fail closed.

Platform key-store adapters may remain public; only their runtime secret values
are private.

### Logging, crash reports, and fixtures

Errors crossing public boundaries contain closed codes and sizes, never remote
bodies or credentials. `Debug` for ticket, key, verified identity, signed
profile, roster profile, and document containers must redact content.
Production logging must not include tickets, Noise keys, full snapshots,
document text, email, subject, device id, display name, or avatar URL.

Repository fixtures must not contain PEM private keys, production-looking
tokens, compact bearer tickets, or copied runtime key files. Deterministic test
signing material is permitted only behind the `test-issuer`/test compilation
gate and must use a non-production `.invalid` issuer.

The permanent Go-to-Rust interoperability vector follows that boundary. Its
public fixture stores the non-production compact JWS as three separate segments
and locks the segments plus public JWKS with one SHA-256 digest. The Rust test
joins and verifies them in memory; a private-repository producer test invokes
the real Go issuer service and independently locks the exact same segments,
JWKS, and digest. It contains no device token, production identity, signing
private key, HSM metadata, or authorization policy.

### Supply-chain and cryptographic downgrade

Cryptographic algorithms and protocol versions are fixed rather than selected
by a peer. Unknown versions and algorithms fail closed. Dependencies remain
covered by the repository's pinned lockfile, cargo-deny advisories/bans checks,
and targeted collaboration CI.

Changing the Noise pattern, JWS algorithm, canonical encoding, key type,
protocol version, dependency source, or maximum size is a security review
event, not a routine refactor.

Committed authentication static archives are inspectable client inputs, not a
trust boundary. The current ABI-v1 matrix is a legacy compatibility lane:
SHA-256 and the narrow C ABI are audited, but the archives still leak
source/debug/private-symbol metadata and are not claimed to be stripped or
obfuscated. Production ABI-v2 requires a private-source hardened rebuild plus
an Ed25519-signed provenance manifest covering the exact artifact hash, target,
version, ABI, source revision, build id, and hardening profile; missing or
invalid provenance fails closed. Obfuscation may raise reverse-engineering
cost, but authorization and ticket trust remain rooted in server-held signing
keys and policy.

Encrypting an archive at rest is useful only when the decryption key remains in
the private release system. Committing the key, embedding it in `build.rs`, or
shipping a client-side decryptor beside the ciphertext does not materially
raise the reverse-engineering boundary and would make a reproducible public
build misleading. A published client binary remains inspectable even when its
build input was encrypted.

## Automated boundary gate

Run:

```bash
bash tools/check-collab-security-boundaries.sh
bash tools/check-collab-security-boundaries.test.sh
bash tools/check-op-auth-prebuilt.sh
bash tools/check-op-auth-prebuilt.test.sh
bash tools/check-macos-bundle-plist.sh
bash tools/check-macos-bundle-plist.test.sh
```

The gate verifies:

- the `op-collab` wasm dependency closure contains no native transport,
  authentication, network, or random-key dependencies;
- every `op-collab*` crate and `op-auth-bridge` resolves its license from the
  MIT workspace license;
- required typed protocol, apply, transport, queue, and hard-limit anchors are
  present, and collaboration source does not return `Result<_, String>`;
- test issuer/signing fixtures remain behind explicit test feature gates;
- committed authentication archives keep their exact hashes and documented C
  ABI, while hardened ABI-v2 artifacts require signed provenance and contain
  no source paths, debug markers, or private Rust module symbols;
- high-signal private-key, token, compact-ticket, and sensitive-file patterns
  are absent;
- both macOS bundle paths and the pre-notarization release path preserve the
  exact local-network and Bonjour plist declarations;
- collaboration Rust files stay within the repository's 800-line limit.

The independent `collab-security.yml` workflow also compiles `op-collab` for
`wasm32-unknown-unknown`, runs the full protocol/state-machine/property suite,
checks transport limits and production-verifier isolation, and exercises the
two-process Noise/ticket collaboration smoke with the isolated public test
issuer.

Static scanning is defense in depth, not a secret scanner or security review
replacement. Production credentials must also be blocked by repository host
secret scanning and incident-response procedures.

## Review triggers

Request a security review when any change:

- adds or changes a claim, trust root, key source, cryptographic algorithm, or
  protocol version;
- permits document or presence data before admission completes;
- adds discovery metadata or log fields;
- changes expiry, renewal, rotation, revocation, or owner-leave behavior;
- introduces an externally controlled allocation, retry, cache, or queue;
- adds a new collaboration crate or moves code across the public/private
  boundary;
- adds a native dependency to `op-collab` or enables a new default feature;
- changes persistent private-key storage.

Security reports should identify the affected boundary and avoid attaching
live tickets, credentials, private keys, or document content.
