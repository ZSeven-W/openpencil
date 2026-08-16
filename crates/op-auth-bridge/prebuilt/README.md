# op-auth prebuilt artifact policy

These target archives may be committed, but they are inspectable inputs to the
final application link. Client-side secrecy is defense in depth, not the trust
root. Ticket signing keys, token exchange, and authorization policy stay on the
server.

## Reusable signed production matrix

The committed matrix is a reusable, signed private-component release. Its
compatibility boundary is the validated op-auth ABI, not the OpenPencil
workspace version or Git topology. A normal OpenPencil source change or version
bump therefore does not require rebuilding unchanged private op-platform code.

`VERSION` identifies the private artifact release. It must be valid and match
the signed release manifest, all ten target `VERSION` files, and all ten signed
`PROVENANCE` manifests, but it need not equal the consuming OpenPencil package
version. Likewise, signed `openpencil_revision` values record the reviewed
public source context used when the private matrix was produced; they are
provenance, not an allowlist for later public commits.

`../AUTH-RELEASE-POLICY` is the source-reviewed adoption lock. It pins the
trusted Ed25519 public key, exact complete release-manifest SHA-256, ABI 3,
private source revision, and immutable build id. This prevents rollback to a
different older matrix that is still validly signed by the same historical
key. The policy is deliberately outside the artifact root and has no
OpenPencil workspace version field.

The reusable artifacts remain independently inspectable:

- Their bytes are pinned by the adjacent `SHA256` and signed `PROVENANCE`.
- Their target, artifact release version, ABI, private revision, build id, and
  hardening declaration are covered by the repository release public key.
- Their exposed `op_auth_*` names match the ABI-v3 allowlist.
- Mach-O and ELF use the hardened profile. A signed Windows pass-through may
  declare the lower `op-auth-signed-unobfuscated-v1` anti-reversing profile.

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

Every production matrix is an atomic, complete ten-target ABI-v3 set:

```text
aarch64-apple-darwin       aarch64-apple-ios
aarch64-apple-ios-sim      aarch64-linux-android
aarch64-pc-windows-msvc    aarch64-unknown-linux-gnu
x86_64-apple-darwin        x86_64-linux-android
x86_64-pc-windows-msvc     x86_64-unknown-linux-gnu
```

The signed release manifest binds all ten targets to the same artifact release
version, recorded public build context, private source revision, and immutable
build id. Release and TestFlight verify the complete matrix in the source they
build. They fail closed on a missing target, invalid signature, mismatched
digest, ABI other than 3, inconsistent metadata, or unrecognized hardening
profile. Unsigned local mobile archives belong only in the explicit Debug
override and must never be copied into this directory.

## ABI-v2 / ABI-v3 signed provenance

Every production target directory must contain:

```text
ABI_VERSION       exactly 3 for production
VERSION           private artifact release identifier
SHA256            lowercase digest of the archive
PROVENANCE        signed key/value manifest
PROVENANCE.sig    raw Ed25519 signature, lowercase hex
libop_auth.a      Unix targets
op_auth.lib       MSVC targets
```

The shared `prebuilt/PROVENANCE_PUBKEY` contains the 32-byte Ed25519 release
public key as lowercase hex. The private signing key exists only in the private
`ZSeven-W/op-platform` production workflow. `build.rs` verifies the signature
over the exact
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

Production matrix rebuilds use the established private-repository release path:

1. The private `ZSeven-W/op-platform` `prebuilt-production` workflow rebuilds
   and audits all ten ABI-v3 targets, and binds every candidate to the private
   source revision, recorded public build context, and one immutable build id.
2. That protected private workflow uses its existing
   `OP_AUTH_PROVENANCE_SIGNING_KEY_PEM` secret to sign every target and the
   complete release matrix, then independently verifies the signed result
   against the public trust root.
3. Promotion stages only the independently verified complete matrix. The
   strict verification mode receives both the expected artifact version and
   recorded public revision. It verifies the new signed matrix before its
   digest is present in the current source policy; supplying only one expected
   value is invalid.
4. Normal Rust, Android, and App Store builds verify the matrix self-consistently
   with neither strict-promotion input. They require the exact source-policy
   digest but do not require the matrix's historical version or recorded public
   revision to equal the consuming checkout.

Rebuild this private matrix when op-platform implementation, ABI, toolchain, or
hardening inputs change. An OpenPencil-only source change or package version
bump does not by itself consume a private production build. Adopting a newly
built private matrix requires a reviewed update to `AUTH-RELEASE-POLICY`.

`tools/package-op-auth-prebuilt.sh` remains a low-level local packaging utility;
it is not the production promotion path and must not receive the production
root. The production signer never modifies or executes candidate archives.
