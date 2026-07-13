use op_process_io::{spawn_null, wait_for_child_or, wait_until_false, WaitOutcome};
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const PID_FILE_NAME: &str = "openpencil-mcp-server.pid";
const PORT_FILE_NAME: &str = "openpencil-mcp-server.port";
const TOKEN_FILE_NAME: &str = "openpencil-mcp-server.token";
const MINIMAL_DOCUMENT: &str = "{\n  \"version\": \"0.8.0\",\n  \"name\": \"OpenPencil CLI Session\",\n  \"children\": []\n}\n";

/// Rust live-canvas discovery file (`~/.openpencil/.op-mcp-port`), written
/// by the running editor when its `McpLiveServer` binds. Deliberately
/// distinct from the TS app's `~/.openpencil/.port`: the Rust editor
/// serves JSON-RPC at `/mcp`, while the TS app serves REST at
/// `/api/mcp/*` behind an app URL — sharing one file would let either
/// stack mis-route to the other. Discovery here always confirms identity
/// with a JSON-RPC `ping` before trusting the advertised port.
const LIVE_PORT_FILE_NAME: &str = op_config_store::well_known::LIVE_MCP_PORT;

#[derive(Debug, Clone)]
struct RunningMcp {
    pid: u32,
    port: u16,
    /// Per-instance identity token the server echoes in `ping` (empty for
    /// a manager file written before tokens existed).
    token: String,
}

/// `op start` — by default launch the live-rendering editor GUI and wait
/// for it to publish `~/.openpencil/.op-mcp-port`, so subsequent `op <tool>`
/// calls drive the on-screen canvas in real time (TS parity). Pass
/// `headless` to instead run the windowless file-backed MCP server
/// (`--mcp-http`) — useful for CI / display-less agents. Pass `web` to run
/// the browser editor: the `--serve-web` daemon (static wasm bundle + REST
/// sync + `/mcp`) is spawned headless and the URL is opened in the browser
/// (TS `startWeb` parity).
pub(crate) fn run_start(
    port: u16,
    document_path: Option<&str>,
    headless: bool,
    web: bool,
    host: Option<&str>,
) -> Result<String, String> {
    if web {
        return run_start_web(port, document_path, host);
    }
    if headless {
        // Reuse an already-running headless server; otherwise launch one.
        if let Some(existing) = reachable_headless_server() {
            return Ok(start_json(existing.pid, existing.port, None));
        }
        run_start_headless(port, document_path)
    } else {
        // Reuse a live editor only — a headless file server is NOT a live
        // canvas, so `op start` must open the real GUI even if one runs.
        if let Some((live_port, live_pid)) = reachable_live_port_file() {
            return Ok(start_json(live_pid, live_port, None));
        }
        run_start_live(port, document_path)
    }
}

/// A CLI-managed headless server whose advertised port still answers our
/// JSON-RPC `ping` with the token in our manager file — proving the pid we
/// recorded owns the port (filters stale / pid-recycled manager files).
fn reachable_headless_server() -> Option<RunningMcp> {
    let info = running_mcp_from_pid_file()?;
    crate::mcp_http_cli::mcp_ping_headless(info.port, &info.token).then_some(info)
}

/// Whether the live editor will actually open `path` on launch — mirrors
/// the desktop's `initial_file_from_argv` gate: a supported `.op` / `.pen` /
/// `.fig` extension AND an existing file. The editor silently ignores
/// anything else (blank canvas), so `op start --file` must not claim such a
/// path in its `documentPath`.
fn editor_will_open(path: &str) -> bool {
    let p = Path::new(path);
    let supported = p
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("op")
                || ext.eq_ignore_ascii_case("pen")
                || ext.eq_ignore_ascii_case("fig")
        });
    supported && p.is_file()
}

