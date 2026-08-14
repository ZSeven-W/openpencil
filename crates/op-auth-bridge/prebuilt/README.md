# op-auth prebuilt artifact policy

These target archives may be committed, but they are inspectable inputs to the
final application link. Client-side secrecy is defense in depth, not the trust
root. Ticket signing keys, token exchange, and authorization policy stay on the
server.

## Current status

The six committed desktop `0.8.4` archives are signed ABI-v3 artifacts:

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

No signed iOS or Android archive is currently committed. Mobile production
authentication therefore stays fail-closed until private release CI stages an
exact-version target artifact. Unsigned local mobile archives belong only in
the explicit Debug override and must never be copied into this directory.

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
public key as lowercase hex. The private signing key must exist only in the
private release system. `build.rs` verifies the signature over the exact
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

Once the candidate already passes its private tests, stage it in a new
directory:

```text
tools/package-op-auth-prebuilt.sh \
  --artifact /private-ci/out/libop_auth.a \
  --target x86_64-unknown-linux-gnu \
  --version 0.8.4 \
  --source-revision <full-private-source-revision> \
  --build-id <immutable-ci-build-id> \
  --signing-key /private-ci/secrets/op-auth-ed25519.pem \
  --output-root /private-ci/staged-prebuilt \
  --abi 3
```

The packaging script never modifies the candidate archive. It produces fresh
metadata, signs it, verifies the signature, and runs the hardened archive gate.
Copy the reviewed staged files into this directory only after that succeeds.
