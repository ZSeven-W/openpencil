# op-auth prebuilt artifact policy

These target archives may be committed, but they are inspectable inputs to the
final application link. Client-side secrecy is defense in depth, not the trust
root. Ticket signing keys, token exchange, and authorization policy stay on the
server.

## Current status

The six `0.8.3` archives are legacy ABI-v1 compatibility artifacts:

- Their bytes are pinned by the adjacent `SHA256`.
- Their exposed `op_auth_*` names match the eight-symbol ABI-v1 allowlist.
- They are not signed release-provenance artifacts.
- They contain substantial source/build-path and debug metadata leakage.
- They must not be relabeled `op-auth-hardened-v1` or promoted to production
  collaboration ABI-v2 without a private-source rebuild.

`tools/check-op-auth-prebuilt.sh` reports the measured leakage without changing
archive bytes. `--require-hardened` intentionally rejects all current archives.
Do not run `strip`, `objcopy`, or an obfuscator in place on these committed
files: archive members and cross-object symbols may be required by the final
link.

## ABI-v2 signed provenance

Every production target directory must contain:

```text
ABI_VERSION       exactly 2
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

No release public key is committed yet because the private release signer has
not been provisioned. Consequently an ABI-v2 archive cannot accidentally link
until that external production step is complete.

## Private rebuild profile

Build the private implementation from a clean, pinned toolchain and record the
full source revision. At minimum:

- compile without debuginfo and remap all source, Cargo registry, build, and
  temporary-directory prefixes;
- use one narrow `extern "C"` wrapper and keep every other implementation
  symbol hidden from the final binary;
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
  --version 0.8.3 \
  --source-revision <full-private-source-revision> \
  --build-id <immutable-ci-build-id> \
  --signing-key /private-ci/secrets/op-auth-ed25519.pem \
  --output-root /private-ci/staged-prebuilt
```

The packaging script never modifies the candidate archive. It produces fresh
metadata, signs it, verifies the signature, and runs the hardened archive gate.
Copy the reviewed staged files into this directory only after that succeeds.