/// Launch the visible editor with `--live-mcp <port>` and wait for it to
/// publish the discovery port file. No `$TMPDIR` manager files: the
/// editor owns `~/.openpencil/.op-mcp-port` and removes it on exit.
fn run_start_live(port: u16, document_path: Option<&str>) -> Result<String, String> {
    let binary = find_desktop_binary()?;
    let mut command = Command::new(&binary);
    command.arg("--live-mcp").arg(port.to_string());
    // An optional document path is opened in the live editor via the
    // file-association argv path (`initial_file_from_argv`).
    if let Some(path) = document_path {
        command.arg(path);
    }
    let mut child = spawn_null(&mut command)
        .map_err(|e| format!("spawn {} --live-mcp: {e}", binary.display()))?;

    // Only report a `documentPath` the editor will ACTUALLY open: its
    // `initial_file_from_argv` gate ignores a missing / unsupported path
    // (the canvas comes up blank), so reporting the raw `--file` arg would
    // claim a document the editor never opened.
    let opened = document_path.filter(|p| editor_will_open(p));
    // Wait up to ~15s for the editor to bind + publish its port.
    match wait_for_child_or(&mut child, 150, Duration::from_millis(100), || {
        reachable_live_port_file()
    })
    .map_err(|e| format!("wait for {} --live-mcp: {e}", binary.display()))?
    {
        WaitOutcome::Ready((live_port, live_pid)) => {
            return Ok(start_json(live_pid, live_port, opened.map(Path::new)));
        }
        WaitOutcome::Exited(status) => {
            return Err(format!(
                "OpenPencil editor exited before serving the live MCP server: {status}"
            ));
        }
        WaitOutcome::TimedOut => {}
    }
    // Timed out without a verified live server. Report failure honestly
    // rather than a fabricated success on the requested port — the editor
    // process is left running and may still come up, but we cannot
    // confirm the canvas is live yet.
    Err(format!(
        "OpenPencil editor did not publish a live MCP server within 15s \
         (expected ~/.openpencil/{LIVE_PORT_FILE_NAME}); it may still be starting"
    ))
}

/// `op start --web` — spawn the `--serve-web` daemon (headless: static wasm
/// bundle at `/` + `/pkg/*`, REST sync, JSON-RPC `/mcp`), track it via the
/// same `$TMPDIR` pid/port/token manager files as the headless server (so
/// `op stop` / discovery work unchanged — the daemon honors the token-authed
/// `openpencil/shutdown`), wait until its HTTP port answers our token, then
/// print the URL and open it in the default browser.
fn run_start_web(
    port: u16,
    document_path: Option<&str>,
    host: Option<&str>,
) -> Result<String, String> {
    // Reuse only an already-running WEB daemon (token-verified ping +
    // `mode:"web-canvas"` health). A plain `--mcp-http` server answers the
    // ping too but serves no editor, so reusing it would hand the user a
    // browser tab full of JSON errors.
    if let Some(existing) = reachable_headless_server() {
        if crate::mcp_http_cli::web_daemon_running(existing.port) {
            let url = format!("http://127.0.0.1:{}", existing.port);
            open_in_browser(&url);
            return Ok(start_web_json(existing.pid, existing.port, None, host));
        }
        return Err(format!(
            "a non-web MCP server (pid {}) already owns port {}; run `op stop` first",
            existing.pid, existing.port
        ));
    }

    let binary = find_desktop_binary()?;
    // An explicit document is created-if-missing (like `--headless`); without
    // one the daemon starts on an empty document the web shell can sync into.
    let document = match document_path {
        Some(path) => {
            let path = PathBuf::from(path);
            ensure_document_file(&path)?;
            Some(path)
        }
        None => None,
    };
    let token = make_token();
    let mut command = Command::new(&binary);
    command.arg("--serve-web").arg(port.to_string());
    if let Some(doc) = &document {
        command.arg(doc);
    }
    if let Some(host) = host {
        command.arg("--host").arg(host);
    }
    command.env("OPENPENCIL_MCP_TOKEN", &token);
    let mut child = spawn_null(&mut command)
        .map_err(|e| format!("spawn {} --serve-web: {e}", binary.display()))?;

    let pid = child.id();
    write_manager_files(pid, port, &token)?;

    // Wait up to ~5s for the daemon's HTTP server to answer. The token-authed
    // JSON-RPC ping doubles as the identity check (never trust a foreign
    // listener on the port), exactly like the headless start path.
    match wait_for_child_or(&mut child, 50, Duration::from_millis(100), || {
        crate::mcp_http_cli::mcp_ping_headless(port, &token).then_some(())
    })
    .map_err(|e| format!("wait for {} --serve-web: {e}", binary.display()))?
    {
        WaitOutcome::Ready(()) => {
            let url = format!("http://127.0.0.1:{port}");
            open_in_browser(&url);
            return Ok(start_web_json(pid, port, document.as_deref(), host));
        }
        WaitOutcome::Exited(status) => {
            remove_manager_files();
            return Err(format!(
                "OpenPencil web daemon exited before accepting connections: {status}"
            ));
        }
        WaitOutcome::TimedOut => {}
    }
    Err(format!(
        "OpenPencil web daemon did not respond on 127.0.0.1:{port} within 5s"
    ))
}

