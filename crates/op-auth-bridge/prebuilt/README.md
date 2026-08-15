# op-auth prebuilt artifact policy

These target archives may be committed, but they are inspectable inputs to the
final application link. Client-side secrecy is defense in depth, not the trust
root. Ticket signing keys, token exchange, and authorization policy stay on the
server.

## Source S and production A

The `0.8.5` source commit S deliberately does **not** contain its production
authentication matrix. The six archives inherited from `0.8.4` cover desktop
targets only. Although those older archives are signed ABI-v3 artifacts, their
`VERSION` does not match the `0.8.5` workspace, so `build.rs` ignores them and
uses the public stub. There are no mobile archives in S. Consequently, S alone
cannot produce a collaboration-enabled production build.

The older desktop artifacts remain independently inspectable:

- Their bytes are pinned by the adjacent `SHA256` and signed `PROVENANCE`.
- Their target, app version, ABI, private revision, build id, and hardening
  declaration are covered by the repository release public key.
- Their exposed `op_auth_*` names match the ABI-v3 allowlist.
- Mach-O and ELF use the hardened profile. The signed Windows pass-through
  declares the lower `op-auth-signed-unobfuscated-v1` anti-reversing profile.

`tools/check-op-auth-prebuilt.sh` reports the measured leakage without changing
archive bytes. `--require-hardened` requires signed provenance for every ABI-v2+
archive and additionally enforces the profile-specific hardening boundary.
Do not run `strip`, `objcopy`, or an obfuscator in place on these committed
files: archive members and cross-object symbols may be required by the final
link.

Linux and MSVC Rust hosts link a temporary Cargo `OUT_DIR` derivative, not
these committed bytes. The build bridge gives the archive's bundled
`rust_eh_personality` an equal-length private name, including its internal
references, because a C-facing Rust `staticlib` otherwise conflicts with the
host toolchain's own personality routine. The transformation happens only
after provenance validation, preserves archive layout, and rejects ambiguous
input; it never weakens the linker's duplicate-symbol checks.

Production promotion creates exactly one auth-only child A of S. A atomically
replaces the six stale desktop directories and adds four mobile directories,
yielding this complete signed `0.8.5` ABI-v3 matrix:

```text
aarch64-apple-darwin       aarch64-apple-ios
aarch64-apple-ios-sim      aarch64-linux-android
aarch64-pc-windows-msvc    aarch64-unknown-linux-gnu
x86_64-apple-darwin        x86_64-linux-android
x86_64-pc-windows-msvc     x86_64-unknown-linux-gnu
```

The signed release manifest binds all ten targets to the same application
version, public source S, private source revision, and immutable build id.
Release and TestFlight workflows accept A only after verifying both the whole
matrix and the S -> A auth-only transition. Until A exists, mobile production
authentication stays fail-closed: an authenticated Release cannot silently use
a missing, stale, mismatched, or unsigned archive. Unsigned local mobile
archives belong only in the explicit Debug override and must never be copied
into this directory.

## ABI-v2 / ABI-v3 signed provenance

Every production target directory must contain:

```text
ABI_VERSION       exactly 2 or 3
VERSION           application package version
SHA256            lowercase digest of the archive
PROVENANCE        signed key/value manifest
PROVENANCE.sig    raw Ed25519 signature, lowercase hex
libop_auth.a      Unix targets
op_auth.lib       MSVC targets
```

The shared `prebuilt/PROVENANCE_PUBKEY` contains the 32-byte Ed25519 release
public key as lowercase hex. The private signing key exists only in the public
repository's reviewer-protected `auth-production` environment; it is exposed
to one isolated GitHub-hosted signer job and deleted before that runner exits.
The later push credential is available only on a different fresh runner.
`build.rs` verifies the signature over the exact
`PROVENANCE` bytes and rejects a mismatched target, filename, version, ABI,
archive digest, hardening profile, source revision, or build id.

ABI v3 appends relay-token minting to the ABI-v2 login and collaboration-ticket
surface. The build bridge gates each appended symbol set by the validated ABI.

## Private rebuild profile

Build the private implementation from a clean, pinned toolchain and record the
full source revision. At minimum:

- compile without debuginfo and remap all source, Cargo registry, build, and
  temporary-directory prefixes;
- use one narrow `extern "C"` wrapper and keep every other implementation
  symbol hidden from the final binary;
- namespace any Rust runtime symbols that must remain in the archive so they
  cannot collide when the C ABI is linked into a different Rust toolchain;
- enable fat LTO, one codegen unit, dead-code elimination, and symbol stripping
  at the final application link;
- use `panic=abort` for the static library if the private implementation can
  uphold that contract;
- apply only reviewed obfuscation/string-protection passes from the private
  LLVM toolchain, then rerun functional and sanitizer tests on their output;
- retain a private SBOM, compiler identity, source revision, build logs, and
  signed artifact attestation.

Rust profile settings commonly used as the starting point are:

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
debug = 0
strip = "symbols"
panic = "abort"
```

Path remapping and symbol visibility remain target/toolchain-specific; those
settings must be applied during the private build, not guessed after archival.
If the private release system also encrypts archives at rest, its decryption
key must remain outside this repository and outside the shipped client.
Checking in the key or a client-side decryptor beside the ciphertext would not
materially impede reverse engineering; the final linked application is still
inspectable. Decrypt into an ephemeral private-CI staging area, run the same
functional and hardening gates, and only then package/sign the reviewed
candidate.

## Production promotion boundary

Production is deliberately split across repositories:

1. The private `prebuilt-production` workflow resolves exact public source S,
   rebuilds and audits all ten ABI-v3 targets, and uploads exactly ten unsigned,
   immutable candidate artifacts. A candidate is build evidence, not a
   production input: it has neither provenance signatures nor authority to
   replace the artifacts in S. The private workflow has no provenance root and
   no OpenPencil write credential.
2. A reviewer dispatches `.github/workflows/auth-production.yml` from the exact
   target branch with the private workflow run ID, private head SHA, public S,
   and that same target branch. The workflow commit, branch head, and S must be
   identical. A narrowly scoped Actions-read credential authenticates the
   private run, workflow path, ten artifact IDs, artifact ZIP digests, and
   canonical candidate metadata on an acquisition-only runner.
3. The acquisition job uploads one immutable, one-day candidate handoff whose
   artifact ID, archive digest, and canonical tree digest are passed forward.
   A fresh signer runner downloads that exact artifact ID, independently
   re-verifies it, receives only the provenance root, signs every target and the
   complete release matrix, deletes the root, and uploads a second immutable
   one-day handoff with its own ID and digests.
4. A third fresh runner receives only the public write credential, downloads
   the exact signed artifact ID, independently verifies its tree digest,
   signatures, trusted public key, and matrix, atomically replaces the stale
   directories and adds the missing mobile directories, creates the
   single-parent auth-only child A, and pushes A with an exact S lease. Rust
   releases and TestFlight accept only a verified A whose parent is S.

`tools/package-op-auth-prebuilt.sh` remains a low-level local packaging utility;
it is not the production promotion path and must not receive the production
root. The production signer never modifies or executes candidate archives.
