//! Idle/session-end finalization for file-backed MCP sessions.

use std::time::{Duration, Instant};

/// Nap between non-blocking accept attempts while no client is connected.
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(250);

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use op_editor_core::EditorState;
use op_orchestrator::repair_summary::RepairSummary;

use super::*;

const AUTO_FINALIZE_ENV: &str = "OPENPENCIL_MCP_AUTO_FINALIZE";
const AUTO_FINALIZE_IDLE_SECS_ENV: &str = "OPENPENCIL_MCP_AUTO_FINALIZE_IDLE_SECS";
const DEFAULT_IDLE_AFTER: Duration = Duration::from_secs(45);

/// Tracks the last file-backed MCP write and prevents repeated finalization
/// of the same document revision.
pub struct AutoFinalize {
    last_write_at: Option<Instant>,
    finalized_at_revision: Option<u64>,
    idle_after: Duration,
}

impl AutoFinalize {
    #[cfg(test)]
    pub(crate) fn for_test(idle_after: Duration) -> Self {
        Self {
            last_write_at: None,
            finalized_at_revision: None,
            idle_after,
        }
    }

    /// Read the file-mode auto-finalize switch and idle threshold.
    /// `OPENPENCIL_MCP_AUTO_FINALIZE=0` disables the feature. Invalid or
    /// missing threshold values use the 45-second default.
    pub fn from_env() -> Self {
        let disabled = std::env::var(AUTO_FINALIZE_ENV)
            .map(|value| value.trim() == "0")
            .unwrap_or(false);
        let idle_after = if disabled {
            Duration::MAX
        } else {
            std::env::var(AUTO_FINALIZE_IDLE_SECS_ENV)
                .ok()
                .and_then(|value| value.trim().parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_IDLE_AFTER)
        };
        Self {
            last_write_at: None,
            finalized_at_revision: None,
            idle_after,
        }
    }

    pub fn note_write(&mut self, now: Instant) {
        self.last_write_at = Some(now);
    }

    pub fn due(&self, now: Instant, current_revision: u64) -> bool {
        self.enabled()
            && self.last_write_at.is_some_and(|written| {
                now.checked_duration_since(written)
                    .is_some_and(|idle| idle >= self.idle_after)
            })
            && self.finalized_at_revision != Some(current_revision)
    }

    /// Mark a successful explicit `finalize_design` MCP call so the session
    /// close/idle trigger does not repeat it for the same revision.
    /// The document revision the most recent finalize ran against, if any.
    #[cfg(test)]
    pub(crate) fn last_finalized_revision(&self) -> Option<u64> {
        self.finalized_at_revision
    }

    pub fn note_finalize(&mut self, revision: u64) {
        if self.enabled() {
            self.finalized_at_revision = Some(revision);
        }
    }

    pub fn run(&mut self, state: &mut EditorState, reason: &'static str) -> Option<RepairSummary> {
        let revision = state.document_revision();
        if !self.enabled()
            || state.active_children().is_empty()
            || self.finalized_at_revision == Some(revision)
        {
            return None;
        }
        let recorded = match op_orchestrator::record_loop_finalize_counted(state) {
            Ok(recorded) => recorded,
            Err(error) => {
                eprintln!("openpencil-desktop mcp: auto-finalize ({reason}) failed: {error}");
                return None;
            }
        };
        let summary = recorded.summary;
        *state = recorded.state;
        let finalized_revision = state.document_revision();
        self.finalized_at_revision = Some(finalized_revision);
        eprintln!(
            "openpencil-desktop mcp: auto-finalize ({reason}) repairs={} revision={finalized_revision}",
            summary.total_repairs()
        );
        Some(summary)
    }

    fn enabled(&self) -> bool {
        self.idle_after != Duration::MAX
    }
}

pub(super) fn run(path: PathBuf) -> Result<(), McpServeError> {
    let mut state = super::load_editor_state(&path)?;
    let signals = install_shutdown_signals()?;
    let mut auto_finalize = AutoFinalize::from_env();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    run_stdio_session(
        &mut reader,
        &mut writer,
        &mut state,
        &path,
        &mut auto_finalize,
        &signals.flag,
    )
}

pub(super) fn run_stdio_session<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    state: &mut EditorState,
    path: &Path,
    auto_finalize: &mut AutoFinalize,
    shutdown_flag: &AtomicBool,
) -> Result<(), McpServeError> {
    let mut line = String::new();
    loop {
        line.clear();
        if shutdown_flag.load(Ordering::Acquire) {
            run_auto_finalize_and_save(auto_finalize, state, path, "shutdown")?;
            return Ok(());
        }
        let n = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(error)
                if error.kind() == std::io::ErrorKind::Interrupted
                    && shutdown_flag.load(Ordering::Acquire) =>
            {
                run_auto_finalize_and_save(auto_finalize, state, path, "shutdown")?;
                return Ok(());
            }
            Err(e) => return Err(McpServeError::Io(format!("stdin read: {e}"))),
        };
        if n == 0 {
            run_auto_finalize_and_save(auto_finalize, state, path, "stdin-eof")?;
            return Ok(());
        }
        if let Some(resp) =
            super::process_message_with_auto_finalize(state, path, &line, Some(auto_finalize))?
        {
            writeln!(writer, "{resp}")
                .map_err(|e| McpServeError::Io(format!("stdout write: {e}")))?;
            writer
                .flush()
                .map_err(|e| McpServeError::Io(format!("stdout flush: {e}")))?;
        }
    }
}

