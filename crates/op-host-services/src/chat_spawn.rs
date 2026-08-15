//! Shared process-spawn helpers for the CLI chat transports
//! (`chat_subprocess.rs` stdio bridges + `chat_http_server.rs`
//! OpenCode server bridge): binary resolution across well-known
//! install locations, cross-platform `Command` construction, and
//! exit-status labeling. Extracted from `chat_subprocess.rs` so the
//! HTTP-server transport doesn't have to re-implement (or reach into)
//! the stdio bridge's internals.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;
#[cfg(not(windows))]
use std::time::Duration;

use tokio::process::Command;

/// Hard budget for the login-shell environment probe. The probe runs an
/// INTERACTIVE shell, so it executes the user's full rc — which is
/// allowed to block forever (a prompt framework waiting on the network, a
/// stale `nvm use`, a hung completion script). The probe fires from
/// `main` before any window exists, so an unbounded wait hangs the whole
/// launch with no UI to explain it. A healthy `-ilc` dump measures ~2.2 s
/// on a developer machine; a run past this budget is a broken rc, and
/// falling back to the unrepaired process env beats never starting.
#[cfg(not(windows))]
const LOGIN_SHELL_PROBE_TIMEOUT: Duration = Duration::from_secs(8);

/// Environment variables grafted from the login shell onto this process,
/// beyond `PATH`. A GUI (Dock / Finder) launch inherits launchd's
/// environment, which carries none of the user's proxy exports, so any
/// probe that must reach the network (`agy models`, `grok models`) stalls
/// behind a proxy-only route until its own deadline fires.
///
/// Deliberately a fixed allowlist rather than a bulk copy: the login
/// shell's full environment holds unrelated host secrets, and this
/// process's env is inherited by every child we spawn.
const GRAFTED_ENV_KEYS: [&str; 8] = [
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
];

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
/// fails or times out.
pub fn login_shell_env() -> Option<&'static BTreeMap<String, String>> {
    static CACHE: OnceLock<Option<BTreeMap<String, String>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            #[cfg(windows)]
            {
                None
            }
            #[cfg(not(windows))]
            {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
                // `-i` is load-bearing. A pure login shell (`-l`) reads
                // only `.zprofile` / `.profile`, while homebrew, nvm and
                // proxy exports conventionally live in the INTERACTIVE rc
                // (`.zshrc` / `.bashrc`) — so `-lc` came back missing
                // exactly the entries a GUI launch needs, and Node-based
                // CLIs died on shebang resolution while networked probes
                // hung with no proxy.
                match capture_env_dump(&shell, &["-ilc", "command env"], LOGIN_SHELL_PROBE_TIMEOUT)
                {
                    EnvDump::Completed(text) => {
                        let map = parse_env_dump(&text);
                        (!map.is_empty()).then_some(map)
                    }
                    EnvDump::TimedOut | EnvDump::Failed => None,
                }
            }
        })
        .as_ref()
}

/// Result of one bounded `<shell> -ilc "command env"` run.
#[cfg(not(windows))]
enum EnvDump {
    /// The shell exited successfully inside the budget; carries stdout.
    Completed(String),
    /// Still running at the deadline (a blocking rc) — killed.
    TimedOut,
    /// Never produced usable output: spawn failed, pipes missing,
    /// non-zero exit, or non-UTF-8 stdout.
    Failed,
}

/// Run `program args…` with a hard deadline and return its stdout.
///
/// The command spec is a parameter rather than baked in so tests can
/// exercise the success / timeout / failure branches against a trivial
/// `/bin/sh` script instead of launching a real interactive shell (which
/// would run the developer's own rc).
#[cfg(not(windows))]
fn capture_env_dump(program: &str, args: &[&str], timeout: Duration) -> EnvDump {
    use std::process::Stdio;
    use std::time::Instant;

    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        // An interactive rc is allowed to be noisy on stderr (a
        // `command not found` from a half-installed completion). Drain
        // it into the void: only stdout is parsed, and an undrained
        // pipe would let a chatty rc deadlock this probe.
        .stderr(Stdio::piped());
    let Ok(mut child) = command.spawn() else {
        return EnvDump::Failed;
    };
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return EnvDump::Failed;
    };
    let stdout_reader = crate::cli_probe_support::PipeCapture::spawn(stdout);
    let stderr_reader = crate::cli_probe_support::PipeCapture::spawn(stderr);
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = stdout_reader.finish();
                drop(stderr_reader);
                if !status.success() {
                    return EnvDump::Failed;
                }
                return match String::from_utf8(out) {
                    Ok(text) => EnvDump::Completed(text),
                    Err(_) => EnvDump::Failed,
                };
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return EnvDump::TimedOut;
            }
        }
    }
}

