//! Fail-closed automation policy for third-party coding-agent CLIs.
//! Antigravity and Grok Build are coding agents, not plain LLM
//! clients. OpenPencil needs their `openpencil` MCP tools, but must not
//! implicitly grant terminal, filesystem-write, web, subagent, or
//! unrelated MCP access just because a user selected a CLI model.

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

/// Grok's native headless tool filter keeps its built-in local access
/// read-only, `dontAsk` silently denies calls without an explicit allow rule,
/// and strict sandboxing contains approved local access. MCP servers from the
/// user's global Grok config can still be visible; this rule only pre-approves
/// the OpenPencil namespace. The in-band automation guard also tells the agent
/// not to call any other configured MCP server.
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

pub fn antigravity_args() -> Vec<String> {
    // Antigravity's verified one-shot interface accepts the prompt through
    // `-p`; unlike Grok Build, it exposes no prompt-file option. The caller
    // therefore has to put the guarded prompt in argv for the lifetime of the
    // child. Keep the remaining containment layers (private cwd, filtered
    // environment, sandbox, and wall-clock timeout) even though argv privacy
    // cannot be provided for this CLI.
    vec!["--sandbox".into(), "--print-timeout".into(), "90s".into()]
}

pub fn grok_args() -> Vec<String> {
    vec![
        "--no-auto-update".into(),
        "--output-format".into(),
        "streaming-json".into(),
        "--permission-mode".into(),
        "dontAsk".into(),
        "--allow".into(),
        GROK_MCP_ALLOW.into(),
        "--sandbox".into(),
        "strict".into(),
        "--tools".into(),
        GROK_READ_TOOLS.into(),
        "--max-turns".into(),
        "24".into(),
        "--no-plan".into(),
        "--no-subagents".into(),
        "--no-ask-user".into(),
        "--no-memory".into(),
        "--disable-web-search".into(),
        "--no-wait-for-background".into(),
    ]
}

const AUTOMATION_GUARD: &str = "OPENPENCIL AUTOMATION SAFETY:\n\
Use the OpenPencil MCP server only (`mcp__openpencil__*` / `openpencil/*`) to inspect or modify the canvas. \
Do not run terminal commands, write local files, browse the web, spawn subagents, or call any other MCP server. \
Never request interactive approval. If the OpenPencil MCP tools are unavailable or denied, report that failure and stop.";
const GROK_COMPAT_SETTINGS: &[u8] = br#"{"permissions":{"defaultMode":"dontAsk"}}"#;

/// Private, empty per-turn working directory. Antigravity gets a private HOME
/// containing only a strict policy and the loopback OpenPencil MCP entry.
/// Grok reads the prompt and compatibility policy from mode-0600 files here.
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
        Self::prepare_with_host_home(cli, prompt, attachments, host_home.as_deref())
    }

    fn prepare_with_host_home(
        cli: Option<CliName>,
        prompt: &str,
        attachments: &[PathBuf],
        host_home: Option<&Path>,
    ) -> io::Result<Option<Self>> {
        let Some(cli @ (CliName::Antigravity | CliName::GrokBuild)) = cli else {
            return Ok(None);
        };
        let dir = create_turn_dir(cli)?;

        let result = (|| {
            let mut guarded_prompt = format!("{AUTOMATION_GUARD}\n\n{prompt}");
            for (index, source) in attachments.iter().enumerate() {
                let file_name = source
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("attachment");
                let destination = dir.join(format!("attachment-{index}-{file_name}"));
                fs::copy(source, &destination)?;
                set_private_file(&destination)?;
                guarded_prompt = guarded_prompt.replace(
                    source.to_string_lossy().as_ref(),
                    destination.to_string_lossy().as_ref(),
                );
            }
            let prompt_file = if cli == CliName::GrokBuild {
                let path = dir.join("prompt.txt");
                fs::write(&path, &guarded_prompt)?;
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
                let host_home = host_home.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "Antigravity cannot locate the authenticated user home",
                    )
                })?;
                Some(prepare_antigravity_home(&dir, host_home)?)
            } else {
                None
            };
            Ok((guarded_prompt, prompt_file, claude_config_dir, home_dir))
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
}

