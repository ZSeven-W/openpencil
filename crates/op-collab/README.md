# op-collab

`op-collab` is OpenPencil's transport-free collaboration foundation. It owns
the versioned wire DTOs, deterministic document hashing, and exact atomic
operations over the canonical `PenDocument` schema.

The crate deliberately contains no sockets, async runtime, ticket parser,
signature verifier, or key material. Native hosts provide transport and pass
only already-verified identity metadata across the collaboration boundary.

Current scope includes bounded/validated wire data, canonical hashes, snapshot
verification, shared collaboration-id grammar, atomic exact apply,
supported-edit diffing, and pure owner/guest session state machines. The owner
core binds authenticated connections to roles and namespaces, enforces message
direction and session/epoch identity, maintains bounded roster, commit-log,
dedupe, resume, and selective-undo state, and exposes a two-phase
document-install contract before a Commit can be finalized. The guest core owns
confirmed-plus-pending reconstruction, exact-id replay after reconnect,
CatchUp/Snapshot installation, replacement-epoch quarantine, and selective
undo results.

Native transport lives in `op-collab-transport`; editor/desktop integration
lives in `op-editor-host-core` and `op-host-desktop`; public ticket verification
lives in `op-auth-bridge`. Those open components do not make a production
session complete by themselves: the private production ticket issuer/provider
and real-platform/two-machine release validation remain separate requirements.