pub(super) fn run_http(path: PathBuf, port: u16) -> Result<(), McpServeError> {
    const HTTP_IO_TIMEOUT: Duration = Duration::from_secs(30);
    let mut state = super::load_editor_state(&path)?;
    let signals = install_shutdown_signals()?;
    let mut auto_finalize = AutoFinalize::from_env();
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| McpServeError::Config(format!("bind 127.0.0.1:{port}: {e}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| McpServeError::Config(format!("set nonblocking listener: {e}")))?;
    eprintln!("openpencil-desktop --mcp-http: listening on 127.0.0.1:{port}");
    loop {
        if signals.requested() {
            break;
        }
        if auto_finalize.due(Instant::now(), state.document_revision()) {
            run_auto_finalize_and_save(&mut auto_finalize, &mut state, &path, "idle")?;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(HTTP_IO_TIMEOUT));
                let _ = stream.set_write_timeout(Some(HTTP_IO_TIMEOUT));
                match serve_http_connection(
                    &mut stream,
                    &mut state,
                    &path,
                    Some(&mut auto_finalize),
                ) {
                    Ok(true) => {
                        eprintln!("openpencil-desktop --mcp-http: shutdown requested; exiting");
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => eprintln!("openpencil-desktop --mcp-http: {e}"),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                // Short nap, not the idle threshold: a tool call arriving
                // during this sleep waits for it, so it must stay well under
                // the per-call latency the model notices. The idle check above
                // runs every iteration, so finalize still fires within ~250 ms
                // of the threshold.
                std::thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(error) => eprintln!("openpencil-desktop --mcp-http: accept: {error}"),
        }
    }
    run_auto_finalize_and_save(&mut auto_finalize, &mut state, &path, "shutdown")
}

pub(super) fn serve_http_connection<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    state: &mut EditorState,
    path: &Path,
    auto_finalize: Option<&mut AutoFinalize>,
) -> Result<bool, McpServeError> {
    let reply = |stream: &mut S, status: &str, body: &str| {
        super::write_mcp_http_response_with_origin(stream, status, body, Some("*"))
    };
    let req = super::read_http_request(stream)?;
    if req.method == "OPTIONS" {
        return reply(stream, "204 No Content", "").map(|()| false);
    }
    if req.path != "/mcp" && req.path != "/" {
        return reply(stream, "404 Not Found", r#"{"error":"Not found"}"#).map(|()| false);
    }
    if req.method != "POST" {
        return reply(
            stream,
            "400 Bad Request",
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"Invalid or missing session ID"},"id":null}"#,
        )
        .map(|()| false);
    }
    if let Some(id) = super::shutdown_request_id(
        &req.body,
        &super::headless_token_from_env().unwrap_or_default(),
    ) {
        reply(stream, "200 OK", &super::shutdown_ok_response(&id))?;
        return Ok(true);
    }
    match super::process_message_with_auto_finalize(state, path, &req.body, auto_finalize)? {
        Some(response) => reply(stream, "200 OK", &response).map(|()| false),
        None => reply(stream, "202 Accepted", "").map(|()| false),
    }
}

fn run_auto_finalize_and_save(
    auto_finalize: &mut AutoFinalize,
    state: &mut EditorState,
    path: &Path,
    reason: &'static str,
) -> Result<(), McpServeError> {
    if auto_finalize.run(state, reason).is_some() {
        super::save_editor_state(state, path)?;
    }
    Ok(())
}

struct ShutdownSignals {
    flag: Arc<AtomicBool>,
    registrations: Vec<signal_hook::SigId>,
}

impl ShutdownSignals {
    fn requested(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

impl Drop for ShutdownSignals {
    fn drop(&mut self) {
        for registration in self.registrations.drain(..) {
            let _ = signal_hook::low_level::unregister(registration);
        }
    }
}

fn install_shutdown_signals() -> Result<ShutdownSignals, McpServeError> {
    use signal_hook::consts::signal::{SIGINT, SIGTERM};

    let flag = Arc::new(AtomicBool::new(false));
    let term = signal_hook::flag::register(SIGTERM, Arc::clone(&flag))
        .map_err(|e| McpServeError::Config(format!("register SIGTERM handler: {e}")))?;
    let interrupt = match signal_hook::flag::register(SIGINT, Arc::clone(&flag)) {
        Ok(id) => id,
        Err(error) => {
            let _ = signal_hook::low_level::unregister(term);
            return Err(McpServeError::Config(format!(
                "register SIGINT handler: {error}"
            )));
        }
    };
    Ok(ShutdownSignals {
        flag,
        registrations: vec![term, interrupt],
    })
}

#[cfg(test)]
#[path = "auto_finalize_tests.rs"]
mod tests;
