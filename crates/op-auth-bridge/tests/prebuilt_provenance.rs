#[path = "../prebuilt_provenance.rs"]
mod prebuilt_provenance;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ed25519_dalek::{Signer, SigningKey};
use prebuilt_provenance::{validate_prebuilt, HARDENING_PROFILE_V1};
use sha2::{Digest, Sha256};

const TARGET: &str = "x86_64-unknown-linux-gnu";
const ARTIFACT: &str = "libop_auth.a";
const VERSION: &str = "1.0.0";
const SOURCE_REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const RELEASE_TARGETS: [&str; 10] = [
    "aarch64-apple-darwin",
    "aarch64-apple-ios",
    "aarch64-apple-ios-sim",
    "aarch64-linux-android",
    "aarch64-pc-windows-msvc",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-linux-android",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
];

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    fixture_root: PathBuf,
    prebuilt_root: PathBuf,
    policy_path: PathBuf,
    target_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let unique = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let fixture_root = std::env::temp_dir().join(format!(
            "op-auth-provenance-{}-{unique}",
            std::process::id()
        ));
        let prebuilt_root = fixture_root.join("prebuilt");
        let policy_path = fixture_root.join("AUTH-RELEASE-POLICY");
        let target_dir = prebuilt_root.join(TARGET);
        fs::create_dir_all(&target_dir).unwrap();
        let artifact = b"deterministic archive fixture";
        fs::write(target_dir.join(ARTIFACT), artifact).unwrap();
        fs::write(
            target_dir.join("SHA256"),
            format!("{:x}\n", Sha256::digest(artifact)),
        )
        .unwrap();
        fs::write(target_dir.join("VERSION"), format!("{VERSION}\n")).unwrap();
        Self {
            fixture_root,
            prebuilt_root,
            policy_path,
            target_dir,
        }
    }

    fn validate(&self) -> Result<prebuilt_provenance::ValidatedPrebuilt, String> {
        validate_prebuilt(
            &self.policy_path,
            &self.prebuilt_root,
            &self.target_dir,
            TARGET,
            ARTIFACT,
        )
        .map_err(|error| error.to_string())
    }

    fn make_signed_abi_v3(&self) {
        fs::write(self.target_dir.join("ABI_VERSION"), "3\n").unwrap();
        fs::write(
            self.target_dir.join("HARDENING-ATTESTATION"),
            format!("target={TARGET}\nartifact={ARTIFACT}\nsource_revision={SOURCE_REVISION}\n"),
        )
        .unwrap();
        let hardening_sha = format!(
            "{:x}",
            Sha256::digest(fs::read(self.target_dir.join("HARDENING-ATTESTATION")).unwrap())
        );
        let sha256 = fs::read_to_string(self.target_dir.join("SHA256")).unwrap();
        let manifest = format!(
            "format=1\n\
             target={TARGET}\n\
             artifact={ARTIFACT}\n\
             version={VERSION}\n\
             abi=3\n\
             sha256={}\n\
             hardening={HARDENING_PROFILE_V1}\n\
             source_revision={SOURCE_REVISION}\n\
             build_id=private-ci-42.a3.{hardening_sha}\n",
            sha256.trim(),
        );
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        fs::write(
            self.prebuilt_root.join("PROVENANCE_PUBKEY"),
            hex(&signing_key.verifying_key().to_bytes()),
        )
        .unwrap();
        fs::write(self.target_dir.join("PROVENANCE"), manifest.as_bytes()).unwrap();
        fs::write(
            self.target_dir.join("PROVENANCE.sig"),
            hex(&signing_key.sign(manifest.as_bytes()).to_bytes()),
        )
        .unwrap();

        let provenance_sha = format!("{:x}", Sha256::digest(manifest.as_bytes()));
        let mut release_manifest = format!(
            "format=op-auth-release-matrix-v1\n\
             version={VERSION}\n\
             abi=3\n\
             source_revision={SOURCE_REVISION}\n\
             openpencil_revision=89abcdef0123456789abcdef0123456789abcdef\n\
             build_id=private-ci-42\n\
             target_count=10\n"
        );
        for target in RELEASE_TARGETS {
            let artifact = if target.ends_with("-pc-windows-msvc") {
                "op_auth.lib"
            } else {
                "libop_auth.a"
            };
            let (archive_sha, provenance_sha, hardening_sha) = if target == TARGET {
                (
                    sha256.trim(),
                    provenance_sha.as_str(),
                    hardening_sha.as_str(),
                )
            } else {
                (ZERO_SHA256, ZERO_SHA256, ZERO_SHA256)
            };
            release_manifest.push_str(&format!(
                "target={target}|artifact={artifact}|sha256={archive_sha}|provenance_sha256={provenance_sha}|hardening_sha256={hardening_sha}\n"
            ));
        }
        fs::write(
            self.prebuilt_root.join("RELEASE-MANIFEST"),
            release_manifest.as_bytes(),
        )
        .unwrap();
        fs::write(
            self.prebuilt_root.join("RELEASE-MANIFEST.sig"),
            hex(&signing_key.sign(release_manifest.as_bytes()).to_bytes()),
        )
        .unwrap();
        fs::write(
            &self.policy_path,
            format!(
                "format=op-auth-release-policy-v1\n\
                 abi=3\n\
                 public_key={}\n\
                 release_manifest_sha256={:x}\n\
                 source_revision={SOURCE_REVISION}\n\
                 build_id=private-ci-42\n",
                hex(&signing_key.verifying_key().to_bytes()),
                Sha256::digest(release_manifest.as_bytes()),
            ),
        )
        .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.fixture_root);
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").unwrap();
    }
    output
}

