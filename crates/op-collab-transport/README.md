# op-collab-transport

Native, open-source transport for OpenPencil peer-to-peer collaboration.

The crate owns the M1 TCP record framing, Noise XX handshake, authenticated
transfer chunking, admission boundary, connection resource limits, device
static-key persistence, and optional mDNS discovery. Collaboration sequencing
and document mutation remain in the wasm-clean `op-collab` crate.

Production signing keys, device tokens, and account credentials are not part
of this crate. Ticket verification is injected through a narrow public trait.

## Production driver

Use `ConnectionDriver` as the sole owner of a connected `TcpStream` and its
Noise state. It is nonblocking in both directions, preserves partial reads and
writes, bounds queued and reassembled data, and never holds a Noise lock across
I/O. GUI and document threads should send typed commands to the event-loop
thread instead of touching the connection directly.

```rust,ignore
let (_, secure) = connect_secure_tcp(
    address,
    &device_static_key,
    selected_discovery_id.as_deref(),
    config,
)?;
let budget = SharedQueueBudget::new(config.connections.global_queued_bytes)?;
let inbound_policy = if is_initiator {
    InboundTransferPolicy::OwnerToGuest
} else {
    InboundTransferPolicy::PeerToOwner
};
let mut driver = ConnectionDriver::new(secure, budget, inbound_policy)?;

if is_initiator {
    driver.queue_admission(&local_hello, Instant::now())?;
}

loop {
    let now = Instant::now();
    let polled = driver.poll(now)?;

    match polled.event {
        Some(DriverEvent::Admission(hello)) => {
            let identity = driver.verify_remote_admission(
                &hello,
                &ticket_verifier,
                expected_issuer,
                expected_subject,
                now_unix_ms(),
                now,
            )?;
            if !is_initiator {
                // The responder reveals its ticket only after authenticating
                // the initiator.
                driver.queue_admission(&local_hello, now)?;
            }
            authorize(identity)?;
            driver.authorize_remote(remote_role)?;
            driver.activate(now)?;
        }
        Some(DriverEvent::Frame { frame, .. }) => handle_frame(frame)?,
        None => {}
    }

    if driver.ticket_renewal_due(now) {
        request_or_expect_renewal();
    }

    let wake_at = [polled.rate_ready_at, driver.next_deadline()]
        .into_iter()
        .flatten()
        .min();
    schedule_socket_or_timer(wake_at, polled.has_pending_output);
}
```

For accepted sockets, acquire `ConnectionLimiter::try_begin_handshake` before
calling `accept_secure_tcp`, and convert the pending guard to an active guard
only after ticket admission succeeds.

The inbound direction policy is part of allocation admission. In particular,
an owner rejects guest-originated `Snapshot` transfers from their authenticated
header before reserving the snapshot reassembly budget; a guest permits
owner-originated snapshots for initial sync and log-gap recovery.

`SecureConnection` also exposes a blocking helper API for focused tools and
tests. Its `send_transfer` atomically preflights the entire transfer against
the configured burst so a rate-limit rejection cannot leave a partial
transfer. The production driver instead rate-limits and resumes one
authenticated chunk at a time.

Pre-encoded frames can only be created by the validated frame encoders.
Non-ticket transfers may share their immutable encoded allocation across peer
queues; ticket transfers stay uniquely owned and zeroize on every drop path.
Ticket bytes cannot enter the ordinary shared or coalescing queue
constructors. Queue item types are crate-private, and received transfer fields
are exposed through borrowed accessors rather than public, cloneable storage.
Payload-bearing transport types implement redacted `Debug` output that reports
only transfer metadata such as class, id, and encoded length.

The generic raw JSON and frame-transfer encoders reject `RenewTicket`. Its
dedicated sensitive encoder writes directly into uniquely owned zeroizing
storage without first constructing a `serde_json::Value` or ordinary encoded
buffer. Admission, chunking, decryption, reassembly, and renewal commands move
that same non-shareable ownership forward rather than cloning ticket
plaintext.

## Liveness and ticket lifecycle

After admission becomes active, idle connections exchange a fixed
transport-level heartbeat encrypted and authenticated by Noise. Only a
successfully decrypted inbound record refreshes the inbound idle deadline, so
an unresponsive peer cannot be kept alive by this process's own writes.

Ticket expiry is converted to monotonic deadlines when a ticket is installed.
The driver exposes a one-shot proactive renewal point at 80% of the remaining
TTL and retains the hard expiry deadline. Successful renewal verifies the same
issuer, subject, device, and Noise static key, requires a later expiry, and
atomically rearms both deadlines.

## Open-source and private boundaries

The wire protocol, validation, rate and connection limits, ticket verifier
trait, Noise/TCP implementation, discovery parser, and key stores are public.
`OsKeyStore` uses macOS Keychain, Windows Credential Manager, or Linux Secret
Service (zbus + RustCrypto). Its stored value is versioned and text-safe for
Secret Service implementations such as KDE Wallet. Locked, inaccessible,
ambiguous, or malformed entries fail closed and never trigger silent identity
replacement.

`FileKeyStore` is the explicit fallback. On Unix it enforces owner-only
permissions, no-follow opens, and atomic creation; on platforms without those
guarantees it fails closed. Hosts should select this fallback only when the
platform store is known to be unavailable, not when it is merely locked, so a
temporary access failure cannot create a second device identity.

Production ticket signing keys, device/account credentials, token exchange,
and account authorization policy stay outside this crate. Logs and `Debug`
output must never contain opaque tickets, Noise private keys, account
identifiers, document text, or snapshots.

## Platform mDNS smoke

The deterministic discovery parser/cache tests run in normal CI. A separate
ignored smoke exercises the actual multicast stack and must run on each native
platform runner (or release machine) with LAN multicast enabled:

```bash
cargo test -p op-collab-transport --features mdns \
  --test mdns_smoke -- --ignored --exact publisher_is_discovered_and_unregisters_cleanly
```

It publishes only an opaque random discovery id, verifies the advertised
endpoint, unregisters it, and waits for the browser cache to remove it. It is
not enabled on generic hosted runners because those networks commonly suppress
mDNS and would make the result environmental rather than deterministic.
Runners where interface auto-selection is ambiguous can set
`OP_COLLAB_MDNS_SMOKE_ADDRESS` to that runner's active LAN address.

On macOS the discovery I/O backend uses the system Bonjour daemon through
`dns_sd.h`; Linux and Windows continue to use `mdns-sd`. The macOS publisher
keeps the random service and host labels, registers only addresses belonging
to eligible local interfaces, and never substitutes the user-visible computer
name. A second opt-in macOS check verifies browser interoperability with the
system `dns-sd` publisher:

```bash
cargo test -p op-collab-transport --features mdns \
  --test mdns_smoke -- --ignored --exact browser_interoperates_with_system_dns_sd_publisher
```
