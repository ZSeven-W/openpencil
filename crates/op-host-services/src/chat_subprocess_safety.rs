//! Fail-closed policy for third-party coding-agent CLIs.

use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use op_ai::chat_provider::CliName;

static TURN_SEQ: AtomicU64 = AtomicU64::new(0);

pub const ANTIGRAVITY_TIMEOUT: Duration = Duration::from_secs(2 * 60);
pub const GROK_TIMEOUT: Duration = Duration::from_secs(5 * 60);
pub const EXIT_GRACE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnPurpose {
    CanvasAgent,
    Generation,
}

#[cfg(test)]
#[path = "chat_subprocess_safety_antigravity_tests.rs"]
mod antigravity_tests;

impl TurnPurpose {
    fn uses_canvas_mcp(self) -> bool {
        self == Self::CanvasAgent
    }
}

pub const GROK_READ_TOOLS: &str = "read_file,grep,list_dir";
pub const GROK_MCP_ALLOW: &str = "MCPTool(openpencil__*)";

const ANTIGRAVITY_MCP_PERMISSION: &str = "mcp(openpencil/*)";
const ANTIGRAVITY_DENY_RULES: &[&str] = &[
    "command(*)",
    "unsandboxed(*)",
    "write_file(*)",
    "read_url(*)",
    "execute_url(*)",
];

pub fn antigravity_args(purpose: TurnPurpose) -> Vec<String> {
    // Antigravity's verified one-shot interface accepts the prompt through
    // `-p`; unlike Grok Build, it exposes no prompt-file option. The caller
    // therefore has to put the guarded prompt in argv for the lifetime of the
    // child. Keep the remaining containment layers (private cwd, filtered
    // environment, sandbox, and wall-clock timeout) even though argv privacy
    // cannot be provided for this CLI.
    let mut args = vec!["--sandbox".into(), "--print-timeout".into(), "90s".into()];
    if purpose == TurnPurpose::Generation {
        args.extend(["--mode".into(), "plan".into()]);
    }
    args
}

pub fn grok_args(purpose: TurnPurpose) -> Vec<String> {
    let mut args = vec![
        "--no-auto-update".into(),
        "--output-format".into(),
        "streaming-json".into(),
        "--permission-mode".into(),
        "dontAsk".into(),
        "--sandbox".into(),
        "strict".into(),
        "--max-turns".into(),
        "24".into(),
        "--no-plan".into(),
        "--no-subagents".into(),
        "--no-ask-user".into(),
        "--no-memory".into(),
        "--disable-web-search".into(),
        "--no-wait-for-background".into(),
    ];
    if purpose.uses_canvas_mcp() {
        args.extend(["--allow".into(), GROK_MCP_ALLOW.into()]);
        args.extend(["--tools".into(), GROK_READ_TOOLS.into()]);
    } else {
        args.extend(["--tools".into(), String::new()]);
    }
    args
}

const AUTOMATION_GUARD: &str = "OPENPENCIL AUTOMATION SAFETY:\n\
Use the OpenPencil MCP server only (`mcp__openpencil__*` / `openpencil/*`) to inspect or modify the canvas. \
Do not run terminal commands, write local files, browse the web, spawn subagents, or call any other MCP server. \
Never request interactive approval. If the OpenPencil MCP tools are unavailable or denied, report that failure and stop.";
const GROK_COMPAT_SETTINGS: &[u8] = br#"{"permissions":{"defaultMode":"dontAsk"}}"#;

pub struct IsolatedTurn {
    dir: PathBuf,
    prompt_file: Option<PathBuf>,
    claude_config_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    prompt: String,
}

impl IsolatedTurn {
    pub fn prepare(
        cli: Option<CliName>,
        prompt: &str,
        attachments: &[PathBuf],
    ) -> io::Result<Option<Self>> {
        let host_home = dirs::home_dir();
        Self::prepare_for(
            cli,
            prompt,
            attachments,
            TurnPurpose::CanvasAgent,
            host_home.as_deref(),
        )
    }

