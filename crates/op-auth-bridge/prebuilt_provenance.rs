//! Validation shared by the build script and provenance regression tests.
//!
//! ABI-v1 archives predate signed provenance and remain usable only as a
//! compatibility lane. ABI-v2 and later artifacts fail closed unless their
//! exact bytes and hardening declaration are covered by an Ed25519 signature
//! rooted in the repository's release public key.

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

impl fmt::Display for ProvenanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

pub fn validate_prebuilt(
    prebuilt_root: &Path,
    target_dir: &Path,
    target: &str,
    artifact_name: &str,
    package_version: &str,
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
    if version.is_empty() || version.len() > 64 || !version.is_ascii() {
        return Err(ProvenanceError("artifact VERSION is invalid"));
    }

    let abi_version = read_optional_abi(&target_dir.join("ABI_VERSION"))?;
    if abi_version == 1 {
        return Ok(ValidatedPrebuilt {
            abi_version,
            signed_provenance: false,
            archive_sha256,
        });
    }
    if version != package_version {
        return Err(ProvenanceError(
            "artifact VERSION does not match the package version",
        ));
    }

    let manifest_bytes = fs::read(target_dir.join("PROVENANCE"))
        .map_err(|_| ProvenanceError("ABI-v2+ provenance manifest is missing"))?;
    let manifest = parse_manifest(&manifest_bytes)?;
    require_field(&manifest, "format", "1")?;
    require_field(&manifest, "target", target)?;
    require_field(&manifest, "artifact", artifact_name)?;
    require_field(&manifest, "version", package_version)?;
    require_field(&manifest, "abi", &abi_version.to_string())?;
    require_field(&manifest, "sha256", &actual_sha256.to_ascii_lowercase())?;
    if !is_accepted_hardening_profile(field(&manifest, "hardening")?) {
        return Err(ProvenanceError(
            "provenance manifest declares an unrecognized hardening profile",
        ));
    }
    validate_source_revision(field(&manifest, "source_revision")?)?;
    validate_build_id(field(&manifest, "build_id")?)?;

    let public_key_hex = read_trimmed(
        &prebuilt_root.join("PROVENANCE_PUBKEY"),
        "release provenance public key is missing",
    )?;
    let signature_hex = read_trimmed(
        &target_dir.join("PROVENANCE.sig"),
        "ABI-v2+ provenance signature is missing",
    )?;
    let public_key = decode_fixed::<32>(&public_key_hex)
        .ok_or(ProvenanceError("release provenance public key is invalid"))?;
    let signature = decode_fixed::<64>(&signature_hex)
        .ok_or(ProvenanceError("ABI-v2+ provenance signature is invalid"))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| ProvenanceError("release provenance public key is invalid"))?;
    verifying_key
        .verify_strict(&manifest_bytes, &Signature::from_bytes(&signature))
        .map_err(|_| ProvenanceError("ABI-v2+ provenance signature verification failed"))?;

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

fn validate_source_revision(value: &str) -> Result<(), ProvenanceError> {
    if matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ProvenanceError(
            "provenance source revision must be a full hexadecimal digest",
        ))
    }
}

fn validate_build_id(value: &str) -> Result<(), ProvenanceError> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        Ok(())
    } else {
        Err(ProvenanceError("provenance build id is invalid"))
    }
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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