/// Best-effort default-browser launch, per OS: `open` (macOS),
/// `cmd /C start` (Windows), `xdg-open` (Linux/BSD). Failures are silent —
/// the URL is always printed in the `op start --web` JSON, so a missing
/// opener (SSH session, container) degrades to copy/paste.
fn open_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = Command::new("cmd");
        // Raw arg keeps the URL inside double quotes so cmd doesn't
        // split it at `&`; the empty "" is `start`'s window-title slot —
        // without it the quoted URL would be consumed as the title.
        c.raw_arg(windows_start_args(url));
        c.creation_flags(CREATE_NO_WINDOW);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    let _ = spawn_null(&mut command);
}

/// Args for `cmd /C start "" "<url>"` with the URL double-quoted so cmd
/// doesn't split it at `&`. `"` is illegal in URLs — stripped
/// defensively.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn windows_start_args(url: &str) -> String {
    format!("/C start \"\" \"{}\"", url.replace('"', "%22"))
}

/// `op start --web` success JSON: the headless `start_json` shape plus
/// `mode:"web"` (and the non-default bind host when one was requested, so
/// scripts can derive the LAN URL).
fn start_web_json(pid: u32, port: u16, document_path: Option<&Path>, host: Option<&str>) -> String {
    let mut value = json!({
        "ok": true,
        "running": true,
        "mode": "web",
        "pid": pid,
        "port": port,
        "url": format!("http://127.0.0.1:{port}"),
        "mcpUrl": format!("http://127.0.0.1:{port}/mcp"),
    });
    if let Some(host) = host {
        value["host"] = json!(host);
    }
    if let Some(path) = document_path {
        value["documentPath"] = json!(path.display().to_string());
    }
    value.to_string()
}

/// Legacy windowless mode: spawn `--mcp-http` against a `.op` file and
/// track it via the `$TMPDIR` pid/port manager files.
fn run_start_headless(port: u16, document_path: Option<&str>) -> Result<String, String> {
    let document = match document_path {
        Some(path) => PathBuf::from(path),
        None => default_document_path()?,
    };
    ensure_document_file(&document)?;
    preflight_document(&document)?;

    let binary = find_desktop_binary()?;
    // Per-instance token passed to the server (echoed in its `ping`) so a
    // later `op stop` can prove the pid in our manager file owns the port.
    let token = make_token();
    let mut command = Command::new(&binary);
    command
        .arg("--mcp-http")
        .arg(port.to_string())
        .arg(&document)
        .env("OPENPENCIL_MCP_TOKEN", &token);
    let mut child = spawn_null(&mut command)
        .map_err(|e| format!("spawn {} --mcp-http: {e}", binary.display()))?;

    let pid = child.id();
    write_manager_files(pid, port, &token)?;

    // Confirm the MCP server actually answers WITH our token (not just
    // an open port), so we never report success for a child that failed
    // to bind or for an unrelated service on the port.
    match wait_for_child_or(&mut child, 30, Duration::from_millis(100), || {
        crate::mcp_http_cli::mcp_ping_headless(port, &token).then_some(())
    })
    .map_err(|e| format!("wait for {} --mcp-http: {e}", binary.display()))?
    {
        WaitOutcome::Ready(()) => {
            return Ok(start_json(pid, port, Some(&document)));
        }
        WaitOutcome::Exited(status) => {
            remove_manager_files();
            return Err(format!(
                "OpenPencil MCP server exited before accepting connections: {status}"
            ));
        }
        WaitOutcome::TimedOut => {}
    }

    Err(format!(
        "OpenPencil MCP server did not respond on 127.0.0.1:{port} within 3s"
    ))
}