    pub(crate) fn prepare_generation(
        cli: Option<CliName>,
        prompt: &str,
        attachments: &[PathBuf],
    ) -> io::Result<Option<Self>> {
        let host_home = dirs::home_dir();
        Self::prepare_for(
            cli,
            prompt,
            attachments,
            TurnPurpose::Generation,
            host_home.as_deref(),
        )
    }

    #[cfg(test)]
    fn prepare_with_host_home(
        cli: Option<CliName>,
        prompt: &str,
        attachments: &[PathBuf],
        host_home: Option<&Path>,
    ) -> io::Result<Option<Self>> {
        Self::prepare_for(
            cli,
            prompt,
            attachments,
            TurnPurpose::CanvasAgent,
            host_home,
        )
    }

    fn prepare_for(
        cli: Option<CliName>,
        prompt: &str,
        attachments: &[PathBuf],
        purpose: TurnPurpose,
        host_home: Option<&Path>,
    ) -> io::Result<Option<Self>> {
        let Some(cli @ (CliName::Antigravity | CliName::GrokBuild)) = cli else {
            return Ok(None);
        };
        let dir = create_turn_dir(cli)?;

        let result = (|| {
            let mut prepared_prompt = if purpose.uses_canvas_mcp() {
                format!("{AUTOMATION_GUARD}\n\n{prompt}")
            } else {
                prompt.to_owned()
            };
            for (index, source) in attachments.iter().enumerate() {
                let file_name = source
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("attachment");
                let destination = dir.join(format!("attachment-{index}-{file_name}"));
                fs::copy(source, &destination)?;
                set_private_file(&destination)?;
                prepared_prompt = prepared_prompt.replace(
                    source.to_string_lossy().as_ref(),
                    destination.to_string_lossy().as_ref(),
                );
            }
            let prompt_file = if cli == CliName::GrokBuild {
                let path = dir.join("prompt.txt");
                fs::write(&path, &prepared_prompt)?;
                set_private_file(&path)?;
                Some(path)
            } else {
                None
            };
            let claude_config_dir = if cli == CliName::GrokBuild {
                let path = dir.join("claude-config");
                fs::create_dir(&path)?;
                set_private_dir(&path)?;
                let settings = path.join("settings.json");
                fs::write(&settings, GROK_COMPAT_SETTINGS)?;
                set_private_file(&settings)?;
                Some(path)
            } else {
                None
            };
            let home_dir = if cli == CliName::Antigravity {
                Some(prepare_antigravity_home(
                    &dir,
                    host_home,
                    purpose.uses_canvas_mcp(),
                )?)
            } else {
                None
            };
            Ok((prepared_prompt, prompt_file, claude_config_dir, home_dir))
        })();
        match result {
            Ok((prompt, prompt_file, claude_config_dir, home_dir)) => Ok(Some(Self {
                dir,
                prompt_file,
                claude_config_dir,
                home_dir,
                prompt,
            })),
            Err(error) => {
                let _ = fs::remove_dir_all(&dir);
                Err(error)
            }
        }
    }

