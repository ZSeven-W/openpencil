//! Shared process-spawn helpers for the CLI chat transports
//! (`chat_subprocess.rs` stdio bridges + `chat_http_server.rs`
//! OpenCode server bridge): binary resolution across well-known
//! install locations, cross-platform `Command` construction, and
//! exit-status labeling. Extracted from `chat_subprocess.rs` so the
//! HTTP-server transport doesn't have to re-implement (or reach into)
//! the stdio bridge's internals.

use std::path::PathBuf;

use tokio::process::Command;

/// Search for `name` on PATH, then in well-known per-platform install
/// locations for Node-based CLIs (npm / pnpm / yarn / bun globals,
/// nvm, volta). Returns the resolved absolute path, or `name` itself
/// as a fallback so `build_command` can still attempt a bare-name
/// spawn (errors surface as a normal spawn-failure `ChatDelta::Error`).
///
/// Cross-platform: each branch only probes paths that exist on that
/// OS so we don't pay for filesystem-stat misses on the wrong OS.
pub fn find_binary(name: &str) -> String {
    // PATH-relative entries first (cross-platform).
    if let Ok(path_env) = std::env::var("PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for dir in path_env.split(sep).filter(|s| !s.is_empty()) {
            let candidate = std::path::Path::new(dir).join(name);
            if candidate.is_file() {
                return candidate.to_string_lossy().into();
            }
            // Windows: PATHEXT-style suffix probe so we find
            // `claude.cmd` / `claude.exe` / `claude.bat` even when the
            // user typed the bare name.
            #[cfg(windows)]
            {
                for ext in &[".exe", ".cmd", ".bat", ".ps1"] {
                    let mut with_ext = candidate.clone();
                    with_ext.set_extension(&ext[1..]);
                    if with_ext.is_file() {
                        return with_ext.to_string_lossy().into();
                    }
                }
            }
        }
    }
    // Fall back through well-known install locations. Mirrors
    // bartolli/anthropic-agent-sdk's `find_cli` for parity with the
    // reference implementation.
    let candidates = well_known_install_paths(name);
    for path in candidates {
        if path.is_file() {
            return path.to_string_lossy().into();
        }
    }
    name.into()
}

fn well_known_install_paths(name: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir();
    let mut out: Vec<PathBuf> = Vec::new();
    #[cfg(unix)]
    {
        if let Some(h) = home.clone() {
            // `~/.opencode/bin` is OpenCode's own installer target —
            // TS `opencode-client.ts` probes it first among its
            // non-PATH candidates.
            out.push(h.join(".opencode/bin").join(name));
            out.push(h.join(".npm-global/bin").join(name));
            out.push(h.join(".local/bin").join(name));
            out.push(h.join(".bun/bin").join(name));
            out.push(h.join(".volta/bin").join(name));
            out.push(h.join("node_modules/.bin").join(name));
            out.push(h.join(".yarn/bin").join(name));
        }
        out.push(PathBuf::from("/usr/local/bin").join(name));
        out.push(PathBuf::from("/opt/homebrew/bin").join(name));
    }
    #[cfg(windows)]
    {
        let _ = home; // not used directly on Windows
        if let Ok(appdata) = std::env::var("APPDATA") {
            for ext in &["cmd", "exe", "bat", "ps1"] {
                out.push(
                    PathBuf::from(&appdata)
                        .join("npm")
                        .join(format!("{name}.{ext}")),
                );
            }
        }
        if let Ok(localapp) = std::env::var("LOCALAPPDATA") {
            for ext in &["cmd", "exe", "bat", "ps1"] {
                out.push(
                    PathBuf::from(&localapp)
                        .join("Programs")
                        .join(name)
                        .join(format!("{name}.{ext}")),
                );
            }
        }
    }
    out
}

/// Build a `tokio::process::Command` that spawns `binary` with `args`.
/// Handles the three desktop platforms identically wherever possible,
/// and papers over the well-known cross-platform binary-lookup gaps:
///
/// - **macOS / Linux**: a bare command name resolves via the usual
///   PATH execvp lookup. We forward straight to `Command::new`.
/// - **Windows**: Win32 `CreateProcessW` does **not** honor PATHEXT,
///   so a bare `claude` only spawns when an exact `claude` (no
///   extension) is on PATH. npm / bun / Volta / scoop / winget all
///   ship Node-based CLIs as `claude.cmd` / `claude.bat` / `claude.ps1`
///   shims. To make those work we route through `cmd /c <binary>`
///   when the binary doesn't already look like a fully-resolved path
///   ending in `.exe`. The binary names we ship from `for_cli` are
///   hardcoded constants (no user-controlled metacharacters) so this
///   is safe from shell injection. Users passing a custom binary via
///   `with_binary` are responsible for not embedding shell payload.
///
/// On every platform stdin / stdout / stderr are piped, and the child
/// is detached from any controlling terminal (`process_group(0)` on
/// Unix so Ctrl-C in the OP terminal doesn't kill the CLI; on Windows
/// `creation_flags(CREATE_NO_WINDOW)` so spawning the CLI doesn't
/// pop a console window for users running the GUI build).
pub fn build_command(binary: &str, args: &[String]) -> Command {
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW from winbase.h — keeps the console hidden
        // when OpenPencil launches from a non-console parent (e.g.,
        // double-click on the .exe from Explorer).
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let bare = std::path::Path::new(binary);
        let has_path_sep = bare
            .parent()
            .map(|p| !p.as_os_str().is_empty())
            .unwrap_or(false);
        let looks_like_exe = bare
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        if has_path_sep || looks_like_exe {
            let mut cmd = Command::new(binary);
            cmd.args(args);
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd
        } else {
            // Route through `cmd /c` so PATHEXT (.cmd / .bat / .ps1)
            // expansion kicks in. /c runs the command and exits.
            let mut cmd = Command::new("cmd");
            cmd.arg("/c").arg(binary).args(args);
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd
        }
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new(binary);
        cmd.args(args);
        // process_group(0) puts the child in its own group so signals
        // sent to OP's pgroup (e.g., Ctrl-C in the terminal that
        // launched the GUI) don't propagate to the CLI mid-stream.
        // The chat bridge has its own kill-on-receiver-drop path, so
        // we never depend on signal propagation for cleanup.
        #[cfg(unix)]
        {
            cmd.process_group(0);
        }
        cmd
    }
}

/// Apply CREATE_NO_WINDOW to a blocking `std::process::Command` so
/// background CLI probes (model discovery, provider version checks)
/// don't flash console windows once the desktop binary runs detached
/// from the console subsystem (`windows_subsystem = "windows"`).
/// No-op off Windows.
pub(crate) fn hide_console_window(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW from winbase.h — same flag build_command
        // applies to the streaming tokio commands.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

/// Stringify an `ExitStatus` for chat error reporting. Cross-platform:
/// on Unix `.code()` is `None` when killed by signal — show the signal
/// number instead; on Windows `.code()` is always populated.
pub fn exit_status_label(status: &std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        return code.to_string();
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return format!("signal {sig}");
        }
    }
    "?".into()
}
