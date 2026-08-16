//! Validation shared by the build script and provenance regression tests.
//!
//! Committed production archives fail closed unless source policy adopts their
//! complete ABI-v3 release matrix and their exact bytes and hardening declaration
//! are covered by Ed25519 signatures rooted in the repository release key.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

pub const HARDENING_PROFILE_V1: &str = "op-auth-hardened-v1";
/// Signed but deliberately un-obfuscated (and un-hardened) release profile. The
/// archive is Ed25519-signed and ABI-pinned, but its private Rust symbol name
/// strings, source/build paths, and debug sections are NOT scrubbed. It is an
/// explicit, signature-bound declaration of a lower anti-reversing bar.
pub const SIGNED_UNOBFUSCATED_PROFILE_V1: &str = "op-auth-signed-unobfuscated-v1";

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

fn is_accepted_hardening_profile(value: &str) -> bool {
    matches!(value, HARDENING_PROFILE_V1 | SIGNED_UNOBFUSCATED_PROFILE_V1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedPrebuilt {
    pub abi_version: u32,
    pub signed_provenance: bool,
    pub archive_sha256: [u8; 32],
}

#[derive(Debug)]
pub struct ProvenanceError(&'static str);

struct AdoptionPolicy {
    public_key: String,
    release_manifest_sha256: String,
    source_revision: String,
    build_id: String,
}

struct ReleaseTargetRow {
    artifact: String,
    archive_sha256: String,
    provenance_sha256: String,
    hardening_sha256: String,
}

struct ReleaseManifest {
    version: String,
    source_revision: String,
    build_id: String,
    selected: ReleaseTargetRow,
}

impl fmt::Display for ProvenanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

pub fn validate_prebuilt(
    policy_path: &Path,
    prebuilt_root: &Path,
    target_dir: &Path,
    target: &str,
    artifact_name: &str,
) -> Result<ValidatedPrebuilt, ProvenanceError> {
    let artifact_path = target_dir.join(artifact_name);
    let artifact = fs::read(&artifact_path)
        .map_err(|_| ProvenanceError("artifact is missing or unreadable"))?;
    let digest = Sha256::digest(&artifact);
    let actual_sha256 = format!("{digest:x}");
    let archive_sha256: [u8; 32] = digest.into();
    let expected_sha256 = read_trimmed(&target_dir.join("SHA256"), "SHA256 is missing")?;
    if !valid_hex(&expected_sha256, 64) || !actual_sha256.eq_ignore_ascii_case(&expected_sha256) {
        return Err(ProvenanceError("artifact SHA-256 does not match"));
    }

    let version = read_trimmed(&target_dir.join("VERSION"), "VERSION is missing")?;
    if !valid_artifact_version(&version) {
        return Err(ProvenanceError("artifact VERSION is invalid"));
    }

    let policy = read_adoption_policy(policy_path)?;
    let abi_version = read_optional_abi(&target_dir.join("ABI_VERSION"))?;
    if abi_version != 3 {
        return Err(ProvenanceError(
            "source policy requires production op-auth ABI 3",
        ));
    }
    let public_key_hex = read_trimmed(
        &prebuilt_root.join("PROVENANCE_PUBKEY"),
        "release provenance public key is missing",
    )?;
    if public_key_hex != policy.public_key {
        return Err(ProvenanceError(
            "release provenance public key is not adopted by source policy",
        ));
    }

    let release_manifest_bytes = fs::read(prebuilt_root.join("RELEASE-MANIFEST"))
        .map_err(|_| ProvenanceError("signed release matrix is missing"))?;
    let release_manifest_digest = format!("{:x}", Sha256::digest(&release_manifest_bytes));
    if release_manifest_digest != policy.release_manifest_sha256 {
        return Err(ProvenanceError(
            "release matrix is not adopted by source policy",
        ));
    }
    let release_signature = read_trimmed(
        &prebuilt_root.join("RELEASE-MANIFEST.sig"),
        "signed release matrix signature is missing",
    )?;
    verify_signature(
        &release_manifest_bytes,
        &release_signature,
        &public_key_hex,
        "signed release matrix signature is invalid",
    )?;
    let release_manifest = parse_release_manifest(&release_manifest_bytes, target)?;
    if release_manifest.source_revision != policy.source_revision
        || release_manifest.build_id != policy.build_id
    {
        return Err(ProvenanceError(
            "release matrix identity does not match source policy",
        ));
    }
    if release_manifest.version != version
        || release_manifest.selected.artifact != artifact_name
        || release_manifest.selected.archive_sha256 != actual_sha256
    {
        return Err(ProvenanceError(
            "release matrix does not describe the selected artifact",
        ));
    }

    let manifest_bytes = fs::read(target_dir.join("PROVENANCE"))
        .map_err(|_| ProvenanceError("ABI-v2+ provenance manifest is missing"))?;
    if format!("{:x}", Sha256::digest(&manifest_bytes))
        != release_manifest.selected.provenance_sha256
    {
        return Err(ProvenanceError(
            "provenance digest does not match the adopted release matrix",
        ));
    }
    let hardening_bytes = fs::read(target_dir.join("HARDENING-ATTESTATION"))
        .map_err(|_| ProvenanceError("hardening attestation is missing"))?;
    let hardening_digest = format!("{:x}", Sha256::digest(&hardening_bytes));
    if hardening_digest != release_manifest.selected.hardening_sha256 {
        return Err(ProvenanceError(
            "hardening digest does not match the adopted release matrix",
        ));
    }

    let manifest = parse_manifest(&manifest_bytes)?;
    require_field(&manifest, "format", "1")?;
    require_field(&manifest, "target", target)?;
    require_field(&manifest, "artifact", artifact_name)?;
    // VERSION identifies this signed private artifact matrix. ABI compatibility,
    // rather than the public application's package version, decides whether the
    // archive can be consumed by a later OpenPencil source revision.
    require_field(&manifest, "version", &version)?;
    require_field(&manifest, "abi", &abi_version.to_string())?;
    require_field(&manifest, "sha256", &actual_sha256.to_ascii_lowercase())?;
    if !is_accepted_hardening_profile(field(&manifest, "hardening")?) {
        return Err(ProvenanceError(
            "provenance manifest declares an unrecognized hardening profile",
        ));
    }
    require_field(&manifest, "source_revision", &policy.source_revision)?;
    require_field(
        &manifest,
        "build_id",
        &format!("{}.a3.{hardening_digest}", policy.build_id),
    )?;
    let signature_hex = read_trimmed(
        &target_dir.join("PROVENANCE.sig"),
        "ABI-v2+ provenance signature is missing",
    )?;
    verify_signature(
        &manifest_bytes,
        &signature_hex,
        &public_key_hex,
        "ABI-v2+ provenance signature verification failed",
    )?;

    Ok(ValidatedPrebuilt {
        abi_version,
        signed_provenance: true,
        archive_sha256,
    })
}

fn read_optional_abi(path: &Path) -> Result<u32, ProvenanceError> {
    if !path.is_file() {
        return Ok(1);
    }
    let value = read_trimmed(path, "ABI_VERSION is unreadable")?;
    match value.parse::<u32>() {
        Ok(version @ 1..=3) => Ok(version),
        _ => Err(ProvenanceError("ABI_VERSION is unsupported")),
    }
}

fn read_trimmed(path: &Path, missing: &'static str) -> Result<String, ProvenanceError> {
    let value = fs::read_to_string(path).map_err(|_| ProvenanceError(missing))?;
    Ok(value.trim().to_owned())
}

fn read_adoption_policy(path: &Path) -> Result<AdoptionPolicy, ProvenanceError> {
    let text = fs::read_to_string(path)
        .map_err(|_| ProvenanceError("auth release adoption policy is missing"))?;
    if text.contains('\r') || !text.ends_with('\n') {
        return Err(ProvenanceError("auth release adoption policy is malformed"));
    }
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 6 || lines[0] != "format=op-auth-release-policy-v1" || lines[1] != "abi=3" {
        return Err(ProvenanceError("auth release adoption policy is malformed"));
    }
    let public_key = exact_value(lines[2], "public_key=")?;
    let release_manifest_sha256 = exact_value(lines[3], "release_manifest_sha256=")?;
    let source_revision = exact_value(lines[4], "source_revision=")?;
    let build_id = exact_value(lines[5], "build_id=")?;
    if !valid_lower_hex(public_key, 64)
        || !valid_lower_hex(release_manifest_sha256, 64)
        || !valid_lower_hex(source_revision, 40)
        || validate_build_id(build_id).is_err()
    {
        return Err(ProvenanceError("auth release adoption policy is malformed"));
    }
    Ok(AdoptionPolicy {
        public_key: public_key.to_owned(),
        release_manifest_sha256: release_manifest_sha256.to_owned(),
        source_revision: source_revision.to_owned(),
        build_id: build_id.to_owned(),
    })
}

fn parse_release_manifest(
    bytes: &[u8],
    selected_target: &str,
) -> Result<ReleaseManifest, ProvenanceError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ProvenanceError("signed release matrix is not UTF-8"))?;
    if text.contains('\r') || !text.ends_with('\n') {
        return Err(ProvenanceError("signed release matrix is malformed"));
    }
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 17
        || lines[0] != "format=op-auth-release-matrix-v1"
        || lines[2] != "abi=3"
        || lines[6] != "target_count=10"
    {
        return Err(ProvenanceError("signed release matrix is malformed"));
    }
    let version = exact_value(lines[1], "version=")?;
    let source_revision = exact_value(lines[3], "source_revision=")?;
    let openpencil_revision = exact_value(lines[4], "openpencil_revision=")?;
    let build_id = exact_value(lines[5], "build_id=")?;
    if !valid_artifact_version(version)
        || !valid_lower_hex(source_revision, 40)
        || !valid_lower_hex(openpencil_revision, 40)
        || validate_build_id(build_id).is_err()
    {
        return Err(ProvenanceError("signed release matrix is malformed"));
    }

    let mut selected = None;
    for (index, expected_target) in RELEASE_TARGETS.iter().enumerate() {
        let row = parse_release_target_row(lines[index + 7], expected_target)?;
        if *expected_target == selected_target {
            selected = Some(row);
        }
    }
    let selected = selected.ok_or(ProvenanceError(
        "selected target is absent from signed release matrix",
    ))?;
    Ok(ReleaseManifest {
        version: version.to_owned(),
        source_revision: source_revision.to_owned(),
        build_id: build_id.to_owned(),
        selected,
    })
}

fn parse_release_target_row(
    line: &str,
    expected_target: &str,
) -> Result<ReleaseTargetRow, ProvenanceError> {
    let fields: Vec<_> = line.split('|').collect();
    if fields.len() != 5 || fields[0] != format!("target={expected_target}") {
        return Err(ProvenanceError(
            "signed release matrix target row is malformed",
        ));
    }
    let artifact = exact_value(fields[1], "artifact=")?;
    let expected_artifact = if expected_target.ends_with("-pc-windows-msvc") {
        "op_auth.lib"
    } else {
        "libop_auth.a"
    };
    let archive_sha256 = exact_value(fields[2], "sha256=")?;
    let provenance_sha256 = exact_value(fields[3], "provenance_sha256=")?;
    let hardening_sha256 = exact_value(fields[4], "hardening_sha256=")?;
    if artifact != expected_artifact
        || !valid_lower_hex(archive_sha256, 64)
        || !valid_lower_hex(provenance_sha256, 64)
        || !valid_lower_hex(hardening_sha256, 64)
    {
        return Err(ProvenanceError(
            "signed release matrix target row is malformed",
        ));
    }
    Ok(ReleaseTargetRow {
        artifact: artifact.to_owned(),
        archive_sha256: archive_sha256.to_owned(),
        provenance_sha256: provenance_sha256.to_owned(),
        hardening_sha256: hardening_sha256.to_owned(),
    })
}

fn exact_value<'a>(line: &'a str, prefix: &str) -> Result<&'a str, ProvenanceError> {
    line.strip_prefix(prefix)
        .filter(|value| !value.is_empty() && !value.contains('='))
        .ok_or(ProvenanceError(
            "signed release metadata field is malformed",
        ))
}