    pub fn cwd(&self) -> &Path {
        &self.dir
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn prompt_file(&self) -> Option<&Path> {
        self.prompt_file.as_deref()
    }

    pub fn claude_config_dir(&self) -> Option<&Path> {
        self.claude_config_dir.as_deref()
    }

    fn home_dir(&self) -> Option<&Path> {
        self.home_dir.as_deref()
    }

    pub fn append_cli_args(&self, args: &mut Vec<String>) {
        let Some(home) = self.home_dir() else {
            return;
        };
        args.push(format!(
            "--gemini_dir={}",
            home.join(".gemini").to_string_lossy()
        ));
        args.push("--app_data_dir=antigravity-cli".into());
        // Point the CLI's own log into this turn's private directory. Its
        // stderr can be as uninformative as a bare "Agent execution
        // terminated due to error." while the real cause (a server-side
        // FAILED_PRECONDITION, an auth refusal) is only ever written here —
        // see `chat_subprocess_antigravity_log`. The file dies with the turn
        // directory, so this adds no retained state.
        if let Some(log) = self.log_file() {
            args.push(format!("--log-file={}", log.to_string_lossy()));
        }
    }

    /// Where this turn's CLI log lives, for the CLIs that write one.
    /// `None` for every turn without a private home (i.e. everything but
    /// Antigravity).
    pub(crate) fn log_file(&self) -> Option<PathBuf> {
        Some(self.home_dir()?.join("cli.log"))
    }
}

/// Add only per-turn configuration to the already-filtered child environment.
/// Antigravity keeps the real HOME so macOS Keychain can find its login key;
/// hidden CLI directory flags redirect all Gemini config and app data to the
/// private turn. Grok uses an isolated `CLAUDE_CONFIG_DIR`.
pub fn append_isolated_env(env: &mut Vec<(String, String)>, turn: Option<&IsolatedTurn>) {
    let Some(turn) = turn else {
        return;
    };
    if let Some(config_dir) = turn.claude_config_dir() {
        env.retain(|(key, _)| key != "CLAUDE_CONFIG_DIR");
        env.push((
            "CLAUDE_CONFIG_DIR".to_string(),
            config_dir.to_string_lossy().into_owned(),
        ));
    }
    if let Some(home) = turn.home_dir() {
        #[cfg(not(windows))]
        let safe_path = "/usr/bin:/bin:/usr/sbin:/sbin".to_string();
        #[cfg(windows)]
        let safe_path = env
            .iter()
            .find(|(key, _)| key == "SYSTEMROOT")
            .map(|(_, root)| format!(r"{root}\System32;{root}"))
            .unwrap_or_else(|| r"C:\Windows\System32;C:\Windows".to_string());
        const PRIVATE_KEYS: &[&str] = &["PATH", "TMPDIR", "TMP", "TEMP"];
        env.retain(|(key, _)| !PRIVATE_KEYS.contains(&key.as_str()));
        let value = |path: &Path| path.to_string_lossy().into_owned();
        env.extend([
            ("PATH".to_string(), safe_path),
            ("TMPDIR".to_string(), value(&home.join("tmp"))),
            ("TMP".to_string(), value(&home.join("tmp"))),
            ("TEMP".to_string(), value(&home.join("tmp"))),
        ]);
        #[cfg(target_os = "macos")]
        {
            env.retain(|(key, _)| key != "BROWSER");
            env.push(("BROWSER".to_string(), "/usr/bin/false".to_string()));
        }
    }
}

/// Build the private per-turn Antigravity HOME.
///
/// SIGNPOST, not a fix — where credentials come from on this path.
/// Every turn gets a FRESH private `--gemini_dir` (see
/// `IsolatedTurn::append_cli_args`), so no on-disk login from a
/// previous turn or from the user's own `agy` session is ever visible
/// to the child. The real HOME is kept (see `append_isolated_env`)
/// precisely so the OS keyring stays reachable — which makes the
/// keyring the ONLY credential source for a generation turn.
///
/// The consequence: if the keyring is unreachable (a GUI launch
/// without a usable session, a denied keychain prompt, a Linux box with
/// no dbus session), EVERY turn fails the same way — the child prints
/// its interactive-login block and exits non-zero once its own auth
/// wait elapses. Measured 2026-08-07: with a private `--gemini_dir` and
/// piped stdio, that block lands entirely on stderr and stdout comes
/// back empty, which is why the failure used to surface as a bare
/// `CLI exited with status 1`. It no longer does — the child's own
/// words now ride the error — so if this is ever suspected again, read
/// the quoted tail rather than re-deriving it from here.
fn prepare_antigravity_home(
    turn_dir: &Path,
    host_home: Option<&Path>,
    use_canvas_mcp: bool,
) -> io::Result<PathBuf> {
    let server_url = if use_canvas_mcp {
        let host_home = host_home.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Antigravity cannot locate user home",
            )
        })?;
        let source_path = host_home.join(".gemini/config/mcp_config.json");
        let source: serde_json::Value = serde_json::from_slice(&fs::read(source_path)?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let server = source["mcpServers"]["openpencil"]
            .as_object()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Antigravity requires OpenPencil MCP to be enabled in Settings",
                )
            })?;
        if server.get("disabled").and_then(serde_json::Value::as_bool) == Some(true) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Antigravity OpenPencil MCP connection is disabled",
            ));
        }
        let url = server
            .get("serverUrl")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing serverUrl"))?;
        validate_live_mcp_record(host_home, validate_openpencil_url(url)?)?;
        Some(url.to_owned())
    } else {
        None
    };

    let home = turn_dir.join("home");
    let gemini = home.join(".gemini");
    let config_dir = gemini.join("config");
    let settings_dir = gemini.join("antigravity-cli");
    for path in [
        &home,
        &gemini,
        &config_dir,
        &settings_dir,
        &home.join("tmp"),
        &home.join("AppData/Roaming"),
        &home.join("AppData/Local"),
    ] {
        fs::create_dir_all(path)?;
        set_private_dir(path)?;
    }

    let mcp_config = server_url.as_deref().map_or_else(
        || serde_json::json!({"mcpServers": {}}),
        |url| serde_json::json!({"mcpServers": {"openpencil": {"serverUrl": url}}}),
    );
    let allow = if server_url.is_some() {
        serde_json::json!([ANTIGRAVITY_MCP_PERMISSION])
    } else {
        serde_json::json!([])
    };
    let settings = serde_json::json!({
        "toolPermission": "strict",
        "allowNonWorkspaceAccess": false,
        "enableTerminalSandbox": true,
        "permissions": {
            "allow": allow,
            "deny": ANTIGRAVITY_DENY_RULES,
            "ask": []
        }
    });
    write_private_json(&config_dir.join("mcp_config.json"), &mcp_config)?;
    write_private_json(&settings_dir.join("settings.json"), &settings)?;
    Ok(home)
}

