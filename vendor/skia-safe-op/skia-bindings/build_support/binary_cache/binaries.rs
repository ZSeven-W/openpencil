//! Support for exporting and building prebuilt binaries.

use super::{git, github_actions};
use crate::build_support::{binaries_config, cargo, features::Features};
use flate2::read::GzDecoder;
use std::{
    collections::HashSet,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

/// Export binaries if we are inside a git repository _and_
/// the artifact staging directory is set.
/// The git repository test is important to support package verifications.
pub fn should_export() -> Option<PathBuf> {
    git::half_hash()?;
    github_actions::artifact_staging_directory()
}

/// Export the binaries to a target directory.
///
/// `source_files` are additional files from below skia-bindings/ that are copied to the target
/// directory.
pub fn export(
    config: &binaries_config::BinariesConfiguration,
    source_files: &[(&str, &str)],
    target_dir: &Path,
) -> io::Result<()> {
    let half_hash = git::half_hash().expect("failed to retrieve the git hash");
    let key = config.key(&half_hash);

    let export_dir = prepare_export_directory(&key, target_dir)?;

    for source_file in source_files {
        let (src, dst) = source_file;
        fs::copy(PathBuf::from(src), export_dir.join(PathBuf::from(dst)))?;
    }

    config.export(&export_dir)
}

/// Prepares the binaries directory and sets the tag.txt and key.txt
/// file.
fn prepare_export_directory(key: &str, artifacts: &Path) -> io::Result<PathBuf> {
    let binaries = artifacts.join("skia-binaries");
    fs::create_dir_all(&binaries)?;

    // this is primarily for GitHub Actions to know the tag and the key of the binaries, but they
    // can stay inside the archive.

    {
        let mut tag_file = fs::File::create(binaries.join("tag.txt")).unwrap();
        tag_file.write_all(cargo::package_version().as_bytes())?;
    }
    {
        let mut key_file = fs::File::create(binaries.join("key.txt")).unwrap();
        key_file.write_all(key.as_bytes())?;
    }

    Ok(binaries)
}

/// The name of the tar archive without any keys or file extensions. This is also the name
/// of the subdirectory that is created when the archive is unpacked.
pub const ARCHIVE_NAME: &str = "skia-binaries";

/// Key generation function.
/// The resulting string will uniquely identify the generated binaries.
/// Every part of the key is separated by '-' and no grouping / enclosing characters are used
/// because GitHub strips them from the filenames (tested "<>[]{}()",
/// and also Unicode characters seem to be stripped).
pub fn key(repository_short_hash: &str, features: &Features, skia_debug: bool) -> String {
    let mut components = Vec::new();

    fn group(str: impl AsRef<str>) -> String {
        // no grouping syntax ATM
        str.as_ref().to_string()
    }

    // SHA hash of the rust-skia repository.
    components.push(repository_short_hash.to_owned());

    // The target architecture, vendor, system, and abi if specified.
    components.push(group(cargo::target().to_string()));

    // Features, sorted.
    if !features.is_empty() {
        components.push(group(features.to_key()));
    };

    if cargo::target_crt_static() {
        components.push("static".into());
    }

    if skia_debug {
        components.push("debug".into())
    }

    components.join("-")
}

/// Prepare the final download URL for the prebuilt binaries archive.
pub fn download_url(url_template: String, tag: impl AsRef<str>, key: impl AsRef<str>) -> String {
    #[allow(unknown_lints)]
    #[allow(clippy::literal_string_with_formatting_args)]
    url_template
        .replace("{tag}", tag.as_ref())
        .replace("{key}", key.as_ref())
}

pub fn unpack(archive: impl Read, output_directory: &Path) -> io::Result<()> {
    let tar = GzDecoder::new(archive);
    // note: this creates the skia-bindings/ directory.
    tar::Archive::new(tar).unpack(output_directory)?;
    let binaries_dir = output_directory.join(ARCHIVE_NAME);
    let paths: Vec<PathBuf> = fs::read_dir(binaries_dir)?
        .map(|e| e.unwrap().path())
        .collect();

    // pull out all nested files.
    for path in paths {
        let name = path.file_name().unwrap();
        let target_path = output_directory.join(name);
        fs::rename(path, target_path)?
    }
    Ok(())
}
