//! In-process MCP HTTP server for the live desktop editor.
//!
//! The CLI-only `mcp_serve` path owns a file-backed `EditorState`.
//! This module keeps the GUI as the source of truth: the server thread
//! requests a fresh snapshot from the UI thread for each HTTP request,
//! then sends write commands back for the UI thread to apply.

use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError};
use std::thread;
use std::time::Duration;

use op_editor_core::{EditorCommand, EditorState};

const UI_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_IDLE_SLEEP: Duration = Duration::from_millis(25);

pub(crate) struct McpLiveServer {
    port: u16,
    req_rx: Receiver<UiRequest>,
    stop_tx: Sender<()>,
}

struct ApplyAck {
    applied: bool,
    state: EditorState,
}

enum UiRequest {
    Snapshot {
        ack: SyncSender<EditorState>,
    },
    Apply {
        cmd: EditorCommand,
        ack: SyncSender<ApplyAck>,
    },
}

impl McpLiveServer {
    pub(crate) fn start(port: u16) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .map_err(|e| format!("bind 127.0.0.1:{port}: {e}"))?;
        let bound_port = listener
            .local_addr()
            .map_err(|e| format!("read bound MCP port: {e}"))?
            .port();
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("set nonblocking: {e}"))?;
        let (req_tx, req_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();
        thread::Builder::new()
            .name("op-mcp-live-http".into())
            .spawn(move || server_loop(listener, req_tx, stop_rx))
            .map_err(|e| format!("spawn MCP live server: {e}"))?;
        eprintln!("openpencil-desktop mcp: listening on 127.0.0.1:{bound_port}/mcp");
        Ok(Self {
            port: bound_port,
            req_rx,
            stop_tx,
        })
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn pump(&mut self, state: &mut EditorState) -> bool {
        let mut any_applied = false;
        loop {
            match self.req_rx.try_recv() {
                Ok(UiRequest::Snapshot { ack }) => {
                    let _ = ack.send(state.clone());
                }
                Ok(UiRequest::Apply { cmd, ack }) => {
                    let applied = state.apply(cmd);
                    let _ = ack.send(ApplyAck {
                        applied,
                        state: state.clone(),
                    });
                    any_applied |= applied;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        any_applied
    }

    pub(crate) fn stop(&mut self) {
        let _ = self.stop_tx.send(());
    }
}

impl Drop for McpLiveServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn server_loop(listener: TcpListener, req_tx: Sender<UiRequest>, stop_rx: Receiver<()>) {
    loop {
        match stop_rx.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                let _ = stream.set_read_timeout(Some(UI_ACK_TIMEOUT));
                let _ = stream.set_write_timeout(Some(UI_ACK_TIMEOUT));
                if let Err(e) = serve_connection(&mut stream, &req_tx) {
                    eprintln!("openpencil-desktop mcp: {e}");
                    let _ = crate::mcp_serve::write_mcp_http_response(
                        &mut stream,
                        "500 Internal Server Error",
                        &error_json(&e),
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_IDLE_SLEEP);
            }
            Err(e) => {
                eprintln!("openpencil-desktop mcp: accept: {e}");
                thread::sleep(ACCEPT_IDLE_SLEEP);
            }
        }
    }
}

fn serve_connection<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    req_tx: &Sender<UiRequest>,
) -> Result<(), String> {
    let req = crate::mcp_serve::read_http_request(stream)?;
    if req.method == "OPTIONS" {
        return crate::mcp_serve::write_mcp_http_response(stream, "204 No Content", "");
    }
    if req.path != "/mcp" && req.path != "/" {
        return crate::mcp_serve::write_mcp_http_response(
            stream,
            "404 Not Found",
            r#"{"error":"Not found"}"#,
        );
    }
    if req.method != "POST" {
        return crate::mcp_serve::write_mcp_http_response(
            stream,
            "400 Bad Request",
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"Invalid or missing session ID"},"id":null}"#,
        );
    }
    let mut state = request_snapshot(req_tx)?;
    let response = crate::mcp_serve::process_message_with_applier(
        &mut state,
        &req.body,
        |local_state, cmd| match request_apply(req_tx, cmd.clone()) {
            Ok(ack) => {
                *local_state = ack.state;
                ack.applied
            }
            Err(e) => {
                eprintln!("openpencil-desktop mcp: apply failed: {e}");
                false
            }
        },
    )?
    .unwrap_or_default();
    if response.is_empty() {
        crate::mcp_serve::write_mcp_http_response(stream, "202 Accepted", "")
    } else {
        crate::mcp_serve::write_mcp_http_response(stream, "200 OK", &response)
    }
}

fn request_snapshot(req_tx: &Sender<UiRequest>) -> Result<EditorState, String> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Snapshot { ack: ack_tx })
        .map_err(|_| "UI thread is not accepting MCP snapshot requests".to_string())?;
    recv_with_timeout(ack_rx.recv_timeout(UI_ACK_TIMEOUT), "snapshot")
}

fn request_apply(req_tx: &Sender<UiRequest>, cmd: EditorCommand) -> Result<ApplyAck, String> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply { cmd, ack: ack_tx })
        .map_err(|_| "UI thread is not accepting MCP apply requests".to_string())?;
    recv_with_timeout(ack_rx.recv_timeout(UI_ACK_TIMEOUT), "apply")
}

fn recv_with_timeout<T>(result: Result<T, RecvTimeoutError>, label: &str) -> Result<T, String> {
    match result {
        Ok(v) => Ok(v),
        Err(RecvTimeoutError::Timeout) => Err(format!("timed out waiting for UI {label} ack")),
        Err(RecvTimeoutError::Disconnected) => Err(format!("UI {label} ack channel closed")),
    }
}

fn error_json(message: &str) -> String {
    format!(r#"{{"error":"{}"}}"#, json_escape(message))
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}