fn validate_openpencil_url(input: &str) -> io::Result<u16> {
    let url = reqwest::Url::parse(input).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Antigravity OpenPencil MCP URL is invalid",
        )
    })?;
    let loopback = url
        .host_str()
        .map(|host| {
            host.parse::<IpAddr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(false)
        })
        .unwrap_or(false);
    let safe = url.scheme() == "http"
        && loopback
        && url.port().is_some()
        && url.path() == "/mcp"
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none();
    if safe {
        Ok(url.port().expect("safe URL has an explicit port"))
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Antigravity OpenPencil MCP must use a local http://loopback:port/mcp URL",
        ))
    }
}

/// The settings file is user-editable, so a loopback URL alone is not enough
/// identity proof. Accept only the port record published by this exact GUI
/// process. We intentionally do not copy its shutdown token into the child.
fn validate_live_mcp_record(host_home: &Path, expected_port: u16) -> io::Result<()> {
    let path = host_home
        .join(op_config_store::OPENPENCIL_DIR_NAME)
        .join(op_config_store::well_known::LIVE_MCP_PORT);
    let record: serde_json::Value = serde_json::from_slice(&fs::read(path)?).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid OpenPencil MCP discovery record: {e}"),
        )
    })?;
    let same_process = record.get("writerPid").and_then(serde_json::Value::as_u64)
        == Some(u64::from(std::process::id()));
    let same_port =
        record.get("port").and_then(serde_json::Value::as_u64) == Some(u64::from(expected_port));
    let json_rpc = record.get("transport").and_then(serde_json::Value::as_str) == Some("json-rpc");
    if same_process && same_port && json_rpc {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Antigravity OpenPencil MCP endpoint is not owned by this editor process",
        ))
    }
}

