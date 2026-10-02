//! Keep Cargo test executables away from installed user settings, including
//! custom CARGO_TARGET_DIR locations. Explicit mobile roots are resolved by
//! the caller first and therefore still take priority over this fallback.

use std::path::{Path, PathBuf};

pub(super) fn settings_dir(
    executable: &Path,
    temporary_root: &Path,
    process_id: u32,
) -> Option<PathBuf> {
    let under_deps = executable.parent()?.file_name()? == "deps";
    let file = executable.file_name()?.to_str()?;
    let file = file.strip_suffix(".exe").unwrap_or(file);
    let (crate_name, hash) = file.rsplit_once('-')?;
    let cargo_hash = hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit());
    (under_deps && !crate_name.is_empty() && cargo_hash)
        .then(|| temporary_root.join(format!("openpencil-test-settings-{process_id}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_test_paths_are_isolated_independently_of_target_directory_name() {
        let temporary_root = Path::new("/private/tmp");
        for executable in [
            "/repo/target/debug/deps/op_engine_ffi-e24129924b1c14b4",
            "/repo/gate-target/debug/deps/op_engine_ffi-e24129924b1c14b4",
            "/build-cache/any-name/aarch64-apple-darwin/release/deps/works_persistence-a123456789abcdef",
            "/private/tmp/temporary-build/debug/deps/editor-a123456789abcdef",
            "/build/custom/debug/deps/editor-a123456789abcdef.exe",
        ] {
            assert_eq!(
                settings_dir(Path::new(executable), temporary_root, 42),
                Some(temporary_root.join("openpencil-test-settings-42")),
                "{executable}",
            );
        }
    }

    #[test]
    fn installed_executables_and_non_cargo_deps_files_keep_their_normal_settings() {
        for executable in [
            "/Applications/OpenPencil.app/Contents/MacOS/openpencil-desktop",
            "/usr/local/bin/openpencil-desktop",
            "/repo/target/debug/openpencil-desktop",
            "/opt/app/deps/openpencil-desktop",
            "/opt/app/deps/editor-not_a_hash_value",
            "/opt/app/deps/editor-abcd",
            "/opt/app/deps/nested/editor-a123456789abcdef",
        ] {
            assert_eq!(
                settings_dir(Path::new(executable), Path::new("/tmp"), 42),
                None,
                "{executable}"
            );
        }
    }
}