fn verify_signature(
    payload: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
    error: &'static str,
) -> Result<(), ProvenanceError> {
    let public_key = decode_fixed::<32>(public_key_hex).ok_or(ProvenanceError(error))?;
    let signature = decode_fixed::<64>(signature_hex).ok_or(ProvenanceError(error))?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key).map_err(|_| ProvenanceError(error))?;
    verifying_key
        .verify_strict(payload, &Signature::from_bytes(&signature))
        .map_err(|_| ProvenanceError(error))
}

fn parse_manifest(bytes: &[u8]) -> Result<BTreeMap<String, String>, ProvenanceError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ProvenanceError("provenance manifest is not UTF-8"))?;
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(ProvenanceError("provenance manifest line is malformed"));
        };
        if key.is_empty()
            || value.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(ProvenanceError("provenance manifest field is malformed"));
        }
        if fields.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(ProvenanceError("provenance manifest field is duplicated"));
        }
    }
    const EXPECTED_FIELDS: [&str; 9] = [
        "abi",
        "artifact",
        "build_id",
        "format",
        "hardening",
        "sha256",
        "source_revision",
        "target",
        "version",
    ];
    if fields.len() != EXPECTED_FIELDS.len()
        || !EXPECTED_FIELDS
            .iter()
            .all(|expected| fields.contains_key(*expected))
    {
        return Err(ProvenanceError(
            "provenance manifest fields do not match the signed schema",
        ));
    }
    Ok(fields)
}