fn write_private_json(path: &Path, value: &serde_json::Value) -> io::Result<()> {
    fs::write(path, serde_json::to_vec(value).map_err(io::Error::other)?)?;
    set_private_file(path)
}

fn create_turn_dir(cli: CliName) -> io::Result<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    for _ in 0..16 {
        let seq = TURN_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "openpencil-cli-turn-{}-{}-{stamp}-{seq}",
            std::process::id(),
            cli.default_binary()
        ));
        match fs::create_dir(&dir) {
            Ok(()) => {
                if let Err(error) = set_private_dir(&dir) {
                    let _ = fs::remove_dir(&dir);
                    return Err(error);
                }
                return Ok(dir);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique CLI turn directory",
    ))
}

impl Drop for IsolatedTurn {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Coding-agent CLIs get only the host/config variables required to start,
/// locate their persisted login, and reach the local MCP server or provider.
/// The MCP shutdown token is deliberately excluded: normal MCP tools do not
/// need it, and a child agent must not inherit authority to stop the host.
pub fn child_env(cli: Option<CliName>) -> Option<Vec<(String, String)>> {
    let cli @ (CliName::OpenCode | CliName::Antigravity | CliName::GrokBuild | CliName::Dsh) = cli?
    else {
        return None;
    };
    Some(filtered_env(cli, std::env::vars()))
}

fn filtered_env<I>(cli: CliName, vars: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (String, String)>,
{
    vars.into_iter()
        .filter(|(key, _)| allowed_env(cli, key))
        .collect()
}

fn allowed_env(cli: CliName, key: &str) -> bool {
    const COMMON: &[&str] = &[
        "HOME",
        "PATH",
        "PATHEXT",
        "USER",
        "LOGNAME",
        "SHELL",
        "SYSTEMROOT",
        "WINDIR",
        "COMSPEC",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PROGRAMDATA",
        "TMPDIR",
        "TMP",
        "TEMP",
        "LANG",
        "LC_ALL",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "no_proxy",
        "all_proxy",
        "REQUESTS_CA_BUNDLE",
        "CURL_CA_BUNDLE",
    ];
    if COMMON.contains(&key) || key.starts_with("LC_") {
        return true;
    }
    match cli {
        CliName::Antigravity => matches!(
            key,
            "ANTIGRAVITY_API_KEY"
                | "GEMINI_API_KEY"
                | "GOOGLE_API_KEY"
                | "GOOGLE_CLOUD_PROJECT"
                | "GOOGLE_CLOUD_LOCATION"
                | "GOOGLE_GENAI_USE_VERTEXAI"
                | "GOOGLE_APPLICATION_CREDENTIALS"
                // Linux desktop OAuth/keyring access relies on the session
                // bus and its runtime socket. Grok does not need these.
                | "DBUS_SESSION_BUS_ADDRESS"
                | "XDG_RUNTIME_DIR"
        ),
        CliName::GrokBuild => matches!(key, "XAI_API_KEY" | "GROK_HOME" | "GROK_API_KEY"),
        // OpenCode reads its auth/config from HOME (or the standard XDG
        // overrides) and supports OPENCODE_* path/config overrides. Model
        // discovery does not need unrelated provider-key namespaces, so do
        // not leak every API key owned by the desktop process into the probe.
        CliName::OpenCode => {
            key.starts_with("OPENCODE_")
                || matches!(
                    key,
                    "XDG_CONFIG_HOME"
                        | "XDG_DATA_HOME"
                        | "XDG_CACHE_HOME"
                        | "XDG_STATE_HOME"
                        | "XDG_RUNTIME_DIR"
                )
        }
        // `dsh` is a Node CLI: it needs the merged login-shell PATH
        // (already in COMMON) so its `#!/usr/bin/env node` shebang
        // resolves Node ≥ 22; the DeepSeek credential rides the
        // standard DEEPSEEK_API_KEY name.
        CliName::Dsh => matches!(key, "DEEPSEEK_API_KEY"),
        _ => false,
    }
}

pub fn is_guarded_cli(cli: Option<CliName>) -> bool {
    matches!(
        cli,
        Some(CliName::Antigravity | CliName::GrokBuild | CliName::Dsh)
    )
}

/// Whether the text is talking about *authentication* rather than
/// *authorship*.
///
/// A bare `contains("auth")` also fires on `author` / `authored` /
/// `authoring` / `authorship`, which appear in ordinary CLI chatter and
/// stack traces. `authoriz…` / `authoris…` is the one `author`-prefixed
/// family that really is authentication, so it is let back through;
/// `oauth`, `unauthenticated`, and `unauthorized` all pass on the base
/// rule. Deliberately not tightened further — guessing at wording we
/// have never seen would cost more coverage than it buys, and a
/// misclassification now carries the child's own words with it.
fn mentions_auth(lower: &str) -> bool {
    lower.match_indices("auth").any(|(at, _)| {
        let rest = &lower[at + "auth".len()..];
        !rest.starts_with("or") || rest.starts_with("oriz") || rest.starts_with("oris")
    })
}

pub fn friendly_stderr_error(cli: Option<CliName>, stderr: &str) -> Option<String> {
    let lower = stderr.to_ascii_lowercase();
    if mentions_auth(&lower) || lower.contains("login") || lower.contains("sign in") {
        return Some(match cli {
            Some(CliName::Antigravity) => {
                "Antigravity is not authenticated. Run `agy` once in a terminal.".into()
            }
            Some(CliName::GrokBuild) => {
                "Grok Build is not authenticated. Run `grok login` in a terminal.".into()
            }
            Some(CliName::Dsh) => {
                "DeepSeek Harness is not authenticated. Run `dsh` once in a terminal.".into()
            }
            _ => return None,
        });
    }
    if lower.contains("permission") || lower.contains("approval") || lower.contains("do you trust")
    {
        return Some(format!(
            "{} stopped because unattended mode cannot grant interactive permission.",
            cli.map(CliName::label).unwrap_or("CLI")
        ));
    }
    None
}

/// Classify a failing turn against BOTH of the child's streams.
///
/// Which stream a CLI uses is not a stable contract: Antigravity prints
/// its interactive-login block to stdout under a TTY but to stderr when
/// stdout is a pipe (measured 2026-08-07 — piped stdout came back
/// empty), so a classifier wired to one stream is silently dead half
/// the time. stderr is tried first: a CLI that writes to both puts its
/// answer on stdout and its diagnosis on stderr.
pub fn friendly_cli_error(cli: Option<CliName>, stderr: &str, stdout: &str) -> Option<String> {
    friendly_stderr_error(cli, stderr)
        .or_else(|| friendly_stdout_error(cli, stdout))
        .or_else(|| friendly_stderr_error(cli, stdout))
}

/// Antigravity's interactive-login block, recognised whether it arrives
/// one live line at a time (the streaming call site) or as a whole
/// retained tail (the exit-status call site).
pub fn friendly_stdout_error(cli: Option<CliName>, text: &str) -> Option<String> {
    if cli != Some(CliName::Antigravity) {
        return None;
    }
    text.lines()
        .any(|line| {
            let line = line.trim().to_ascii_lowercase();
            line.starts_with("authentication required")
                || line.starts_with("waiting for authentication")
                || line.starts_with("or, paste the authorization code")
                || line.contains("accounts.google.com/o/oauth2/auth?")
        })
        .then(|| "Antigravity is not authenticated. Run `agy` once in a terminal.".into())
}

#[cfg(unix)]
fn set_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "chat_subprocess_safety_tests.rs"]
mod tests;