pub(crate) fn run_stop() -> Result<String, String> {
    // `op stop` asks the server to quit ITSELF via a token-authed
    // `openpencil/shutdown`. This NEVER signals a pid, so there is no
    // recycled-pid / wrong-process race anywhere — and the live editor
    // exits cleanly (saving state + removing its own discovery file).
    // Safe by construction: only the server holding this exact token acts,
    // so a stale file, a recycled pid, or an unrelated service on the port
    // is a no-op (we never touch it). A `.token` is always present for a
    // server we started; an empty token can't authenticate, so it is left
    // alone.
    if let Some((live_port, live_pid, live_token)) = read_live_port_file() {
        if !live_token.is_empty() && crate::mcp_http_cli::request_shutdown(live_port, &live_token) {
            wait_until(|| crate::mcp_http_cli::mcp_ping_live(live_port, &live_token));
            remove_live_port_file();
            return Ok(stop_message("OpenPencil editor stopped"));
        }
        // No live server answered our token. Tidy an orphaned file only when
        // the recorded process is definitively gone; a live-but-unresponsive
        // editor is left intact (discovery re-pings it).
        if live_pid != 0 && !is_pid_alive(live_pid) {
            remove_live_port_file();
        }
    }

    if let Some(info) = running_mcp_from_pid_file() {
        if !info.token.is_empty() && crate::mcp_http_cli::request_shutdown(info.port, &info.token) {
            wait_until(|| crate::mcp_http_cli::mcp_ping_headless(info.port, &info.token));
            remove_manager_files();
            return Ok(stop_message("OpenPencil MCP server stopped"));
        }
        // Didn't answer our token (recycled pid / reused port, or transiently
        // busy). Don't signal anything; only clean up files when the recorded
        // process is gone, so a live server is never disturbed.
        if !is_pid_alive(info.pid) {
            remove_manager_files();
        }
        return Ok(stop_message("No running MCP server found"));
    }

    remove_manager_files();
    Ok(stop_message("No running MCP server found"))
}

fn stop_message(message: &str) -> String {
    json!({ "ok": true, "running": false, "message": message }).to_string()
}

fn remove_live_port_file() {
    if let Some(path) = live_port_file_path() {
        let _ = fs::remove_file(path);
    }
}

/// Poll `still_up` every 100ms for up to ~3s, returning once it reports
/// false (the server stopped responding to its token) or the budget
/// elapses — used to confirm a graceful shutdown actually took effect.
fn wait_until<F: Fn() -> bool>(still_up: F) {
    let _ = wait_until_false(30, Duration::from_millis(100), still_up);
}

/// Path to the Rust live discovery file `~/.openpencil/.op-mcp-port`.
fn live_port_file_path() -> Option<PathBuf> {
    op_config_store::ConfigStore::user()
        .ok()
        .and_then(|store| live_port_file_path_in(&store).ok())
}

fn live_port_file_path_in(store: &op_config_store::ConfigStore) -> std::io::Result<PathBuf> {
    store.path(op_config_store::well_known::LIVE_MCP_PORT)
}

/// Read `~/.openpencil/.op-mcp-port` → `(port, pid, token)`. `pid` is 0
/// when absent or out of `u32` range (so callers never act on a truncated
/// pid); `token` is empty when absent.
fn read_live_port_file() -> Option<(u16, u32, String)> {
    let raw = fs::read_to_string(live_port_file_path()?).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let port = u16::try_from(value.get("port")?.as_u64()?).ok()?;
    let pid = value
        .get("pid")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0);
    let token = value
        .get("token")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    Some((port, pid, token))
}

/// Read `(port, pid, timestamp_ms)` from the live discovery file for
/// `op status`. The host writes `timestamp` at startup (see
/// `mcp_port_file::write`), so the CLI can report pid + uptime exactly like
/// the TS `op status` (`uptime = floor((now - timestamp) / 1000)`). The
/// caller matches `port` against the server it's reporting before trusting
/// pid/timestamp. `None` when the file is absent or lacks the fields.
pub(crate) fn live_status_fields() -> Option<(u16, u32, u64)> {
    let raw = fs::read_to_string(live_port_file_path()?).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let port = u16::try_from(value.get("port")?.as_u64()?).ok()?;
    let pid = value
        .get("pid")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())?;
    let timestamp = value.get("timestamp").and_then(serde_json::Value::as_u64)?;
    Some((port, pid, timestamp))
}