fn append(path: &Path, suffix: &[u8]) {
    let mut bytes = fs::read(path).unwrap();
    bytes.extend_from_slice(suffix);
    fs::write(path, bytes).unwrap();
}

#[test]
fn rejects_a_committed_abi_v1_downgrade() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    fs::write(fixture.target_dir.join("ABI_VERSION"), "1\n").unwrap();
    assert_eq!(
        fixture.validate().unwrap_err(),
        "source policy requires production op-auth ABI 3"
    );
}

#[test]
fn rejects_archive_substitution() {
    let fixture = Fixture::new();
    append(&fixture.target_dir.join(ARTIFACT), b"tampered");
    assert_eq!(
        fixture.validate().unwrap_err(),
        "artifact SHA-256 does not match"
    );
}

#[test]
fn accepts_adopted_signed_hardened_abi_v3_provenance() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    let validated = fixture.validate().unwrap();
    assert_eq!(validated.abi_version, 3);
    assert!(validated.signed_provenance);
}

#[test]
fn rejects_a_committed_abi_v2_downgrade() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    fs::write(fixture.target_dir.join("ABI_VERSION"), "2\n").unwrap();
    assert_eq!(
        fixture.validate().unwrap_err(),
        "source policy requires production op-auth ABI 3"
    );
}

#[test]
fn rejects_tampered_signed_provenance() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    append(
        &fixture.target_dir.join("PROVENANCE"),
        b"# unsigned suffix\n",
    );
    assert_eq!(
        fixture.validate().unwrap_err(),
        "provenance digest does not match the adopted release matrix"
    );
}

#[test]
fn rejects_a_version_mismatch_between_metadata_and_signed_provenance() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    fs::write(fixture.target_dir.join("VERSION"), "1.0.1\n").unwrap();
    assert_eq!(
        fixture.validate().unwrap_err(),
        "release matrix does not describe the selected artifact"
    );
}

#[test]
fn accepts_a_signed_artifact_version_independent_of_the_package_version() {
    assert_ne!(VERSION, env!("CARGO_PKG_VERSION"));
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    assert!(fixture.validate().unwrap().signed_provenance);
}

#[test]
fn rejects_a_same_key_signed_release_matrix_not_adopted_by_source_policy() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    let path = fixture.prebuilt_root.join("RELEASE-MANIFEST");
    let changed = fs::read_to_string(&path)
        .unwrap()
        .replacen("version=1.0.0", "version=0.9.0", 1);
    fs::write(&path, changed.as_bytes()).unwrap();
    let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
    fs::write(
        fixture.prebuilt_root.join("RELEASE-MANIFEST.sig"),
        hex(&signing_key.sign(changed.as_bytes()).to_bytes()),
    )
    .unwrap();
    assert_eq!(
        fixture.validate().unwrap_err(),
        "release matrix is not adopted by source policy"
    );
}

#[test]
fn rejects_a_substituted_release_public_key() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    let attacker_key = SigningKey::from_bytes(&[9_u8; 32]);
    fs::write(
        fixture.prebuilt_root.join("PROVENANCE_PUBKEY"),
        hex(&attacker_key.verifying_key().to_bytes()),
    )
    .unwrap();
    assert_eq!(
        fixture.validate().unwrap_err(),
        "release provenance public key is not adopted by source policy"
    );
}

#[test]
fn rejects_a_tampered_release_matrix_signature() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    fs::write(
        fixture.prebuilt_root.join("RELEASE-MANIFEST.sig"),
        format!("{}\n", "00".repeat(64)),
    )
    .unwrap();
    assert_eq!(
        fixture.validate().unwrap_err(),
        "signed release matrix signature is invalid"
    );
}

#[test]
fn rejects_hardening_substitution() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v3();
    append(
        &fixture.target_dir.join("HARDENING-ATTESTATION"),
        b"tampered=true\n",
    );
    assert_eq!(
        fixture.validate().unwrap_err(),
        "hardening digest does not match the adopted release matrix"
    );
}

#[test]
fn validates_every_committed_target_archive() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let policy_path = crate_root.join("AUTH-RELEASE-POLICY");
    let prebuilt_root = crate_root.join("prebuilt");
    let mut validated_targets = 0_usize;
    for entry in fs::read_dir(&prebuilt_root).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_dir() {
            continue;
        }
        let target = entry.file_name().into_string().unwrap();
        let artifact = if target.ends_with("-pc-windows-msvc") {
            "op_auth.lib"
        } else {
            "libop_auth.a"
        };
        if !entry.path().join(artifact).is_file() {
            continue;
        }
        let validated = validate_prebuilt(
            &policy_path,
            &prebuilt_root,
            &entry.path(),
            &target,
            artifact,
        )
        .unwrap_or_else(|error| panic!("{target}: {error}"));
        if validated.abi_version >= 2 {
            assert!(
                validated.signed_provenance,
                "{target}: ABI-v2+ must have verified provenance"
            );
        }
        validated_targets += 1;
    }
    assert!(
        validated_targets > 0,
        "the committed prebuilt matrix must not be empty"
    );
}