/// Parse `env` output into a map.
///
/// Multi-line values lose their tail — acceptable: the consumers (PATH,
/// API keys, base URLs, proxy URLs) are single-line by construction. A
/// key containing whitespace is not an environment entry but rc chatter
/// that happened to print an `=`, so it is dropped.
#[cfg(any(not(windows), test))]
fn parse_env_dump(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.is_empty() || key.contains(char::is_whitespace) {
            continue;
        }
        map.insert(key.to_string(), value.to_string());
    }
    map
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

/// The PATH a GUI process inherits when nothing has customised it — what
/// launchd hands a Dock/Finder launch. Membership in this set is the
/// FACT that separates "this process has no PATH of its own" from "the
/// user arranged this PATH deliberately"; it is not a heuristic guess
/// about the entries' meaning.
const SYSTEM_DEFAULT_PATH_DIRS: &[&str] = &["/usr/bin", "/bin", "/usr/sbin", "/sbin"];

/// Whether `current` carries no information beyond the system default —
/// every entry is a stock system directory (or there are none at all).
fn path_is_system_default(current: &str) -> bool {
    current
        .split(':')
        .filter(|dir| !dir.is_empty())
        .all(|dir| SYSTEM_DEFAULT_PATH_DIRS.contains(&dir.trim_end_matches('/')))
}

/// Merge order, split out of [`effective_path_env`] so the decision is
/// unit-testable without mutating the test process's environment.
///
/// The two launch modes want opposite things, and the difference between
/// them is a fact we can check rather than guess:
///
/// - **Dock/Finder launch** — the process PATH is launchd's stock set, so
///   it expresses no intent. The login shell's PATH is the only place the
///   user's toolchain exists; it must lead, or `#!/usr/bin/env node`
///   shebangs resolve against a system node (the 2026-07-25 exit-127
///   fix).
/// - **Terminal launch (or any customised PATH)** — the process PATH *is*
///   the user's current intent, ordering included. Letting the login
///   shell lead here silently overrides which binary we pick.
///
/// That second case shipped as a real defect: with two `codex` installs
/// (homebrew 0.133.0 and nvm 0.146.0) and a `.zshrc` that puts homebrew
/// first, the app resolved the OLD one even though the user's terminal
/// resolved the new one — so the model picker showed a stale catalog, and
/// the old binary rewrote the shared `~/.codex/models_cache.json` with
/// its own outdated list (measured 2026-07-31). Login entries are still
/// appended, so nothing that was reachable before becomes unreachable.
fn merged_path_for(login: &str, current: &str) -> String {
    if path_is_system_default(current) {
        merge_path_lists(login, current)
    } else {
        merge_path_lists(current, login)
    }
}

/// Process `PATH` merged with the login shell's, deduped. This is what
/// spawned CLIs get as their `PATH` so `#!/usr/bin/env node` shebangs
/// resolve under a GUI launch. Which list leads depends on whether the
/// process PATH carries any intent — see [`merged_path_for`]. Falls back
/// to the current PATH when the shell probe failed.
pub fn effective_path_env() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    match login_shell_path() {
        Some(login) => merged_path_for(login, &current),
        None => current,
    }
}

/// Repair THIS PROCESS's environment from the login shell — call once at
/// GUI startup, before any worker threads spawn. A Dock-launched app
/// inherits launchd's minimal environment; grafting the login-shell
/// values onto the process itself means every child (CLI subprocesses,
/// the Claude agent SDK's `env::vars()` baseline) inherits them naturally
/// — WITHOUT passing them through per-request env maps, which the agent
/// SDK's dangerous-env blocklist rejects outright.
///
/// Two repairs, both narrow: the merged `PATH` (so `#!/usr/bin/env node`
/// shebangs resolve) and the [`GRAFTED_ENV_KEYS`] proxy allowlist (so
/// networked probes are not stranded on a proxy-only route).
pub fn repair_gui_process_env() {
    repair_process_path();
    repair_process_proxy_vars();
}

fn repair_process_path() {
    let merged = effective_path_env();
    if merged.is_empty() || std::env::var("PATH").as_deref() == Ok(merged.as_str()) {
        return;
    }
    // Single-threaded startup call site; the desktop main calls this before
    // the winit loop or any chat worker exists.
    std::env::set_var("PATH", merged);
}

fn repair_process_proxy_vars() {
    let Some(login) = login_shell_env() else {
        return;
    };
    for (key, value) in proxy_grafts(login, |key| std::env::var_os(key).is_some()) {
        std::env::set_var(key, value);
    }
}