/// Like [`read_live_port_file`] but only returns it when the advertised
/// port is genuinely serving the *live* editor that published this exact
/// token — rejecting stale files, unrelated services, and a headless
/// server squatting on a reused port.
fn reachable_live_port_file() -> Option<(u16, u32)> {
    let (port, pid, token) = read_live_port_file()?;
    crate::mcp_http_cli::mcp_ping_live(port, &token).then_some((port, pid))
}

/// Discover the port of a running OpenPencil MCP server for `op <tool>`
/// calls, preferring the live editor over a CLI-managed headless server.
/// Both candidates are confirmed with a JSON-RPC `ping`. Returns `None`
/// when nothing is reachable.
pub(crate) fn discover_running_port() -> Option<u16> {
    if let Some((port, _)) = reachable_live_port_file() {
        return Some(port);
    }
    reachable_headless_server().map(|info| info.port)
}

pub(crate) fn ensure_document_file(path: &Path) -> Result<(), String> {
    if path.exists() {
        if path.is_file() {
            return Ok(());
        }
        return Err(format!("{} exists but is not a file", path.display()));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(path, MINIMAL_DOCUMENT).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Verify `path` holds a document the headless MCP server can actually load,
/// using the same loader (`op_pen_loader::load_canonical`) the server runs at
/// startup — so a pass here guarantees the server will load it, and a failure
/// is surfaced as a clear, actionable CLI error.
///
/// Without this, a malformed / wrong-format file makes the `--mcp-http` server
/// exit(1) *before* it binds the socket (it loads the document first), so the
/// CLI only ever sees "exited before accepting connections" and the caller a
/// bare "connection refused" — with no hint that the file is the cause. The
/// check never mutates the file, so a corrupt-but-valuable document is
/// preserved rather than silently replaced.
fn preflight_document(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    // A `.op` document is UTF-8 JSON; a real `.fig` / `.pen` is a binary ZIP
    // archive (invalid UTF-8). Reject the binary case here as a wrong file type
    // rather than letting the server's `read_to_string` fail with an IO-flavored
    // error that reads like the file is unreadable.
    let src = std::str::from_utf8(&bytes).map_err(|_| {
        format!(
            "{} is not a valid OpenPencil document: not UTF-8 text \
             (looks like a binary archive). Only .op (JSON) documents open \
             directly; legacy .pen / .fig files must be imported.",
            path.display()
        )
    })?;
    op_pen_loader::load_canonical(src).map(|_| ()).map_err(|e| {
        format!(
            "{} is not a valid OpenPencil document: {e}\n\
             Only .op (JSON) documents open directly; legacy .pen / .fig files must be imported.",
            path.display()
        )
    })
}

fn start_json(pid: u32, port: u16, document_path: Option<&Path>) -> String {
    let mut value = json!({
        "ok": true,
        "running": true,
        "pid": pid,
        "port": port,
        "url": format!("http://127.0.0.1:{port}"),
        "mcpUrl": format!("http://127.0.0.1:{port}/mcp"),
    });
    if let Some(path) = document_path {
        value["documentPath"] = json!(path.display().to_string());
    }
    value.to_string()
}

fn running_mcp_from_pid_file() -> Option<RunningMcp> {
    let (pid_file, port_file, token_file) = manager_files();
    let pid = fs::read_to_string(&pid_file)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()?;
    if !is_pid_alive(pid) {
        remove_manager_files();
        return None;
    }
    let port = fs::read_to_string(&port_file)
        .ok()
        .and_then(|text| text.trim().parse::<u16>().ok())
        .unwrap_or(3100);
    let token = fs::read_to_string(&token_file)
        .map(|text| text.trim().to_string())
        .unwrap_or_default();
    Some(RunningMcp { pid, port, token })
}

fn write_manager_files(pid: u32, port: u16, token: &str) -> Result<(), String> {
    let (pid_file, port_file, token_file) = manager_files();
    fs::write(&pid_file, pid.to_string())
        .map_err(|e| format!("write {}: {e}", pid_file.display()))?;
    fs::write(&port_file, port.to_string())
        .map_err(|e| format!("write {}: {e}", port_file.display()))?;
    fs::write(&token_file, token).map_err(|e| format!("write {}: {e}", token_file.display()))
}

fn remove_manager_files() {
    let (pid_file, port_file, token_file) = manager_files();
    let _ = fs::remove_file(pid_file);
    let _ = fs::remove_file(port_file);
    let _ = fs::remove_file(token_file);
}

fn manager_files() -> (PathBuf, PathBuf, PathBuf) {
    let dir = env::temp_dir();
    (
        dir.join(PID_FILE_NAME),
        dir.join(PORT_FILE_NAME),
        dir.join(TOKEN_FILE_NAME),
    )
}

/// Per-instance identity token (pid + nanos, hex). Not security-sensitive —
/// it only needs to be distinct per spawned server so a later `op stop`
/// can confirm the recorded pid still owns the port.
fn make_token() -> String {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{pid:x}-{nanos:x}")
}

fn default_document_path() -> Result<PathBuf, String> {
    let store = op_config_store::ConfigStore::user().map_err(|e| e.to_string())?;
    default_document_path_in(&store).map_err(|e| e.to_string())
}

fn default_document_path_in(store: &op_config_store::ConfigStore) -> std::io::Result<PathBuf> {
    store.path(op_config_store::well_known::CLI_SESSION)
}

fn home_dir() -> Result<PathBuf, String> {
    op_config_store::home_dir()
        .map_err(|_| "home directory not available; pass --file <path.op>".to_string())
}

fn find_desktop_binary() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("OPENPENCIL_DESKTOP_BIN").map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "OPENPENCIL_DESKTOP_BIN points to a missing file: {}",
            path.display()
        ));
    }

    for path in desktop_binary_candidates() {
        if path.is_file() {
            return Ok(path);
        }
    }
    Err("OpenPencil desktop binary not found; set OPENPENCIL_DESKTOP_BIN or build openpencil-desktop".into())
}

