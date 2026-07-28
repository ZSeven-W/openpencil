# op-collab-smoke

Two-process authenticated collaboration smoke for the public M1 stack.

The supervisor launches a fresh pair of distinct owner and guest processes for
every scenario. Every pair completes a Noise XX handshake, mutually verifies
signed tickets bound to each device static key, and exchanges real bounded
collaboration frames over loopback TCP. The matrix covers:

- alternating guest → owner → guest commits;
- a lost Commit recovered by retained-log CatchUp, followed by a second real
  TCP disconnect whose unsent pending Submit is replayed with its original id,
  then a lost Applied acknowledgement whose next resume preserves the owner
  dedupe window and replays only the newly missing Commit;
- stale-base rejection, catch-up, pending rebase, and fresh-id resubmission;
- an atomic transaction whose valid prefix must not survive a later failure;
- same-epoch reconnect through retained-log catch-up;
- same-epoch reconnect through a log-gap Snapshot;
- replacement-epoch quarantine of an old pending edit;
- owner-left read-only termination with an independent local fork.

Each scenario succeeds only when both processes finish at the same canonical
document hash and its scenario-specific sequencing assertions hold.
Every pair also has a 30-second supervisor deadline so a protocol deadlock
terminates both children with captured status and stderr instead of hanging CI.

```bash
cargo run -p op-collab-smoke --features test-issuer -- run
```

## Two-device LAN acceptance

The default supervisor deliberately binds loopback so it is deterministic in
CI. For physical-device acceptance, build the same binary on both devices and
bind the owner to an explicit unicast interface address:

```bash
# Device A
cargo run -p op-collab-smoke --features test-issuer -- \
  lan-owner 192.168.1.20:45123

# Device B
cargo run -p op-collab-smoke --features test-issuer -- \
  lan-guest 192.168.1.20:45123
```

Each process prints one `openpencil-p2p-lan-smoke/v1` JSON record after the
exchange. The acceptance evidence is valid only when the two records come from
different physical devices, both commands exit successfully, and their
`canonical_hash` values are identical. The owner rejects loopback, wildcard,
and multicast bind addresses so this command cannot silently produce loopback
evidence. Run it once with mDNS discovery and once by entering the address
manually while mDNS is disabled.

The binary is unavailable without the explicit `test-issuer` feature. Its
deterministic public key and `.invalid` issuer are isolated from the production
trust root and cannot authenticate a production session. Production tickets,
device credentials, signing keys, and account policy are never part of this
crate.
