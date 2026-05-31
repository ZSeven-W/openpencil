//! Hidden `--tcc-selftest` diagnostic mode.
//!
//! macOS gates access to the protected user folders (Desktop /
//! Documents / Downloads) behind TCC, keyed on the launching app's
//! code-signing identity. A bare or ad-hoc-signed binary is denied;
//! a bundle signed with the same Developer ID + bundle id as an
//! already-granted app inherits that grant (TCC matches on the
//! designated requirement, not the exact binary).
//!
//! `openpencil-desktop --tcc-selftest <dir> [outfile]` probes whether
//! THIS process can read `<dir>` — via `read_dir`, the exact syscall
//! git's working-tree scan makes — and writes an `OK …` / `ERR …`
//! line to `outfile` (default `$TMPDIR/openpencil-tcc-selftest.txt`)
//! and to stderr, then asks `main` to exit. Launching the bundle with
//! `open(1)` routes through LaunchServices, so the app becomes its own
//! TCC responsible process; this confirms the signed bundle's folder
//! access without any GUI interaction.

use std::path::PathBuf;

/// When `--tcc-selftest` is present on argv, run the folder-access
/// probe and return `true` so `main` exits; otherwise return `false`
/// and let the normal GUI path continue.
pub fn run_cli_if_requested() -> bool {
    let args: Vec<String> = std::env::args().collect();
    let Some(pos) = args.iter().position(|a| a == "--tcc-selftest") else {
        return false;
    };

    let dir = args
        .get(pos + 1)
        .cloned()
        .unwrap_or_else(|| "/".to_string());
    let out: PathBuf = args
        .get(pos + 2)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("openpencil-tcc-selftest.txt"));

    let report = match std::fs::read_dir(&dir) {
        // `read_dir` returns a lazy iterator; pull one entry so the
        // directory is actually opened (the call TCC gates).
        Ok(mut entries) => {
            let saw = entries.next().is_some();
            format!("OK read_dir({dir}) accessible (non_empty={saw})")
        }
        Err(e) => format!("ERR read_dir({dir}) {e}"),
    };

    let _ = std::fs::write(&out, format!("{report}\n"));
    eprintln!("tcc-selftest: {report}");
    eprintln!("tcc-selftest: wrote {}", out.display());
    true
}
