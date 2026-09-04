//! Link the prebuilt proprietary auth library when one exists for the
//! current target; otherwise the crate compiles its stub and the account
//! UI stays hidden. Open-source checkouts therefore always build.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "dev_prebuilt_policy.rs"]
mod dev_prebuilt_policy;
#[path = "prebuilt_link_compat.rs"]
mod prebuilt_link_compat;
#[path = "prebuilt_provenance.rs"]
mod prebuilt_provenance;

const DEV_ARCHIVE_ENV: &str = "OPENPENCIL_DEV_OP_AUTH_ARCHIVE";
const DEV_ABI_VERSION_ENV: &str = "OPENPENCIL_DEV_OP_AUTH_ABI_VERSION";
const DEV_FEATURE_ENV: &str = "CARGO_FEATURE_DEV_PREBUILT";

fn main() {
    println!("cargo:rustc-check-cfg=cfg(op_auth_prebuilt)");
    println!("cargo:rustc-check-cfg=cfg(op_auth_collab_ticket_prebuilt)");
    println!("cargo:rustc-check-cfg=cfg(op_auth_collab_relay_token_prebuilt)");
    println!("cargo:rustc-check-cfg=cfg(op_auth_development_prebuilt)");
    println!("cargo:rerun-if-changed=prebuilt");
    println!("cargo:rerun-if-changed=AUTH-RELEASE-POLICY");
    println!("cargo:rerun-if-changed=prebuilt_link_compat.rs");
    println!("cargo:rerun-if-env-changed={DEV_ARCHIVE_ENV}");
    println!("cargo:rerun-if-env-changed={DEV_ABI_VERSION_ENV}");

    let target = env::var("TARGET").unwrap_or_default();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // MSVC static libraries follow the `<name>.lib` convention; every
    // other target uses the Unix `lib<name>.a` archive name.
    let artifact = if target.ends_with("-pc-windows-msvc") {
        "op_auth.lib"
    } else {
        "libop_auth.a"
    };

    let development = development_prebuilt(artifact);
    let (prebuilt_dir, abi_version, development_override, signed_provenance, expected_sha256) =
        if let Some((directory, abi_version)) = development {
            println!(
                "cargo:warning=using unsigned local op-auth ABI {abi_version} \
                 archive for a debug build"
            );
            println!("cargo:rustc-cfg=op_auth_development_prebuilt");
            (directory, abi_version, true, false, None)
        } else {
            let prebuilt_dir = manifest_dir.join("prebuilt").join(&target);
            let artifact_path = prebuilt_dir.join(artifact);
            if !artifact_path.is_file() {
                return;
            }
            let validated = match prebuilt_provenance::validate_prebuilt(
                &manifest_dir.join("AUTH-RELEASE-POLICY"),
                &manifest_dir.join("prebuilt"),
                &prebuilt_dir,
                &target,
                artifact,
            ) {
                Ok(validated) => validated,
                Err(error) => {
                    println!("cargo:warning=ignoring op-auth prebuilt: {error}");
                    return;
                }
            };
            (
                prebuilt_dir,
                validated.abi_version,
                false,
                validated.signed_provenance,
                Some(validated.archive_sha256),
            )
        };

    println!("cargo:rustc-cfg=op_auth_prebuilt");
    if abi_version >= 2 {
        assert!(
            development_override || signed_provenance,
            "production ABI-v2+ archives require signed provenance"
        );
        println!("cargo:rustc-cfg=op_auth_collab_ticket_prebuilt");
    }
    if abi_version >= 3 {
        println!("cargo:rustc-cfg=op_auth_collab_relay_token_prebuilt");
    }
    println!("cargo:rustc-env=OP_AUTH_PREBUILT_ABI_VERSION={abi_version}");
    let link_dir = rust_host_link_directory(&target, &prebuilt_dir, artifact, expected_sha256);
    println!("cargo:rustc-link-search=native={}", link_dir.display());
    // `-bundle`: keep the archive out of this crate's rlib and hand it to
    // the final link instead. Bundled foreign objects would otherwise be
    // fed to thin-LTO in release builds, which fails with "failed to get
    // bitcode from object file for LTO".
    println!("cargo:rustc-link-lib=static:-bundle=op_auth");

    // System libraries the static library's TLS/network stack expects.
    if target.contains("-apple-") {
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else if target.contains("windows-msvc") {
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=bcrypt");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=ntdll");
    }
}