fn desktop_binary_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    if let Some(dir) = exe_dir {
        candidates.push(dir.join(desktop_binary_name()));
        candidates.push(dir.join("OpenPencil"));
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("target").join("debug").join(desktop_binary_name()));
        candidates.push(
            cwd.join("target")
                .join("release")
                .join(desktop_binary_name()),
        );
    }

    if cfg!(target_os = "macos") {
        if let Ok(home) = home_dir() {
            candidates.push(
                home.join("Applications")
                    .join("OpenPencil.app")
                    .join("Contents")
                    .join("MacOS")
                    .join("OpenPencil"),
            );
        }
        candidates.push(
            PathBuf::from("/Applications")
                .join("OpenPencil.app")
                .join("Contents")
                .join("MacOS")
                .join("OpenPencil"),
        );
    } else if cfg!(target_os = "windows") {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            candidates.push(
                local_app_data
                    .join("Programs")
                    .join("openpencil")
                    .join("OpenPencil.exe"),
            );
        }
        candidates.push(PathBuf::from(r"C:\Program Files\OpenPencil\OpenPencil.exe"));
        candidates.push(PathBuf::from(
            r"C:\Program Files (x86)\OpenPencil\OpenPencil.exe",
        ));
    } else {
        candidates.push(PathBuf::from("/usr/bin/openpencil"));
        candidates.push(PathBuf::from("/usr/local/bin/openpencil"));
        if let Ok(home) = home_dir() {
            candidates.push(home.join(".local").join("bin").join("openpencil"));
            collect_app_images(&home.join("Applications"), &mut candidates);
            collect_app_images(&home.join("Downloads"), &mut candidates);
        }
    }
    candidates
}

fn desktop_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "openpencil-desktop.exe"
    } else {
        "openpencil-desktop"
    }
}

fn collect_app_images(dir: &Path, candidates: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("OpenPencil") && name.ends_with(".AppImage") {
            candidates.push(path);
        }
    }
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_pid_alive(pid: u32) -> bool {
    let filter = format!("PID eq {pid}");
    Command::new("tasklist")
        .arg("/FI")
        .arg(filter)
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| stdout.contains(&pid.to_string()))
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "app_control_cli_tests.rs"]
mod tests;