/// Which allowlisted proxy variables to graft, given the login-shell dump
/// and a predicate reporting what this process already carries. A key
/// already present in the process wins — an explicit launch-time setting
/// (including an intentional empty `no_proxy`) must not be overwritten by
/// a shell rc. Split out of [`repair_process_proxy_vars`] so the
/// allowlist and the no-overwrite rule are testable without mutating the
/// test process's own environment.
fn proxy_grafts<F>(login: &BTreeMap<String, String>, is_set: F) -> Vec<(&str, &str)>
where
    F: Fn(&str) -> bool,
{
    GRAFTED_ENV_KEYS
        .iter()
        .filter_map(|key| {
            if is_set(key) {
                return None;
            }
            let value = login.get(*key)?;
            (!value.trim().is_empty()).then_some((*key, value.as_str()))
        })
        .collect()
}

/// Login-shell entries first, current-process entries appended, deduped.
/// Split out of [`effective_path_env`] so the merge is unit-testable
/// without spawning a real login shell.
fn merge_path_lists(login: &str, current: &str) -> String {
    // A login-shell PATH is only collected on Unix; Windows returns `None`
    // from `login_shell_env` and keeps its registry-provided PATH unchanged.
    // Keep this merge target-independent so its Unix inputs are not parsed as
    // semicolon-delimited merely because the test binary runs on Windows.
    let sep = ':';
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

/// Dangerous environment variables that should never be propagated to
/// the spawned CLI: linker preload paths can hijack execution; PATH
/// can substitute a malicious binary; runtime-library paths
/// (NODE_OPTIONS, PYTHONPATH, etc.) can inject code into Node-based
/// CLIs. Mirrors bartolli/anthropic-agent-sdk's `DANGEROUS_ENV_VARS`.
/// Used for Claude Code + custom binaries; Codex gets the stricter
/// TS allowlist (`chat_subprocess_quirks`). Returns the
/// env-var pairs the child will receive (parent env minus the
/// dangerous names — preserving every safe var so node version
/// managers like nvm / volta still pick the right Node). Moved here
/// from `chat_subprocess.rs` to keep that spine under the 800-line
/// cap — pure code motion.
pub(crate) fn scrubbed_child_env() -> Vec<(String, String)> {
    const DANGEROUS: &[&str] = &[
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "NODE_OPTIONS",
        "PYTHONPATH",
        "PERL5LIB",
        "RUBYLIB",
    ];
    std::env::vars()
        .filter(|(k, _)| !DANGEROUS.iter().any(|d| k.eq_ignore_ascii_case(d)))
        .collect()
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

    #[test]
    fn stock_process_path_lets_the_login_shell_lead() {
        // Dock/Finder launch: launchd's PATH expresses no intent, so the
        // login shell supplies the toolchain (the exit-127 fix).
        assert!(path_is_system_default("/usr/bin:/bin:/usr/sbin:/sbin"));
        assert!(path_is_system_default(""));
        // A trailing slash is the same directory, not a customisation.
        assert!(path_is_system_default("/usr/bin/:/bin"));
        let merged = merged_path_for(
            "/Users/x/.nvm/versions/node/v20/bin:/opt/homebrew/bin:/usr/bin",
            "/usr/bin:/bin:/usr/sbin:/sbin",
        );
        assert!(
            merged.starts_with("/Users/x/.nvm/versions/node/v20/bin:/opt/homebrew/bin"),
            "login toolchain leads a stock PATH, got {merged}"
        );
    }

    #[test]
    fn customised_process_path_wins_over_the_login_shell() {
        // The 2026-07-31 codex defect, as a test: two installs of the same
        // CLI, the login shell ordering homebrew first, the process PATH
        // ordering nvm first. The process PATH is the user's live intent
        // and must decide which binary we resolve.
        let login = "/opt/homebrew/bin:/Users/x/.nvm/versions/node/v20/bin:/usr/bin";
        let current = "/Users/x/.nvm/versions/node/v20/bin:/opt/homebrew/bin:/usr/bin:/bin";
        assert!(!path_is_system_default(current));
        let merged = merged_path_for(login, current);
        let nvm = merged
            .split(':')
            .position(|dir| dir.contains(".nvm"))
            .expect("nvm entry survives");
        let brew = merged
            .split(':')
            .position(|dir| dir == "/opt/homebrew/bin")
            .expect("homebrew entry survives");
        assert!(nvm < brew, "the process PATH's order decides, got {merged}");
    }

    #[test]
    fn login_only_entries_are_still_appended_to_a_customised_path() {
        // Reordering must never make something unreachable: a directory
        // only the login shell knows about still lands on the merged PATH.
        let merged = merged_path_for("/opt/only-in-login/bin:/usr/bin", "/Users/x/bin:/usr/bin");
        assert!(merged.starts_with("/Users/x/bin"), "got {merged}");
        assert!(
            merged.split(':').any(|dir| dir == "/opt/only-in-login/bin"),
            "login-only entry survives, got {merged}"
        );
    }

    #[test]
    fn parse_env_dump_keeps_entries_and_drops_rc_chatter() {
        let dump = concat!(
            "PATH=/opt/homebrew/bin:/usr/bin\n",
            "https_proxy=http://127.0.0.1:7890\n",
            // A noisy interactive rc printing to stdout: not an entry.
            "_encode: command not found = maybe\n",
            // Continuation of a multi-line value; the tail is dropped.
            "some trailing prose\n",
            "=novalue\n",
        );
        let map = parse_env_dump(dump);
        assert_eq!(
            map.get("PATH").map(String::as_str),
            Some("/opt/homebrew/bin:/usr/bin")
        );
        assert_eq!(
            map.get("https_proxy").map(String::as_str),
            Some("http://127.0.0.1:7890")
        );
        assert_eq!(map.len(), 2, "only real KEY=VALUE lines survive: {map:?}");
    }

    fn login_dump() -> BTreeMap<String, String> {
        [
            ("http_proxy", "http://127.0.0.1:7890"),
            ("https_proxy", "http://127.0.0.1:7890"),
            ("no_proxy", "   "),
            ("ANTHROPIC_API_KEY", "sk-secret"),
            ("PATH", "/opt/homebrew/bin"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }

    #[test]
    fn proxy_grafts_only_allowlisted_keys_that_are_unset() {
        let login = login_dump();
        let grafts = proxy_grafts(&login, |_| false);
        assert_eq!(
            grafts,
            vec![
                ("http_proxy", "http://127.0.0.1:7890"),
                ("https_proxy", "http://127.0.0.1:7890"),
            ],
            "blank values are skipped, and nothing outside the allowlist \
             (ANTHROPIC_API_KEY / PATH) is grafted"
        );
    }

    #[test]
    fn proxy_grafts_never_overwrite_a_key_this_process_already_carries() {
        let login = login_dump();
        let grafts = proxy_grafts(&login, |key| key == "http_proxy");
        assert_eq!(grafts, vec![("https_proxy", "http://127.0.0.1:7890")]);
        assert!(proxy_grafts(&login, |_| true).is_empty());
    }
}

#[cfg(all(test, not(windows)))]
mod env_dump_tests {
    use super::*;

    /// Deliberately short: these assert on the deadline firing, not on
    /// any real shell finishing.
    const TIMEOUT: Duration = Duration::from_millis(400);

    #[test]
    fn capture_reads_stdout_and_tolerates_stderr_noise() {
        let script = "printf 'A=1\\nB=2\\n'; printf '_encode: command not found\\n' >&2";
        let EnvDump::Completed(text) = capture_env_dump("/bin/sh", &["-c", script], TIMEOUT) else {
            panic!("a script that exits inside the budget must complete");
        };
        let map = parse_env_dump(&text);
        assert_eq!(map.get("A").map(String::as_str), Some("1"));
        assert_eq!(map.get("B").map(String::as_str), Some("2"));
    }

    #[test]
    fn capture_times_out_instead_of_hanging_on_a_blocking_rc() {
        let started = std::time::Instant::now();
        let dump = capture_env_dump("/bin/sh", &["-c", "sleep 30"], TIMEOUT);
        assert!(
            matches!(dump, EnvDump::TimedOut),
            "a shell still running at the deadline must be killed, not awaited"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the deadline, not the child, decides when the probe returns"
        );
    }

    #[test]
    fn capture_reports_failure_for_nonzero_exit_and_missing_program() {
        assert!(matches!(
            capture_env_dump("/bin/sh", &["-c", "printf 'A=1\\n'; exit 3"], TIMEOUT),
            EnvDump::Failed
        ));
        assert!(matches!(
            capture_env_dump(
                "/nonexistent/shell-that-is-not-installed",
                &["-ilc"],
                TIMEOUT
            ),
            EnvDump::Failed
        ));
    }

    #[test]
    fn a_dump_with_no_entries_falls_back_to_the_unrepaired_env() {
        // The fallback contract behind `login_shell_env`: an empty parse
        // must read as "no login-shell env", so `effective_path_env`
        // keeps the current process PATH instead of clearing it.
        let EnvDump::Completed(text) =
            capture_env_dump("/bin/sh", &["-c", "printf 'no entries here\\n'"], TIMEOUT)
        else {
            panic!("the script exits inside the budget");
        };
        assert!(parse_env_dump(&text).is_empty());
        assert_eq!(
            effective_path_env().is_empty(),
            std::env::var("PATH").unwrap_or_default().is_empty()
        );
    }
}