fn rust_host_link_directory(
    target: &str,
    prebuilt_dir: &Path,
    artifact: &str,
    expected_sha256: Option<[u8; 32]>,
) -> PathBuf {
    if !target.contains("linux") && !target.ends_with("-pc-windows-msvc") {
        return prebuilt_dir.to_path_buf();
    }

    let link_dir = PathBuf::from(
        env::var("OUT_DIR").expect("Cargo provides OUT_DIR to the op-auth build script"),
    )
    .join("rust-host-link");
    let report = prebuilt_link_compat::stage_archive_for_rust_host(
        &prebuilt_dir.join(artifact),
        &link_dir.join(artifact),
        expected_sha256,
    )
    .unwrap_or_else(|error| panic!("failed to stage op-auth for Rust host linking: {error}"));
    if report.renamed_occurrences != 0 {
        println!(
            "cargo:warning=isolated {} bundled Rust personality symbol occurrence(s) \
             in the temporary op-auth link archive",
            report.renamed_occurrences
        );
    }
    link_dir
}

fn development_prebuilt(artifact: &str) -> Option<(PathBuf, u32)> {
    let feature_enabled = env::var_os(DEV_FEATURE_ENV).is_some();
    let archive = env::var_os(DEV_ARCHIVE_ENV);
    let abi_version = env::var_os(DEV_ABI_VERSION_ENV);
    let (archive, abi_version) = match (feature_enabled, archive, abi_version) {
        (_, None, None) => return None,
        (true, Some(archive), Some(abi_version)) => (archive, abi_version),
        (false, _, _) => {
            panic!("{DEV_ARCHIVE_ENV} requires the op-auth-bridge/dev-prebuilt feature");
        }
        _ => {
            panic!(
                "op-auth-bridge/dev-prebuilt requires {DEV_ARCHIVE_ENV} and {DEV_ABI_VERSION_ENV}"
            );
        }
    };

    let profile = env::var("PROFILE").unwrap_or_default();
    dev_prebuilt_policy::require_debug_profile(&profile)
        .unwrap_or_else(|error| panic!("{DEV_ARCHIVE_ENV}: {error}"));

    let requested_archive = PathBuf::from(archive);
    assert!(
        requested_archive.is_absolute(),
        "{DEV_ARCHIVE_ENV} must be an absolute path"
    );
    let metadata = fs::symlink_metadata(&requested_archive)
        .unwrap_or_else(|_| panic!("{DEV_ARCHIVE_ENV} is missing or unreadable"));
    assert!(
        metadata.file_type().is_file(),
        "{DEV_ARCHIVE_ENV} must select a regular non-symlink file"
    );
    let archive = fs::canonicalize(&requested_archive)
        .unwrap_or_else(|_| panic!("{DEV_ARCHIVE_ENV} is missing or unreadable"));
    assert!(
        archive.is_file(),
        "{DEV_ARCHIVE_ENV} must select a regular file"
    );
    assert_eq!(
        archive.file_name(),
        Some(OsStr::new(artifact)),
        "{DEV_ARCHIVE_ENV} must select the target's {artifact}"
    );

    let abi_version = abi_version
        .into_string()
        .ok()
        .and_then(|value| dev_prebuilt_policy::parse_development_abi_version(&value).ok())
        .unwrap_or_else(|| {
            panic!(
                "{DEV_ABI_VERSION_ENV} must be an integer between {} and {}",
                dev_prebuilt_policy::MIN_DEVELOPMENT_ABI_VERSION,
                dev_prebuilt_policy::MAX_DEVELOPMENT_ABI_VERSION
            )
        });
    let directory = PathBuf::from(
        env::var("OUT_DIR").expect("Cargo provides OUT_DIR to the op-auth build script"),
    );
    fs::copy(&archive, directory.join(artifact))
        .unwrap_or_else(|_| panic!("failed to stage {DEV_ARCHIVE_ENV} in OUT_DIR"));
    println!("cargo:rerun-if-changed={}", requested_archive.display());
    println!("cargo:rerun-if-changed={}", archive.display());
    Some((directory, abi_version))
}
