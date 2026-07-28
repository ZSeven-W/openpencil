//! UI-thread request helpers — every `UiRequest` round-trip the connection
//! threads make (snapshot / list-pages / apply / screenshot / replace /
//! editor-meta), plus the shared ack timeout and small JSON helpers. Split
//! out of `mcp_live.rs` to keep the spine under the 800-line cap.

use super::*;

pub(super) fn request_snapshot(
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
) -> Result<EditorState, McpLiveError> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Snapshot { ack: ack_tx })
        .map_err(|_| {
            McpLiveError::UiThread("UI thread is not accepting MCP snapshot requests".to_string())
        })?;
    wake_ui();
    recv_with_timeout(ack_rx.recv_timeout(UI_ACK_TIMEOUT), "snapshot")
}

pub(super) fn request_list_pages(
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
) -> Result<op_mcp::ListPages, McpLiveError> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::ListPages { ack: ack_tx })
        .map_err(|_| {
            McpLiveError::UiThread("UI thread is not accepting MCP page-list requests".to_string())
        })?;
    wake_ui();
    recv_with_timeout(ack_rx.recv_timeout(UI_ACK_TIMEOUT), "page-list")
}

pub(super) fn request_apply(
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
    tool_name: String,
    cmd: EditorCommand,
) -> Result<ApplyAck, McpLiveError> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name,
            cmd,
            ack: ack_tx,
        })
        .map_err(|_| {
            McpLiveError::UiThread("UI thread is not accepting MCP apply requests".to_string())
        })?;
    wake_ui();
    recv_with_timeout(ack_rx.recv_timeout(UI_ACK_TIMEOUT), "apply")
}

/// Ask the UI thread to render a `debug_screenshot` capture. Bounded
/// wait per the request's own budget (TS `min(timeoutMs ?? 15000,
/// 60000)`) — the same recv-timeout discipline the chat canvas tools
/// use, so a stalled UI pump can never pin the connection thread.
#[cfg(feature = "mcp-debug-tools")]
pub(super) fn request_screenshot(
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
    req: op_mcp::ScreenshotRequest,
) -> Result<crate::export::screenshot::ScreenshotPng, McpLiveError> {
    let timeout = Duration::from_millis(req.timeout_ms.max(1));
    let spec = screenshot::capture_spec(&req);
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Screenshot { spec, ack: ack_tx })
        .map_err(|_| {
            McpLiveError::UiThread("UI thread is not accepting MCP screenshot requests".to_string())
        })?;
    wake_ui();
    // Two layers: the outer `?` is the UI round-trip (channel closed / ack
    // timed out), the inner `?` is the raster capture itself — both land in
    // `McpLiveError`, the export one via `From<ExportError>`.
    Ok(recv_with_timeout(
        ack_rx.recv_timeout(timeout),
        "screenshot",
    )??)
}

/// Ask the UI thread to swap in an already-loaded document. `Ok(false)` means
/// collaboration rejected the replacement; `Err` is a channel/timeout fault.
/// Load/validation failures are handled before this call and surface as 400.
pub(super) fn request_replace(
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
    doc: jian_ops_schema::PenDocument,
    editor_meta: op_pen_loader::EditorMeta,
) -> Result<bool, McpLiveError> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::ReplaceDocument {
            doc: Box::new(doc),
            editor_meta,
            ack: ack_tx,
        })
        .map_err(|_| {
            McpLiveError::UiThread(
                "UI thread is not accepting MCP document-sync requests".to_string(),
            )
        })?;
    wake_ui();
    recv_with_timeout(ack_rx.recv_timeout(UI_ACK_TIMEOUT), "document-sync")
}

pub(super) fn request_update_editor_meta(
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
    editor_meta: op_pen_loader::EditorMeta,
) -> Result<(), McpLiveError> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::UpdateEditorMeta {
            editor_meta,
            ack: ack_tx,
        })
        .map_err(|_| {
            McpLiveError::UiThread(
                "UI thread is not accepting MCP metadata-sync requests".to_string(),
            )
        })?;
    wake_ui();
    recv_with_timeout(ack_rx.recv_timeout(UI_ACK_TIMEOUT), "metadata-sync")
}

pub(super) fn recv_with_timeout<T>(
    result: Result<T, RecvTimeoutError>,
    label: &str,
) -> Result<T, McpLiveError> {
    match result {
        Ok(v) => Ok(v),
        Err(RecvTimeoutError::Timeout) => Err(McpLiveError::UiThread(format!(
            "timed out waiting for UI {label} ack"
        ))),
        Err(RecvTimeoutError::Disconnected) => Err(McpLiveError::UiThread(format!(
            "UI {label} ack channel closed"
        ))),
    }
}

/// Build a per-instance identity token. Not security-sensitive — it only
/// needs to be distinct across editor instances so the CLI can tell a
/// live server apart from a stale discovery file or a recycled port. pid
/// + start nanos is unique enough (two instances never start at the same
///   nanosecond under the same pid).
pub(super) fn make_live_token() -> String {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{pid:x}-{nanos:x}")
}

/// Live `ping` reply: OpenPencil identity + `mode:"live"` + the instance
/// token, so `op` can distinguish this live canvas server from the
/// headless file server and confirm it owns the discovery file.
pub(super) fn live_ping_response(id_raw: &str, token: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"server":"{}","mode":"live","token":"{}"}}}}"#,
        crate::mcp_serve::MCP_SERVER_NAME,
        crate::mcp_serve::json_escape(token)
    )
}

pub(super) fn error_json(message: &str) -> String {
    format!(
        r#"{{"error":"{}"}}"#,
        crate::mcp_serve::json_escape(message)
    )
}
