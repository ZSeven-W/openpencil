//! Live-MCP accept loop + per-connection dispatch: the TCP `server_loop`,
//! the JSON-RPC/REST `serve_connection` router, and the lightweight tool
//! fast path that needs no full `EditorState` snapshot. Split out of
//! `mcp_live.rs` to keep the spine under the 800-line cap.

use super::*;

/// RAII decrement for the live-connection counter — see its use in the
/// per-connection thread. `Drop` runs during normal exit AND panic unwind, so a
/// panicking connection can't leak its reserved slot.
pub(super) struct ConnGuard(Arc<AtomicUsize>);

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub(super) fn server_loop(
    listener: TcpListener,
    req_tx: Sender<UiRequest>,
    stop_rx: Receiver<()>,
    admission: Arc<LiveAdmission>,
    quit_flag: Arc<AtomicBool>,
    wake_ui: UiWake,
    client_identity: Arc<Mutex<Option<(String, String)>>>,
) {
    // Serializes only *stateful* requests so a concurrent multi-apply batch
    // can't interleave. Stateless probes (`ping`/`initialize`) bypass it, so
    // `op`'s liveness probe is never blocked behind an in-flight write.
    let stateful_lock = Arc::new(Mutex::new(()));
    // Bounds the number of in-flight connection threads.
    let conn_count = Arc::new(AtomicUsize::new(0));
    loop {
        match stop_rx.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                if let Err(e) = stream.set_nonblocking(false) {
                    eprintln!("openpencil-desktop mcp: accepted stream blocking mode: {e}");
                    continue;
                }
                if conn_count.load(Ordering::Acquire) >= MAX_LIVE_CONNS {
                    // Shed load rather than spawn unbounded threads.
                    let _ = stream.set_write_timeout(Some(ACCEPT_IDLE_SLEEP));
                    let _ = crate::mcp_serve::write_mcp_http_response(
                        &mut stream,
                        "503 Service Unavailable",
                        r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"server busy"},"id":null}"#,
                    );
                    continue;
                }
                // Handle each connection on its own thread so a long write
                // (which waits on the UI thread) doesn't stall concurrent
                // pings. Threads are short-lived and detached.
                conn_count.fetch_add(1, Ordering::AcqRel);
                let req_tx = req_tx.clone();
                let admission = Arc::clone(&admission);
                let lock = Arc::clone(&stateful_lock);
                let quit = Arc::clone(&quit_flag);
                let conns = Arc::clone(&conn_count);
                let wake = Arc::clone(&wake_ui);
                let identity = Arc::clone(&client_identity);
                let spawned = thread::Builder::new()
                    .name("op-mcp-live-conn".into())
                    .stack_size(LIVE_CONN_STACK_SIZE)
                    .spawn(move || {
                        // RAII: decrement the live-connection count even if
                        // `serve_connection` panics. Without this, a panic would
                        // leak the slot, and after MAX_LIVE_CONNS panics the
                        // server would wedge into permanent load-shedding.
                        let _conn_guard = ConnGuard(conns);
                        let mut stream = stream;
                        let _ = stream.set_read_timeout(Some(UI_ACK_TIMEOUT));
                        let _ = stream.set_write_timeout(Some(UI_ACK_TIMEOUT));
                        if let Err(e) = serve_connection(
                            &mut stream,
                            &req_tx,
                            &admission,
                            &lock,
                            &quit,
                            &wake,
                            &identity,
                        ) {
                            eprintln!("openpencil-desktop mcp: {e}");
                            let _ = crate::mcp_serve::write_mcp_http_response(
                                &mut stream,
                                "500 Internal Server Error",
                                &error_json(&e.to_string()),
                            );
                        }
                    });
                if spawned.is_err() {
                    // The closure (and its `conns` clone) was dropped without
                    // running — undo the count we reserved above.
                    conn_count.fetch_sub(1, Ordering::AcqRel);
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

#[allow(clippy::too_many_arguments)]
pub(super) fn serve_connection<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    req_tx: &Sender<UiRequest>,
    admission: &LiveAdmission,
    stateful_lock: &Mutex<()>,
    quit_flag: &AtomicBool,
    wake_ui: &UiWake,
    client_identity: &Mutex<Option<(String, String)>>,
) -> Result<(), McpLiveError> {
    // A refused body FRAMING (over a route's declared cap, or missing the
    // `Content-Length` a route requires) is a client fault detected before a
    // single body byte was read — answer it with its own status instead of
    // letting it bubble out as a 500. See `mcp_serve::read_http_request`.
    let req = match crate::mcp_serve::read_http_request(stream) {
        Ok(req) => req,
        Err(crate::mcp_serve::McpServeError::Framing { status, message }) => {
            return write_http_with_origin(
                stream,
                status,
                &crate::mcp_serve::rest_error_body(&message),
                None,
            );
        }
        Err(e) => return Err(e.into()),
    };
    let token = admission.token();
    // The one origin this request may be answered to (never `*`) — computed
    // once and threaded through every reply below, refusals included.
    let cors_origin = admission::cors_origin_for(&req, admission);
    // Gate 1 (see `admission.rs`): browser screening, ahead of ALL routing —
    // including the preflight and the stateless probes, so a foreign page
    // cannot even fingerprint this endpoint. `Host`/`Origin` are not
    // page-forgeable, which is what closes DNS rebinding.
    if let Err(denial) = admission::check_boundary(&req, admission) {
        return write_http_with_origin(
            stream,
            denial.http_status(),
            &admission::denial_json_rpc(&req.body, denial),
            cors_origin,
        );
    }
    if req.method == "OPTIONS" {
        if design_md_route::is_design_md_path(&req.path) {
            return design_md_route::write_preflight(stream, cors_origin);
        }
        // The preflight is scoped exactly like the request it precedes: a
        // browser only proceeds when the reply names ITS origin, so this is
        // what stops a non-accepted extension from reaching the ingress.
        return write_http_with_origin(stream, "204 No Content", "", cors_origin);
    }
    // TS live-canvas whole-document sync (REST `POST /api/mcp/document`),
    // distinct from the JSON-RPC `/mcp` path below. Lets a TS whole-doc-sync
    // client (`setSyncDocument` → POST `{document}`) drive THIS editor's
    // on-screen canvas, mirroring `apps/web/server/api/mcp/document.post.ts`.
    if crate::mcp_serve::is_document_sync_route(&req.method, &req.path) {
        // Whole-document replacement of the live (possibly SHARED) document.
        // The local desktop and a self-hosted serve-web daemon trust every
        // caller that clears the `Host`/`Origin` boundary above — the online
        // multi-tenant daemon is a separate request loop with its own
        // per-account auth — so no per-instance token is demanded here. The
        // REST route answers `{ok,error}`, not JSON-RPC.
        return serve_document_sync(stream, req_tx, wake_ui, stateful_lock, &req.body);
    }
    // Content-free design-token evidence from the Chrome extension. This
    // route queues an asynchronous LLM request only; it never snapshots or
    // mutates the live document and therefore does not take `stateful_lock`.
    if design_md_route::is_design_md_path(&req.path) {
        return design_md_route::serve_design_md(stream, req_tx, wake_ui, &req, cors_origin);
    }
    // Insert-only browser-extension ingress (`POST /api/import/web-snapshot`).
    // Deliberately before the general `/mcp` router — see `snapshot_ingest`
    // for why this one route accepts an extension origin and what that does
    // (and does not) grant.
    if snapshot_ingest::is_snapshot_ingest_route(&req.method, &req.path) {
        return snapshot_ingest::serve_snapshot_ingest(
            stream,
            req_tx,
            wake_ui,
            stateful_lock,
            &req,
            cors_origin,
        );
    }
    if req.path != "/mcp" && req.path != "/" {
        return write_http(stream, "404 Not Found", r#"{"error":"Not found"}"#);
    }
    if req.method != "POST" {
        return write_http(
            stream,
            "400 Bad Request",
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"Invalid or missing session ID"},"id":null}"#,
        );
    }
    // Token-authed graceful shutdown: ack, then flag the UI thread to exit
    // the event loop. No pid-kill ⇒ no signal-the-wrong-process race.
    if let Some(id) = crate::mcp_serve::shutdown_request_id(&req.body, token) {
        write_json_rpc_response(stream, &crate::mcp_serve::shutdown_ok_response(&id))?;
        quit_flag.store(true, Ordering::Release);
        wake_ui();
        return Ok(());
    }
    // Stateless handshake/liveness methods must NOT block on the UI thread or
    // take the stateful lock, so `op`'s `ping` probe stays fast and never
    // false-negatives a busy editor. The live `ping` reply carries our
    // identity token so the CLI can confirm THIS server published the file.
    match crate::mcp_serve::classify_stateless(&req.body) {
        crate::mcp_serve::Stateless::Respond(resp) => {
            // Capture the client's declared identity off the SAME
            // `initialize` request `classify_stateless` just answered —
            // the ONLY message this wire protocol carries a name in.
            // Always overwrite (not "first wins"): a later `initialize`
            // means a different tool connected, and the badge should
            // say who is ACTUALLY driving now.
            if let Some(name) = crate::mcp_serve::parse_client_info_name(&req.body) {
                if let Ok(mut identity) = client_identity.lock() {
                    *identity = Some((name, MCP_CLIENT_COLOR.to_string()));
                }
            }
            return write_json_rpc_response(stream, &resp);
        }
        crate::mcp_serve::Stateless::Swallow => {
            return write_json_rpc_response(stream, "");
        }
        crate::mcp_serve::Stateless::Ping(id) => {
            return write_json_rpc_response(stream, &live_ping_response(&id, token));
        }
        crate::mcp_serve::Stateless::NeedsState => {}
    }
    // Gate 2 (see `admission.rs`): everything from here down reads or writes
    // the live document — every `tools/call`, not just the write ones. The
    // `Host`/`Origin` boundary above is the only gate the local desktop and a
    // self-hosted serve-web daemon apply; `CollabGatePolicy` (which still runs
    // on the UI thread for each apply) decides what a session permits. The
    // online multi-tenant daemon authenticates per account in its own request
    // loop, not here.
    // Everything below observes or mutates shared state — the live
    // `EditorState` OR a `--file` document on disk (a read-modify-write).
    // Serialize it under one lock so snapshots stay coherent, concurrent live
    // applies cannot interleave, and file-backed writes cannot race. Tolerate a
    // poisoned lock (a panicked sibling connection) rather than cascade.
    let _guard = stateful_lock
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    // File-backed path (`--file` arg): handle the whole read-modify-write
    // while holding the lock.
    if let Some(response) =
        crate::mcp_serve::file_path::process_message_for_file_path_arg(None, &req.body)?
    {
        return write_json_rpc_response(stream, &response);
    }
    // `debug_screenshot` against the LIVE canvas: route through the
    // raster export pipeline on the UI thread instead of the generic
    // dispatch (whose headless tool can only report no-live-canvas).
    // Gate-closed (`OPENPENCIL_DEBUG_TOOLS` unset) falls through to the
    // generic dispatch, which reports UnknownTool exactly like the
    // headless registry that never registered the tool.
    #[cfg(feature = "mcp-debug-tools")]
    if let Some(response) =
        screenshot::maybe_serve(&req.body, op_mcp::debug_tools_enabled(), |shot_req| {
            request_screenshot(req_tx, wake_ui, shot_req)
        })
    {
        return write_json_rpc_response(stream, &response);
    }
    // These tools need either no editor snapshot (`set_active_page`) or only
    // page metadata (`list_pages`). Keep them on the normal MCP
    // parser/registry/serializer path, but do not deep-clone the live document.
    if let Some(response) = process_lightweight_live_tool(&req.body, req_tx, wake_ui)? {
        return write_json_rpc_response(stream, &response);
    }
    let mut state = request_snapshot(req_tx, wake_ui)?;
    let response = crate::mcp_serve::process_message_with_applier(
        &mut state,
        &req.body,
        |tool_name, local_state, cmd| match request_apply(
            req_tx,
            wake_ui,
            tool_name.to_string(),
            cmd.clone(),
        ) {
            Ok(ack) => {
                if ack.applied {
                    let _ = local_state.apply(cmd.clone());
                }
                ack.applied
            }
            Err(e) => {
                eprintln!("openpencil-desktop mcp: apply failed: {e}");
                false
            }
        },
    )?
    .unwrap_or_default();
    write_json_rpc_response(stream, &response)
}

/// Dispatch live tools whose registry snapshot is independent of the full
/// [`EditorState`]. Returning `None` delegates to the general snapshot path.
pub(super) fn process_lightweight_live_tool(
    line: &str,
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
) -> Result<Option<String>, McpLiveError> {
    let Some(call) = op_mcp::parse_tool_call(line) else {
        return Ok(None);
    };
    let is_write = match call.tool.as_str() {
        "set_active_page" => true,
        "list_pages" => false,
        _ => return Ok(None),
    };

    let mut registry = op_mcp::ToolRegistry::default();
    if is_write {
        registry.register(Box::new(op_mcp::set_active_page_snapshot()));
    } else {
        registry.register(Box::new(request_list_pages(req_tx, wake_ui)?));
    }
    Ok(crate::mcp_serve::process_tool_message_with_registry(
        &registry,
        line,
        |tool_name, cmd| {
            if !is_write {
                return false;
            }
            match request_apply(req_tx, wake_ui, tool_name.to_string(), cmd.clone()) {
                Ok(ack) => ack.applied,
                Err(e) => {
                    eprintln!("openpencil-desktop mcp: apply failed: {e}");
                    false
                }
            }
        },
    )?)
}
