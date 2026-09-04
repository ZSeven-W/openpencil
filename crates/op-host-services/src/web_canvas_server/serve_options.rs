//! `--serve-web` invocation parsing (`ServeWebOptions`), the managed-mode
//! handshake line + token, and the startup-document loader. Split out of
//! `web_canvas_server.rs` to keep the spine under the 800-line cap.

use super::*;

/// Fully parsed `--serve-web` invocation, covering both the legacy
/// positional syntax and the new `--managed` flag syntax (see
/// [`parse_serve_web_args`]).
pub struct ServeWebOptions {
    pub port: u16,
    pub path: Option<PathBuf>,
    pub host: String,
    /// `--managed`: the daemon was spawned by a supervising process (e.g.
    /// the VS Code extension) that expects the handshake-JSON + stdin-EOF
    /// lifecycle contract instead of the legacy fire-and-forget daemon.
    pub managed: bool,
    /// `--allow-origin <origin>` (repeatable), managed mode only. Enforced by
    /// `serve_one` via `cors_origin_for` to gate which `Origin` headers are
    /// echoed back in `Access-Control-Allow-Origin` responses.
    pub allow_origins: Vec<String>,
    /// `--online`: public multi-account deployment. Every request carries a
    /// verified identity and is served against that account's own tenant, and
    /// the routes that would share one process's filesystem / settings file /
    /// device session between accounts are refused. See `online_policy.rs`.
    ///
    /// Mutually exclusive with `--managed`: managed mode's whole contract is
    /// one supervising operator holding the process stdin lease.
    pub online: bool,
}

impl ServeWebOptions {
    /// The deployment mode these options describe.
    pub const fn mode(&self) -> ServeMode {
        if self.online {
            ServeMode::Online
        } else if self.managed {
            ServeMode::Managed
        } else {
            ServeMode::Local
        }
    }
}

/// Parse the argv tail after `--serve-web` itself. Pure, so the flag shape is
/// unit-testable without spawning the binary. Supports two syntaxes:
///
/// - Legacy positional (unchanged): `<port> [doc] [--host <addr>]`.
/// - Managed flag form: `--managed --port <n|0> [--file <path>]
///   [--host <addr>] [--allow-origin <origin>]...` — used by supervising
///   processes that want the handshake-JSON / stdin-EOF lifecycle contract
///   (see [`run_web_canvas`]).
///
/// The host defaults to loopback. Local and online modes may explicitly use
/// `--host 0.0.0.0` for LAN/Docker (no TLS — deploy behind a proxy for
/// anything beyond a trusted network). Managed mode is deliberately
/// loopback-only: its ordinary requests are credential-free and the stdin
/// lease proves local supervision, not authority for a remote peer.
pub fn parse_serve_web_args<I: Iterator<Item = String>>(mut args: I) -> Result<ServeWebOptions> {
    let Some(first) = args.next() else {
        return Err(WebCanvasError::Config("missing <port> arg".into()));
    };
    if first.starts_with("--") {
        return parse_serve_web_args_managed(first, args);
    }
    let Ok(port) = first.parse::<u16>() else {
        return Err(WebCanvasError::Config(format!(
            "<port> must be a u16, got {first:?}"
        )));
    };
    let mut path: Option<PathBuf> = None;
    let mut host = "127.0.0.1".to_string();
    let mut online = false;
    while let Some(arg) = args.next() {
        if arg == "--host" {
            host = args.next().ok_or_else(|| {
                WebCanvasError::Config("--host needs a value (e.g. 0.0.0.0)".into())
            })?;
        } else if let Some(value) = arg.strip_prefix("--host=") {
            host = value.to_string();
        } else if arg == "--online" {
            online = true;
        } else if path.is_none() {
            // The document path is optional — without it the daemon starts
            // from the same starter document the web shell paints locally.
            path = Some(PathBuf::from(arg));
        } else {
            return Err(WebCanvasError::Config(format!("unexpected arg {arg:?}")));
        }
    }
    if host.is_empty() {
        return Err(WebCanvasError::Config("--host must not be empty".into()));
    }
    Ok(ServeWebOptions {
        port,
        path,
        host,
        managed: false,
        allow_origins: Vec::new(),
        online,
    })
}

