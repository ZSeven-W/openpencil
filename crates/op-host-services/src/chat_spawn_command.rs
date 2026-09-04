//! Blocking command construction shared by bounded CLI probes.

use std::path::Path;
use std::process::Command;

/// Rust's Windows launcher handles resolved `.cmd` / `.bat` paths, but a
/// `.ps1` needs an explicit PowerShell host. Other targets spawn directly.
pub(crate) fn build_blocking_command(binary: &Path, args: &[&str]) -> Command {
    #[cfg(windows)]
    if is_powershell_script(binary.to_string_lossy().as_ref()) {
        let mut command = Command::new("powershell.exe");
        command
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-File")
            .arg(binary)
            .args(args)
            .env("PATH", super::runtime_path_for_binary(binary));
        return command;
    }

    let mut command = Command::new(binary);
    command
        .args(args)
        .env("PATH", super::runtime_path_for_binary(binary));
    command
}

#[cfg(any(windows, test))]
pub(super) fn is_powershell_script(binary: &str) -> bool {
    Path::new(binary)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ps1"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_shim_detection_is_extension_exact() {
        assert!(is_powershell_script(r"C:\Users\x\bin\opencode.ps1"));
        assert!(is_powershell_script("/tmp/OPENCODE.PS1"));
        assert!(!is_powershell_script(r"C:\Users\x\bin\opencode.cmd"));
        assert!(!is_powershell_script("opencode"));
    }
}