/// Add only per-turn configuration to the already-filtered child environment.
/// Never forward a host Antigravity HOME or `CLAUDE_CONFIG_DIR`: either can
/// contain always-approve policy, unrelated MCP servers, hooks, or plugins.
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
        const HOME_KEYS: &[&str] = &[
            "HOME",
            "PATH",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "TMPDIR",
            "TMP",
            "TEMP",
        ];
        env.retain(|(key, _)| !HOME_KEYS.contains(&key.as_str()));
        let value = |path: &Path| path.to_string_lossy().into_owned();
        env.extend([
            ("HOME".to_string(), value(home)),
            ("PATH".to_string(), safe_path),
            ("USERPROFILE".to_string(), value(home)),
            ("APPDATA".to_string(), value(&home.join("AppData/Roaming"))),
            (
                "LOCALAPPDATA".to_string(),
                value(&home.join("AppData/Local")),
            ),
            ("TMPDIR".to_string(), value(&home.join("tmp"))),
            ("TMP".to_string(), value(&home.join("tmp"))),
            ("TEMP".to_string(), value(&home.join("tmp"))),
        ]);
    }
}

/// Build a minimal Antigravity profile under the private turn workspace.
/// Authentication remains available through the OS keyring, while settings, plugins,
/// skills, hooks, permission grants, histories, and unrelated MCP servers are
/// deliberately outside the child's HOME.
fn prepare_antigravity_home(turn_dir: &Path, host_home: &Path) -> io::Result<PathBuf> {
    let source_path = host_home
        .join(".gemini")
        .join("config")
        .join("mcp_config.json");
    let source: serde_json::Value =
        serde_json::from_slice(&fs::read(&source_path)?).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid Antigravity MCP config: {e}"),
            )
        })?;
    let server = source
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .and_then(|servers| servers.get("openpencil"))
        .and_then(serde_json::Value::as_object)
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
    let server_url = server
        .get("serverUrl")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Antigravity OpenPencil MCP serverUrl is missing",
            )
        })?;
    let port = validate_openpencil_url(server_url)?;
    validate_live_mcp_record(host_home, port)?;

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

    let mcp_config = serde_json::json!({
        "mcpServers": {
            "openpencil": { "serverUrl": server_url }
        }
    });
    let settings = serde_json::json!({
        "toolPermission": "strict",
        "allowNonWorkspaceAccess": false,
        "enableTerminalSandbox": true,
        "permissions": {
            "allow": [ANTIGRAVITY_MCP_PERMISSION],
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
    let path = host_home.join(".openpencil").join(".op-mcp-port");
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
    let cli @ (CliName::Antigravity | CliName::GrokBuild) = cli? else {
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
        _ => false,
    }
}

pub fn is_guarded_cli(cli: Option<CliName>) -> bool {
    matches!(cli, Some(CliName::Antigravity | CliName::GrokBuild))
}

pub fn friendly_stderr_error(cli: Option<CliName>, stderr: &str) -> Option<String> {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("auth") || lower.contains("login") || lower.contains("sign in") {
        return Some(match cli {
            Some(CliName::Antigravity) => {
                "Antigravity is not authenticated. Run `agy` once in a terminal.".into()
            }
            Some(CliName::GrokBuild) => {
                "Grok Build is not authenticated. Run `grok login` in a terminal.".into()
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
mod tests {
    use super::*;

    fn test_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "openpencil-{label}-{}-{}",
            std::process::id(),
            TURN_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn seed_antigravity_mcp(home: &Path, value: serde_json::Value) {
        let path = home.join(".gemini/config/mcp_config.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        let record_path = home.join(".openpencil/.op-mcp-port");
        fs::create_dir_all(record_path.parent().unwrap()).unwrap();
        fs::write(
            record_path,
            serde_json::to_vec(&serde_json::json!({
                "port": 3100,
                "writerPid": std::process::id(),
                "transport": "json-rpc",
                "token": "must-not-copy"
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn antigravity_turn_uses_private_home_and_minimal_policy() {
        let host_home = test_dir("agy-host-home");
        seed_antigravity_mcp(
            &host_home,
            serde_json::json!({
                "mcpServers": {
                    "openpencil": {
                        "serverUrl": "http://127.0.0.1:3100/mcp",
                        "headers": {"Authorization": "must-not-copy"}
                    },
                    "other": {"serverUrl": "https://example.com/mcp"}
                }
            }),
        );
        assert!(validate_live_mcp_record(&host_home, 3101).is_err());
        let turn = IsolatedTurn::prepare_with_host_home(
            Some(CliName::Antigravity),
            "design a screen",
            &[],
            Some(&host_home),
        )
        .unwrap()
        .unwrap();
        let private_home = turn.home_dir().unwrap().to_path_buf();
        assert!(private_home.starts_with(turn.cwd()));

        let mcp: serde_json::Value = serde_json::from_slice(
            &fs::read(private_home.join(".gemini/config/mcp_config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            mcp,
            serde_json::json!({"mcpServers": {"openpencil": {
                "serverUrl": "http://127.0.0.1:3100/mcp"
            }}})
        );
        let settings: serde_json::Value = serde_json::from_slice(
            &fs::read(private_home.join(".gemini/antigravity-cli/settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(settings["toolPermission"], "strict");
        assert_eq!(settings["allowNonWorkspaceAccess"], false);
        assert_eq!(settings["enableTerminalSandbox"], true);
        assert_eq!(
            settings["permissions"]["allow"],
            serde_json::json!([ANTIGRAVITY_MCP_PERMISSION])
        );
        assert_eq!(
            settings["permissions"]["deny"],
            serde_json::json!(ANTIGRAVITY_DENY_RULES)
        );

        let mut env = vec![
            ("HOME".into(), "/host/home".into()),
            ("PATH".into(), "/host/bin".into()),
            ("APPDATA".into(), "/host/appdata".into()),
            ("TMPDIR".into(), "/host/tmp".into()),
            ("GOOGLE_API_KEY".into(), "provider-auth".into()),
        ];
        append_isolated_env(&mut env, Some(&turn));
        let private_home_text = private_home.to_string_lossy().into_owned();
        assert!(env
            .iter()
            .any(|(key, value)| key == "HOME" && value == &private_home_text));
        assert!(!env.iter().any(|(_, value)| value.starts_with("/host/")));
        assert!(env
            .iter()
            .any(|(key, value)| key == "GOOGLE_API_KEY" && value == "provider-auth"));
        let cwd = turn.cwd().to_path_buf();
        drop(turn);
        assert!(!cwd.exists());
        let _ = fs::remove_dir_all(host_home);
    }

    #[test]
    fn antigravity_rejects_non_loopback_or_disabled_mcp() {
        for server in [
            serde_json::json!({"serverUrl": "https://example.com/mcp"}),
            serde_json::json!({"serverUrl": "http://127.0.0.1:3100/mcp", "disabled": true}),
        ] {
            let host_home = test_dir("agy-unsafe-home");
            seed_antigravity_mcp(
                &host_home,
                serde_json::json!({"mcpServers": {"openpencil": server}}),
            );
            let error = IsolatedTurn::prepare_with_host_home(
                Some(CliName::Antigravity),
                "unsafe config must fail",
                &[],
                Some(&host_home),
            )
            .err()
            .expect("unsafe config should fail");
            assert!(matches!(error.kind(), io::ErrorKind::PermissionDenied));
            let _ = fs::remove_dir_all(host_home);
        }
    }

    #[test]
    fn grok_prompt_file_is_private_and_removed_with_workspace() {
        let source_dir = std::env::temp_dir().join(format!(
            "openpencil-cli-source-{}-{}",
            std::process::id(),
            TURN_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("reference.png");
        fs::write(&source, b"image").unwrap();
        let original = format!("inspect [attached image: {}]", source.display());

        let turn = IsolatedTurn::prepare(Some(CliName::GrokBuild), &original, &[source])
            .unwrap()
            .unwrap();
        let cwd = turn.cwd().to_path_buf();
        let prompt_path = turn.prompt_file().unwrap();
        let config_dir = turn.claude_config_dir().unwrap().to_path_buf();
        let settings_path = config_dir.join("settings.json");
        assert!(prompt_path.starts_with(&cwd));
        assert!(config_dir.starts_with(&cwd));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&settings_path).unwrap())
                .unwrap(),
            serde_json::json!({"permissions": {"defaultMode": "dontAsk"}})
        );
        let mut env = vec![(
            "CLAUDE_CONFIG_DIR".to_string(),
            "/host/claude-config".to_string(),
        )];
        append_isolated_env(&mut env, Some(&turn));
        assert_eq!(
            env,
            vec![(
                "CLAUDE_CONFIG_DIR".to_string(),
                config_dir.to_string_lossy().into_owned(),
            )]
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&config_dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(&settings_path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(turn.prompt().contains("OPENPENCIL AUTOMATION SAFETY"));
        assert!(!turn.prompt().contains(&source_dir.to_string_lossy()[..]));
        assert!(turn.prompt().contains("attachment-0-reference.png"));
        drop(turn);
        assert!(!cwd.exists());
        assert!(!config_dir.exists());
        let _ = fs::remove_dir_all(source_dir);
    }

    #[test]
    fn non_agent_cli_does_not_get_an_isolated_turn() {
        assert!(IsolatedTurn::prepare(Some(CliName::Codex), "hi", &[])
            .unwrap()
            .is_none());
    }

    #[test]
    fn stderr_errors_explain_auth_and_permission_failures() {
        assert!(
            friendly_stderr_error(Some(CliName::GrokBuild), "login required")
                .unwrap()
                .contains("grok login")
        );
        assert!(
            friendly_stderr_error(Some(CliName::Antigravity), "permission required")
                .unwrap()
                .contains("interactive permission")
        );
    }

    #[test]
    fn guarded_child_env_excludes_host_token_and_unrelated_secrets() {
        let vars = vec![
            ("HOME".to_string(), "/tmp/home".to_string()),
            ("HTTPS_PROXY".to_string(), "http://proxy".to_string()),
            ("OPENPENCIL_MCP_TOKEN".to_string(), "shutdown".to_string()),
            ("DATABASE_URL".to_string(), "secret".to_string()),
            ("XAI_API_KEY".to_string(), "xai".to_string()),
            ("GOOGLE_API_KEY".to_string(), "google".to_string()),
        ];

        let grok = filtered_env(CliName::GrokBuild, vars.clone());
        assert!(grok.iter().any(|(key, _)| key == "HOME"));
        assert!(grok.iter().any(|(key, _)| key == "HTTPS_PROXY"));
        assert!(grok.iter().any(|(key, _)| key == "XAI_API_KEY"));
        assert!(!grok.iter().any(|(key, _)| key == "GOOGLE_API_KEY"));
        assert!(!grok.iter().any(|(key, _)| key == "OPENPENCIL_MCP_TOKEN"));
        assert!(!grok.iter().any(|(key, _)| key == "DATABASE_URL"));

        let antigravity = filtered_env(CliName::Antigravity, vars);
        assert!(antigravity.iter().any(|(key, _)| key == "GOOGLE_API_KEY"));
        assert!(!antigravity.iter().any(|(key, _)| key == "XAI_API_KEY"));
        assert!(!antigravity
            .iter()
            .any(|(key, _)| key == "OPENPENCIL_MCP_TOKEN"));
    }

    #[test]
    fn antigravity_keeps_linux_keyring_session_but_grok_does_not() {
        for key in ["DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR"] {
            assert!(allowed_env(CliName::Antigravity, key), "{key}");
            assert!(!allowed_env(CliName::GrokBuild, key), "{key}");
        }
    }
}