/// Parse the flag-style `--managed --port <n|0> [--file <path>]
/// [--host <addr>] [--allow-origin <origin>]...` form. `first_flag` is the
/// already-consumed first token (always `--managed` in practice, but any
/// leading `--`-prefixed token routes here so an unknown flag reports a
/// useful error instead of misparsing as a port).
pub(super) fn parse_serve_web_args_managed<I: Iterator<Item = String>>(
    first_flag: String,
    mut args: I,
) -> Result<ServeWebOptions> {
    let mut managed = false;
    let mut port: Option<u16> = None;
    let mut path: Option<PathBuf> = None;
    let mut host = "127.0.0.1".to_string();
    let mut allow_origins: Vec<String> = Vec::new();
    let mut online = false;
    let mut next_flag = Some(first_flag);
    while let Some(arg) = next_flag.take().or_else(|| args.next()) {
        match arg.as_str() {
            "--managed" => managed = true,
            "--online" => online = true,
            "--port" => {
                let value = args
                    .next()
                    .ok_or_else(|| WebCanvasError::Config("--port needs a value".into()))?;
                port = Some(value.parse::<u16>().map_err(|_| {
                    WebCanvasError::Config(format!("--port must be a u16, got {value:?}"))
                })?);
            }
            "--file" => {
                path = Some(PathBuf::from(args.next().ok_or_else(|| {
                    WebCanvasError::Config("--file needs a value".into())
                })?));
            }
            "--host" => {
                host = args.next().ok_or_else(|| {
                    WebCanvasError::Config("--host needs a value (e.g. 0.0.0.0)".into())
                })?;
            }
            "--allow-origin" => {
                allow_origins.push(args.next().ok_or_else(|| {
                    WebCanvasError::Config("--allow-origin needs a value".into())
                })?);
            }
            other => return Err(WebCanvasError::Config(format!("unexpected arg {other:?}"))),
        }
    }
    let Some(port) = port else {
        return Err(WebCanvasError::Config("missing --port <n>".into()));
    };
    if host.is_empty() {
        return Err(WebCanvasError::Config("--host must not be empty".into()));
    }
    // Managed mode means one supervising operator holding the process stdin
    // lease over one document; online means many verified accounts over many.
    // A daemon cannot be both, and silently picking one would hand whichever
    // contract lost its callers the wrong isolation.
    if managed && online {
        return Err(WebCanvasError::Config(
            "--managed and --online are mutually exclusive".into(),
        ));
    }
    if managed && !is_loopback_web_host(&host) {
        return Err(WebCanvasError::Config(
            "--managed requires a loopback --host (127.0.0.1, localhost, or ::1)".into(),
        ));
    }
    Ok(ServeWebOptions {
        port,
        path,
        host,
        managed,
        allow_origins,
        online,
    })
}

/// Build the single-line handshake JSON printed to stdout in managed mode
/// once the listener is bound: `{"ok":true,"port":<n>,"token":"<hex32>",
/// "version":"<crate version>"}`. The supervising process reads exactly one
/// line from the child's stdout to learn the actual bound port (relevant when
/// `--port 0` requested an OS-assigned port) and the lifecycle token retained
/// for compatibility and optional graceful shutdown. Ordinary HTTP requests
/// do not carry this token.
pub(crate) fn handshake_json(port: u16, token: &str) -> String {
    format!(
        r#"{{"ok":true,"port":{port},"token":"{token}","version":"{}"}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

/// Generate a per-instance lifecycle token for managed mode. Not a
/// cryptographic PRNG — `RandomState`'s per-process keying plus a nanosecond
/// timestamp and the pid distinguish separate daemon invocations. It remains
/// in the handshake for compatibility and optional graceful shutdown; request
/// authority comes from the local supervisor lease plus the managed browser
/// origin gate, not from a header token.
pub(super) fn random_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut h1 = s.build_hasher();
    h1.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    let mut h2 = s.build_hasher();
    h2.write_u64(std::process::id() as u64);
    format!("{:016x}{:016x}", h1.finish(), h2.finish())
}

pub(super) fn startup_editor_from_base_for_web_canvas(
    base: EditorState,
    path: Option<PathBuf>,
) -> Result<EditorState> {
    match path {
        Some(p) => {
            let mut next = crate::mcp_serve::load_editor_state(&p)?;
            preserve_web_canvas_preferences(&base, &mut next);
            set_file_name_display(&mut next, &p);
            next.editor_ui.touch_recent_file(
                p.to_string_lossy().into_owned(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            );
            Ok(next)
        }
        None => Ok(base),
    }
}

pub(super) fn startup_editor_for_web_canvas_with_loader<Checked>(
    path: Option<PathBuf>,
    _policy: WebCredentialPersistence,
    checked_load: Checked,
) -> Result<EditorState>
where
    // `settings_io::load_checked` reports its own typed `SettingsIoError`
    // now; a settings file the daemon cannot round-trip aborts start-up, so
    // it lands on `Config` exactly as the pre-conversion `String` did.
    Checked:
        FnOnce(&mut EditorState) -> std::result::Result<(), crate::settings_io::SettingsIoError>,
{
    let mut base = EditorState::starter();
    checked_load(&mut base).map_err(|error| WebCanvasError::Config(error.to_string()))?;
    startup_editor_from_base_for_web_canvas(base, path)
}

pub(super) fn startup_editor_for_web_canvas_with_policy(
    path: Option<PathBuf>,
    policy: WebCredentialPersistence,
) -> Result<EditorState> {
    startup_editor_for_web_canvas_with_loader(path, policy, crate::settings_io::load_checked)
}

/// Public entry point: resolve the daemon's start-up document under the
/// environment's credential-persistence policy.
pub fn startup_editor_for_web_canvas(path: Option<PathBuf>) -> Result<EditorState> {
    startup_editor_for_web_canvas_with_policy(path, crate::web_credential_policy::from_env())
}
