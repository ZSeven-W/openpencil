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
const VERSION: &str = "0.8.3";
const SOURCE_REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
    target_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let unique = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "op-auth-provenance-{}-{unique}",
            std::process::id()
        ));
        let target_dir = root.join(TARGET);
        fs::create_dir_all(&target_dir).unwrap();
        let artifact = b"deterministic archive fixture";
        fs::write(target_dir.join(ARTIFACT), artifact).unwrap();
        fs::write(
            target_dir.join("SHA256"),
            format!("{:x}\n", Sha256::digest(artifact)),
        )
        .unwrap();
        fs::write(target_dir.join("VERSION"), format!("{VERSION}\n")).unwrap();
        Self { root, target_dir }
    }

    fn validate(&self) -> Result<prebuilt_provenance::ValidatedPrebuilt, String> {
        validate_prebuilt(&self.root, &self.target_dir, TARGET, ARTIFACT, VERSION)
            .map_err(|error| error.to_string())
    }

    fn make_signed_abi_v2(&self) {
        fs::write(self.target_dir.join("ABI_VERSION"), "2\n").unwrap();
        let sha256 = fs::read_to_string(self.target_dir.join("SHA256")).unwrap();
        let manifest = format!(
            "format=1\n\
             target={TARGET}\n\
             artifact={ARTIFACT}\n\
             version={VERSION}\n\
             abi=2\n\
             sha256={}\n\
             hardening={HARDENING_PROFILE_V1}\n\
             source_revision={SOURCE_REVISION}\n\
             build_id=private-ci-42\n",
            sha256.trim()
        );
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        fs::write(
            self.root.join("PROVENANCE_PUBKEY"),
            hex(&signing_key.verifying_key().to_bytes()),
        )
        .unwrap();
        fs::write(self.target_dir.join("PROVENANCE"), manifest.as_bytes()).unwrap();
        fs::write(
            self.target_dir.join("PROVENANCE.sig"),
            hex(&signing_key.sign(manifest.as_bytes()).to_bytes()),
        )
        .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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
fn accepts_checksum_pinned_legacy_abi_v1() {
    let fixture = Fixture::new();
    let validated = fixture.validate().unwrap();
    assert_eq!(validated.abi_version, 1);
    assert!(!validated.signed_provenance);
}

#[test]
fn rejects_legacy_archive_substitution() {
    let fixture = Fixture::new();
    append(&fixture.target_dir.join(ARTIFACT), b"tampered");
    assert_eq!(
        fixture.validate().unwrap_err(),
        "artifact SHA-256 does not match"
    );
}

#[test]
fn accepts_signed_hardened_abi_v2_provenance() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v2();
    let validated = fixture.validate().unwrap();
    assert_eq!(validated.abi_version, 2);
    assert!(validated.signed_provenance);
}

#[test]
fn rejects_unsigned_abi_v2_artifacts() {
    let fixture = Fixture::new();
    fs::write(fixture.target_dir.join("ABI_VERSION"), "2\n").unwrap();
    assert_eq!(
        fixture.validate().unwrap_err(),
        "ABI-v2 provenance manifest is missing"
    );
}

#[test]
fn rejects_tampered_signed_provenance() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v2();
    append(
        &fixture.target_dir.join("PROVENANCE"),
        b"# unsigned suffix\n",
    );
    assert_eq!(
        fixture.validate().unwrap_err(),
        "ABI-v2 provenance signature verification failed"
    );
}

#[test]
fn rejects_a_stale_artifact_version() {
    let fixture = Fixture::new();
    fixture.make_signed_abi_v2();
    let stale_version = format!("{}-stale\n", env!("CARGO_PKG_VERSION"));
    fs::write(fixture.target_dir.join("VERSION"), stale_version).unwrap();
    assert_eq!(
        fixture.validate().unwrap_err(),
        "artifact VERSION does not match the package version"
    );
}

#[test]
fn validates_every_committed_target_archive() {
    let prebuilt_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("prebuilt");
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
            &prebuilt_root,
            &entry.path(),
            &target,
            artifact,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap_or_else(|error| panic!("{target}: {error}"));
        if validated.abi_version >= 2 {
            assert!(
                validated.signed_provenance,
                "{target}: ABI-v2 must have verified provenance"
            );
        }
        validated_targets += 1;
    }
    assert!(
        validated_targets > 0,
        "the committed prebuilt matrix must not be empty"
    );
}
