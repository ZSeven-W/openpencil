//! Shared process-spawn helpers for the CLI chat transports
//! (`chat_subprocess.rs` stdio bridges + `chat_http_server.rs`
//! OpenCode server bridge): binary resolution across well-known
//! install locations, cross-platform `Command` construction, and
//! exit-status labeling. Extracted from `chat_subprocess.rs` so the
//! HTTP-server transport doesn't have to re-implement (or reach into)
//! the stdio bridge's internals.

use std::path::PathBuf;
use std::sync::OnceLock;

use tokio::process::Command;

/// The user's LOGIN-SHELL `PATH`, resolved once and cached.
///
/// A GUI-launched app (Dock / Finder) inherits launchd's minimal PATH —
/// no `/opt/homebrew/bin`, no nvm/volta shims — so Node-based CLIs
/// (`codex` is `#!/usr/bin/env node`) fail to spawn even when
/// `find_binary` locates the script itself: the shebang can't resolve
/// `node`. Terminal launches (`cargo run`) inherit the full shell PATH,
/// which is why "works in dev, breaks in the installed app". The
/// standard fix (VS Code, Electron `fix-path`): ask the user's login
/// shell for its PATH once and graft it onto ours.
///
/// Returns `None` on Windows (PATH comes from the registry, GUI apps
/// get the real one) or when the shell probe fails/times out.
pub fn login_shell_path() -> Option<&'static str> {
    login_shell_env()?.get("PATH").map(String::as_str)
}

/// The user's full LOGIN-SHELL environment, captured once and cached
/// in memory (never persisted). Complements [`login_shell_path`]: a
/// GUI launch also misses `ANTHROPIC_API_KEY`-style exports living in
/// the user's shell rc, which flips the Claude transport from API-key
/// to the subscription OAuth credential — and Anthropic rejects
/// non-Claude-Code-shaped requests on that credential with
/// `403 Request not allowed`. `None` on Windows or when the probe
/// fails.
pub fn login_shell_env() -> Option<&'static std::collections::BTreeMap<String, String>> {
    static CACHE: OnceLock<Option<std::collections::BTreeMap<String, String>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            #[cfg(windows)]
            {
                None
            }
            #[cfg(not(windows))]
            {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
                let out = std::process::Command::new(&shell)
                    .args(["-lc", "command env"])
                    .stdin(std::process::Stdio::null())
                    .output()
                    .ok()?;
                if !out.status.success() {
                    return None;
                }
                let text = String::from_utf8(out.stdout).ok()?;
                let mut map = std::collections::BTreeMap::new();
                for line in text.lines() {
                    // Multi-line values lose their tail here — acceptable:
                    // the consumers (PATH, API keys, base URLs) are
                    // single-line by construction.
                    if let Some((key, value)) = line.split_once('=') {
                        if !key.is_empty() {
                            map.insert(key.to_string(), value.to_string());
                        }
                    }
                }
                (!map.is_empty()).then_some(map)
            }
        })
        .as_ref()
}

/// One env var through the process env first, then the captured
/// login-shell env. Blank values read as absent at both tiers.
pub fn env_var_with_login_shell(name: &str) -> Option<String> {
    if let Some(value) = std::env::var_os(name) {
        let value = value.to_string_lossy().into_owned();
        if !value.trim().is_empty() {
            return Some(value);
        }
    }
    let value = login_shell_env()?.get(name)?;
    (!value.trim().is_empty()).then(|| value.clone())
}

/// Process `PATH` merged with the login shell's (login entries first,
/// current entries appended, deduped). This is what spawned CLIs get as
/// their `PATH` so `#!/usr/bin/env node` shebangs resolve under a GUI
/// launch. Falls back to the current PATH when the shell probe failed.
pub fn effective_path_env() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    match login_shell_path() {
        Some(login) => merge_path_lists(login, &current),
        None => current,
    }
}

/// Repair THIS PROCESS's `PATH` from the login shell — call once at GUI
/// startup, before any worker threads spawn. A Dock-launched app inherits
/// launchd's minimal PATH; grafting the login-shell PATH onto the process
/// itself means every child (CLI subprocesses, the Claude agent SDK's
/// `env::vars()` baseline) inherits the repaired PATH naturally — WITHOUT
/// passing PATH through per-request env maps, which the agent SDK's
/// dangerous-env blocklist rejects outright.
pub fn repair_gui_process_path() {
    let merged = effective_path_env();
    if merged.is_empty() || std::env::var("PATH").as_deref() == Ok(merged.as_str()) {
        return;
    }
    // Single-threaded startup call site; the desktop main calls this before
    // the winit loop or any chat worker exists.
    std::env::set_var("PATH", merged);
}

/// Login-shell entries first, current-process entries appended, deduped.
/// Split out of [`effective_path_env`] so the merge is unit-testable
/// without spawning a real login shell.
fn merge_path_lists(login: &str, current: &str) -> String {
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut seen = std::collections::BTreeSet::new();
    let mut merged: Vec<&str> = Vec::new();
    for dir in login.split(sep).chain(current.split(sep)) {
        if !dir.is_empty() && seen.insert(dir) {
            merged.push(dir);
        }
    }
    merged.join(&sep.to_string())
}

/// Search for `name` on PATH, then in well-known per-platform install
/// locations for Node-based CLIs (npm / pnpm / yarn / bun globals,
/// nvm, volta). Returns the resolved absolute path, or `name` itself
/// as a fallback so `build_command` can still attempt a bare-name
/// spawn (errors surface as a normal spawn-failure `ChatDelta::Error`).
///
/// Cross-platform: each branch only probes paths that exist on that
/// OS so we don't pay for filesystem-stat misses on the wrong OS.
pub fn find_binary(name: &str) -> String {
    // PATH-relative entries first (cross-platform) — against the
    // MERGED login-shell PATH so a GUI launch sees the same binaries
    // a terminal launch does (nvm/volta/homebrew shims included).
    {
        let path_env = effective_path_env();
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
        // Login-shell PATH for the child: a Dock-launched app's own
        // PATH has no nvm/homebrew shims, so a Node-based CLI script
        // (`#!/usr/bin/env node`) found via `find_binary`'s fallback
        // list would still die on shebang resolution ("CLI not
        // responding"). The merged PATH makes GUI and terminal
        // launches spawn identically.
        cmd.env("PATH", effective_path_env());
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

#[cfg(test)]
mod login_shell_tests {
    use super::*;

    #[test]
    fn merge_prefers_login_entries_and_dedupes() {
        let merged = merge_path_lists(
            "/opt/homebrew/bin:/Users/x/.nvm/versions/node/v20/bin:/usr/bin",
            "/usr/bin:/bin",
        );
        assert_eq!(
            merged, "/opt/homebrew/bin:/Users/x/.nvm/versions/node/v20/bin:/usr/bin:/bin",
            "login entries lead, duplicates collapse, current-only entries survive"
        );
    }

    #[test]
    fn merge_handles_empty_segments() {
        assert_eq!(merge_path_lists("", "/usr/bin"), "/usr/bin");
        assert_eq!(merge_path_lists("/usr/bin", ""), "/usr/bin");
    }
}
