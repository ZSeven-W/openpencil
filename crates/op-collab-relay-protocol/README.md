# op-collab-relay-protocol

Open, transport-independent wire types for OpenPencil's public collaboration
relay. The relay is a rendezvous and byte-forwarding service: application
traffic remains protected by the existing end-to-end Noise session.

The `opcl1_` value is a signed public locator. It deliberately contains no
account subject, document name, or collaboration session identifier. Internally
the 32-byte bearer route capability remains a separate secret. `RelayInviteV1`
combines both as one pasteable `opc1_` value for the UI; that complete invite
must not be logged or placed in a URL path or query. A protected URL fragment
is acceptable when a product needs a single share action.

Locators use one fixed-size canonical binary representation and base64url
without padding. Their four-byte checksum detects accidental corruption only.
It is unkeyed, is trivial for an attacker to recompute, and provides no
authentication. Authentication comes from the locator signature and the
separate authorization ticket bound to `caller_device_dh_pub_x25519`.

`RelayLocatorV1::decode` and `RelayInviteV1::from_fragment` return unverified
data. A caller must pass either through its explicit verification API; the
resulting `VerifiedRelayLocator` / `VerifiedRelayRoute` types gate outbound
client-hello construction. Servers must also verify the locator, validate its
time window, validate the bearer authorization ticket and its device-DH
binding, validate any possession proof, and apply rate and connection limits
before pairing.

`RelayRegion::Cn` and `RelayRegion::Global` describe a locator's home relay.
They do not constrain where a peer runs: an overseas peer may connect to a
China relay when policy and network reachability permit it.

## Pairing sealed invite v2

Short pairing codes store an opaque sealed invite in the control plane. The
storage handle deliberately retains the v0.8.4 derivation: the first 16 bytes
of BLAKE3 `derive_key("openpencil/op-collab-relay-protocol/pairing-code-id/v1",
canonical_code)`. It remains separate from the HKDF sealing key.

The sealed binary format has its own version, independent from the locator
protocol version:

```text
[sealed_version=2][nonce:12][ChaCha20 ciphertext][Poly1305 tag:16]
```

The 32-byte AEAD key is HKDF-SHA256 with the canonical pairing-code bytes as
IKM, the 12-byte nonce as salt, and
`"openpencil/op-collab-relay-protocol/sealed-pairing-invite/chacha20poly1305-key/v2\0"`
as info. The AAD is
`"openpencil/op-collab-relay-protocol/sealed-pairing-invite/chacha20poly1305-aad/v2\0" || sealed_version || nonce`.
The nonce must be nonzero and unique for a pairing code; production sealing
uses a fresh CSPRNG value. The full 16-byte RFC 8439 tag is always carried.
The v2 parser ceiling is 541 bytes. The public opaque transport/storage ceiling
remains the v0.8.4 value of 569 bytes so a rolling control-plane upgrade can
continue storing and forwarding legacy blobs without understanding them.

Version 2 intentionally does not open the BLAKE3-XOF/XOR/MAC sealed v1
envelope published in OpenPencil v0.8.4. The outer opaque transport media type remains
`application/vnd.openpencil.relay-sealed-invite-v1`; its schema is only a
bounded byte string, while the first byte inside that string selects this
independently versioned cryptographic envelope. Existing control planes and
edges can forward v2 without learning or deploying its cipher, but that does
not make mixed clients compatible: a v0.8.4 short code opens only on a v0.8.4
peer, and a sealed v2 short code opens only on a sealed-v2 peer. Same-version
pairing is required; the LAN/manual endpoint path remains an alternative where
the product exposes it. The current host collapses an unsupported sealed
version to the generic invalid-invite UI error rather than surfacing a
version-specific message.

## Challenge-bound proof v2

Authentication mode `2` replaces the reusable v1 possession attestation with a
relay challenge bound to the caller's X25519 shared secret. The HTTP response
header name is `openpencil-relay-challenge`. Its canonical value is `oprc1_`
followed by unpadded base64url of:

```text
[challenge_version=1][key_id_length:u8][key_id:1..30 graphic ASCII][nonce:32]
```

The nonce and the X25519 shared-secret result must each be nonzero. A v2 proof
has exactly 33 bytes:

```text
[proof_version=2][HMAC-SHA256 tag:32]
```

The interoperable derivation is:

```text
bearer_hash = SHA256(exact bearer-token bytes)
normalized_hello = fixed 501-byte hello with byte [68] and range [69,165) zeroed
binding = SHA256(
  "openpencil/op-collab-relay-protocol/challenge-proof-binding/v2\x00" ||
  u16be(challenge_binary_length) ||
  challenge_binary ||
  bearer_hash ||
  normalized_hello
)
info =
  "openpencil/op-collab-relay-protocol/challenge-proof-key/v2\x00" ||
  0x01 || key_id_length || key_id
okm = HKDF-SHA256(
  ikm = X25519_shared_secret,
  salt = nonce,
  info = info,
  length = 32
)
tag = HMAC-SHA256(okm, binding)
```

Zeroing the complete proof slot removes the circular dependency while retaining
the protocol and extension versions, role, caller DH public key, route
capability, and signed locator in the binding.

Licensed under MIT.