fn require_field(
    fields: &BTreeMap<String, String>,
    name: &str,
    expected: &str,
) -> Result<(), ProvenanceError> {
    if field(fields, name)? == expected {
        Ok(())
    } else {
        Err(ProvenanceError(
            "provenance manifest does not describe the selected artifact",
        ))
    }
}

fn field<'a>(fields: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, ProvenanceError> {
    fields
        .get(name)
        .map(String::as_str)
        .ok_or(ProvenanceError("provenance manifest field is missing"))
}

fn validate_build_id(value: &str) -> Result<(), ProvenanceError> {
    if !value.is_empty()
        && value.len() <= 59
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !value.contains("..")
        && !value.ends_with(".lock")
    {
        Ok(())
    } else {
        Err(ProvenanceError("provenance build id is invalid"))
    }
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_artifact_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || !value.is_ascii() {
        return false;
    }
    let split = value
        .bytes()
        .position(|byte| matches!(byte, b'+' | b'-'))
        .unwrap_or(value.len());
    let core = &value[..split];
    let suffix = &value[split..];
    let mut components = core.split('.');
    let valid_core = (0..3).all(|_| {
        components
            .next()
            .is_some_and(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    }) && components.next().is_none();
    let valid_suffix = suffix.is_empty()
        || (suffix.len() > 1
            && suffix[1..]
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-')));
    valid_core && valid_suffix
}

fn decode_fixed<const N: usize>(value: &str) -> Option<[u8; N]> {
    if !valid_hex(value, N * 2) {
        return None;
    }
    let mut decoded = [0_u8; N];
    for (index, byte) in decoded.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = (hex_nibble(value.as_bytes()[offset])? << 4)
            | hex_nibble(value.as_bytes()[offset + 1])?;
    }
    Some(decoded)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}
