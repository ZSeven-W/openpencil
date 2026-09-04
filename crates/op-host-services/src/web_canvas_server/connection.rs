//! One-connection handling: route dispatch across static assets, SSE, REST
//! and JSON-RPC (`serve_one`), plus the SSE writer it hands long-lived
//! subscribers to. Split out of `web_canvas_server.rs` to keep the spine
//! under the 800-line cap; the `/api/ai/*` branch lives in the sibling
//! `connection_ai_routes.rs` for the same reason.

use super::*;

/// Everything one request is served against.
///
/// The single-user daemon has exactly one `Mutex<WebCanvasState>` and one
/// [`SseHub`], so this used to be two arguments. The online daemon has one of
/// each per account and resolves them from the connection's verified identity
/// before dispatch, so the pair travels together with the deployment mode that
/// decides which routes exist at all.
///
/// `Local` and `Managed` build exactly the ctx the two arguments used to mean,
/// and every mode predicate answers `true` for them — the non-online dispatch
/// below is unchanged.
pub(super) struct ConnCtx<'a> {
    pub(super) state: &'a Mutex<WebCanvasState>,
    pub(super) hub: &'a SseHub,
    pub(super) mode: ServeMode,
    /// Which MCP tools this connection may see and call. `UNRESTRICTED` for
    /// the local and managed daemons — the whole catalog, full authority,
    /// exactly as before capability profiles existed.
    pub(super) mcp_profile: crate::mcp_serve::tool_profile::McpAccessProfile,
    /// How this connection's caller authenticated, and what it may do.
    ///
    /// `None` for the local and managed daemons, which have no per-request
    /// identity and are unrestricted — the REST scope gate is skipped whole.
    pub(super) rest_identity: Option<super::tenant_auth::ResolvedIdentity>,
    /// Admits document writes until shutdown closes it. `None` for the local
    /// and managed daemons, whose lifecycle has no flush to protect.
    pub(super) write_barrier: Option<&'a super::tenant::WriteBarrier>,
}

/// Handle one connection against the single-user document authority.
///
/// The `Local`-mode entry point in the two-argument shape the connection
/// tests were written against, so those tests keep proving that the
/// parameterisation below did not change local behaviour. Production callers
/// name their mode via [`serve_one_in_mode`].
#[cfg(test)]
pub(super) fn serve_one<S: Read + Write>(
    stream: &mut S,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
) -> Result<bool> {
    serve_one_in_mode(stream, state, hub, ServeMode::Local)
}

/// Handle one connection. Routes: static host page + wasm bundle (`GET /`,
/// `GET /pkg/*` via `crate::web_static`); SSE live-update stream (`GET
/// /api/mcp/events`); REST whole-doc sync / health (`/api/*` via
/// [`handle_web_canvas_request`]); else JSON-RPC `/mcp` tool dispatch. A
/// mutation (REST POST or a mutating tool call) bumps the version and is
/// broadcast to SSE subscribers. The state `Mutex` is held only across the
/// in-memory operation, never across the (long-lived) SSE wait.
///
/// Managed mode is a single-document, single-operator daemon. Its supervisor
/// already owns the process through the stdin lease, so ordinary HTTP requests
/// do not carry a per-request token. Browser requests are still constrained by
/// the explicit supervisor origin allowlist plus the daemon's own exact
/// loopback origin: an unrelated `Origin` is rejected before route dispatch,
/// while native clients without an `Origin` remain usable. The existing
/// sensitive-POST Host/Origin and JSON-content checks below remain in force in
/// every single-user mode.
///
/// Returns `Ok(true)` when the client requested a token-authenticated graceful
/// shutdown (same `openpencil/shutdown` body contract as `--mcp-http`) — the
/// caller then stops the accept loop so `op stop` never signals a pid. In
/// managed mode only this lifecycle request, not ordinary traffic, uses the
/// token emitted in the startup handshake.
pub(super) fn serve_one_in_mode<S: Read + Write>(
    stream: &mut S,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
    mode: ServeMode,
) -> Result<bool> {
    let req = crate::mcp_serve::read_http_request(stream)?;
    dispatch(
        stream,
        &req,
        &ConnCtx {
            state,
            hub,
            mode,
            mcp_profile: crate::mcp_serve::tool_profile::McpAccessProfile::UNRESTRICTED,
            rest_identity: None,
            write_barrier: None,
        },
    )
}

/// Route one already-parsed request against `ctx`.
///
/// Split from the read so the online accept loop can resolve the identity —
/// and therefore the tenant this ctx points at — from the request headers
/// before anything is dispatched.
pub(super) fn dispatch<S: Read + Write>(
    stream: &mut S,
    req: &crate::mcp_serve::HttpRequest,
    ctx: &ConnCtx<'_>,
) -> Result<bool> {
    let state = ctx.state;
    let hub = ctx.hub;
    let (managed_lifecycle_token, allow_origins) = {
        let guard = state.lock().unwrap_or_else(|p| p.into_inner());
        (guard.managed_token.clone(), guard.allow_origins.clone())
    };
    // Online never emits `*`: its requests carry credentials, and a wildcard
    // plus credentials is what lets any page on the internet read another
    // account's document. Managed mode keeps its own allowlist echo, and the
    // local daemon keeps the permissive value it has always used.
    let cors_origin: Option<String> = if ctx.mode.is_online() {
        online_policy::online_cors_origin(&allow_origins, req.origin.as_deref())
    } else if matches!(ctx.mode, ServeMode::Managed) {
        cors_origin_for(&allow_origins, req.origin.as_deref(), req.host.as_deref())
    } else {
        Some("*".to_string())
    };
    let cors_origin = cors_origin.as_deref();
    if matches!(ctx.mode, ServeMode::Managed)
        && !managed_request_origin_allowed(
            &allow_origins,
            req.origin.as_deref(),
            req.host.as_deref(),
        )
    {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "403 Forbidden",
            &crate::mcp_serve::rest_error_body("request origin is not allowed"),
            cors_origin,
        )?;
        return Ok(false);
    }
    if req.method == "OPTIONS" {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "204 No Content",
            "",
            cors_origin,
        )?;
        return Ok(false);
    }
    if is_sensitive_browser_post(req) && !credential_request_origin_allowed(req) {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "403 Forbidden",
            &crate::mcp_serve::rest_error_body("cross-origin sensitive request is forbidden"),
            cors_origin,
        )?;
        return Ok(false);
    }
    // Sensitive JSON routes refuse CORS "simple request" content types
    // (text/plain, form-encoded, or none): a drive-by page can fire those
    // without a preflight, and unmanaged daemons have no token gate.
    if is_sensitive_browser_post(req) && !content_type_is_json(req.content_type.as_deref()) {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "415 Unsupported Media Type",
            &crate::mcp_serve::rest_error_body(
                "this route requires Content-Type: application/json",
            ),
            cors_origin,
        )?;
        return Ok(false);
    }
    // Static serving: the host page (`/`) and the wasm-bindgen bundle
    // (`/pkg/*`). Owns only those paths — everything else falls through.
    if req.method == "GET" {
        let bundle_dir = crate::web_static::resolve_bundle_dir();
        if let Some(reply) =
            crate::web_static::handle_static_request(&req.path, bundle_dir.as_deref())
        {
            return crate::web_static::write_static_response(stream, &reply, cors_origin)
                .map(|()| false);
        }
    }
    // Sign-in popup interstitial — same auth-exempt static surface as the
    // bundle routes above (it renders a spinner and nothing else). It only
    // exists to host the daemon's device-login proxy, so a deployment with no
    // proxy has no interstitial either.
    if req.method == "GET"
        && req.path == op_editor_core::auth_routes::LOADING_PAGE
        && ctx.mode.allows_device_login_proxy()
    {
        let reply = crate::web_static::StaticReply {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body: crate::web_auth::LOADING_PAGE_HTML.as_bytes().to_vec(),
        };
        return crate::web_static::write_static_response(stream, &reply, cors_origin)
            .map(|()| false);
    }
    // There is deliberately no managed-mode request-token gate here. Managed
    // is a single-tenant child process whose authority comes from the local
    // supervisor's stdin lease and the browser Origin boundary above. Online
    // mode is unchanged: its accept loop resolves a verified account identity
    // before constructing `ctx`, and the scope checks immediately below still
    // constrain that identity on every credentialed route.
    // Scopes apply to every credentialed route, not just the REST tier and
    // `/mcp`. They used to be checked inside the `/api/*` branch, which sits
    // BELOW the specially dispatched routes (AI streams, SSE, figma) — so a
    // read-only token could drive all of those. Checked here, ahead of every
    // branch, there is no route left to slip past it.
    if let Some(identity) = ctx.rest_identity.as_ref() {
        if let Some(refusal) = super::tool_scopes::check_rest_scope(
            identity.via,
            identity.scopes,
            &req.method,
            &req.path,
        ) {
            crate::mcp_serve::write_mcp_http_response_with_origin(
                stream,
                refusal.http_status(),
                &serde_json::json!({
                    "ok": false,
                    "error": refusal.code(),
                    "message": refusal.to_string(),
                })
                .to_string(),
                cors_origin,
            )?;
            return Ok(false);
        }
    }
    // Online account projection. The device-login proxy stays 404 (it drives
    // a process-wide device session), but the shell must be able to learn
    // which account it is showing — without this the identity epoch never
    // fires and an account switch leaks the previous account's document.
    if req.method == "GET" && req.path == op_editor_core::auth_routes::STATUS {
        if let Some(identity) = ctx.rest_identity.as_ref() {
            crate::mcp_serve::write_mcp_http_response_with_origin(
                stream,
                "200 OK",
                &identity.auth_status_json(),
                cors_origin,
            )?;
            return Ok(false);
        }
    }
    // Current-account avatar proxy: performs bounded public HTTPS I/O on this
    // connection thread, never while holding the editor-state mutex. Part of
    // the device-login proxy, so it is off wherever that is.
    if req.method == "POST"
        && req.path == op_editor_core::auth_routes::AVATAR
        && ctx.mode.allows_device_login_proxy()
    {
        let reply = crate::web_auth::avatar();
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            reply.status,
            &reply.body,
            cors_origin,
        )?;
        return Ok(false);
    }
    // Collaboration participant avatar proxy: same shape as the account proxy
    // above — bounded public HTTPS I/O on this connection thread, off the
    // editor-state mutex, so a roster URL never reaches the browser. It reads
    // a process-global registry that only a live relay session populates, so
    // it is gated with relay collaboration itself rather than left to answer
    // one account out of another account's roster.
    if req.method == "POST"
        && req.path == op_editor_core::collab_routes::AVATAR
        && ctx.mode.allows_relay_collaboration()
    {
        let reply = crate::collab_avatar_proxy::avatar(&req.body);
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            reply.status,
            &reply.body,
            cors_origin,
        )?;
        return Ok(false);
    }
    // Device-login begin: waits (per-connection thread, off the state
    // lock) for the pairing's verification URI so the popup can navigate
    // straight from this response — handled here rather than in the
    // whole-body REST tier, which runs under the state mutex.
    if req.method == "POST"
        && req.path == op_editor_core::auth_routes::LOGIN_BEGIN
        && ctx.mode.allows_device_login_proxy()
    {
        let reply = crate::web_auth::login_begin_and_wait(state);
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            reply.status,
            &reply.body,
            cors_origin,
        )?;
        return Ok(false);
    }
    // SSE live-update stream: the browser shell subscribes and re-syncs whenever
    // the document version advances. Subscribe BEFORE reading the current
    // version so no broadcast is missed (a duplicate is harmless — versions are
    // monotonic). The state lock is released before the long SSE wait.
    if req.method == "GET" && req.path == "/api/mcp/events" {
        let slot = hub.subscribe();
        let current = state.lock().unwrap_or_else(|p| p.into_inner()).sse_tick();
        return serve_sse(stream, &slot, current, cors_origin).map(|()| false);
    }
    // The AI / image routes, which parse under the lock and then run long
    // network on this connection thread. See `connection_ai_routes.rs`.
    if let Some(done) = connection_ai_routes::serve_ai_route(stream, req, ctx, cors_origin)? {
        return Ok(done);
    }
    // Offline `.fig` -> `.op` convert for the VS Code plugin: it can't parse
    // fig-kiwi itself, so it POSTs the raw bytes here and boots the returned
    // document JSON through its normal open-document push. Conversion is
    // pure (no network, no state) so — unlike the image routes above — it
    // runs on this connection's own thread without ever touching the state
    // lock; only large/slow parsing needs to stay off it.
    if req.method == "POST" && req.path == "/api/figma/convert" {
        let (status, body) = match crate::figma_convert::convert_fig_json(&req.body) {
            Ok(body) => ("200 OK", body),
            Err(error) => (
                "400 Bad Request",
                serde_json::json!({ "ok": false, "error": error.to_string() }).to_string(),
            ),
        };
        crate::mcp_serve::write_mcp_http_response_with_origin(stream, status, &body, cors_origin)?;
        return Ok(false);
    }
    // All `/api/mcp/*` REST paths go to the REST handler — including ones this
    // daemon doesn't implement yet, which it answers with 404 rather than
    // mis-routing them into the JSON-RPC dispatch below.
    if req.path.starts_with("/api/") {
        // Parse the whole-document push BEFORE taking the state lock. A push
        // can carry megabytes of embedded images, and parsing it under the
        // lock stalled every other request to this tenant — including the
        // version probe that tells the browser anything changed. Nothing here
        // reads live state; `baseVersion` is deliberately re-checked inside
        // the lock, where it is not a race.
        let pending_push = (req.method == "POST" && req.path == "/api/mcp/document")
            .then(|| PendingDocumentPush::parse(&req.body, ctx.mode));
        // A document write must hold a pass for its whole locked segment, so
        // shutdown cannot snapshot the document between the pass being taken
        // and the commit landing.
        let write_pass = match admit_mutation(ctx, pending_push.is_some()) {
            MutationAdmission::NotAWrite => None,
            MutationAdmission::Admitted(pass) => pass,
            MutationAdmission::ShuttingDown => {
                crate::mcp_serve::write_mcp_http_response_with_origin(
                    stream,
                    "503 Service Unavailable",
                    SHUTTING_DOWN_REST_BODY,
                    cors_origin,
                )?;
                return Ok(false);
            }
        };
        let reply = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            let before = guard.version;
            let settings_before = crate::settings_io::fingerprint(&guard.editor);
            let credential_settings_before = (req.method == "POST"
                && req.path == "/api/settings/credentials")
                .then(|| guard.editor.editor_ui.agent_settings.clone());
            let reply = match pending_push {
                // The pre-parsed push installs directly; the generic handler
                // never sees this route's body twice.
                Some(parsed) => document_push_reply(parsed, &mut guard),
                None => handle_web_canvas_request(&req.method, &req.path, &req.body, &mut guard),
            };
            let reply = persist_api_settings(
                &req.method,
                &req.path,
                &mut guard,
                settings_before,
                credential_settings_before,
                reply,
                crate::settings_io::save_checked,
            );
            // Broadcast INSIDE the state lock so the version bump and its
            // broadcast are atomic — otherwise two concurrent mutations could
            // broadcast their versions out of order (SSE clients seeing N then
            // N-1). `broadcast` only sends to unbounded channels (non-blocking),
            // so the lock is held briefly. Lock order is always state→hub.
            if guard.version != before {
                hub.broadcast(guard.sse_tick());
            }
            reply
        };
        // Released only after the commit is visible in the shared state.
        drop(write_pass);
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            reply.status,
            &reply.body,
            cors_origin,
        )?;
        return Ok(false);
    }
    // JSON-RPC tool dispatch is served ONLY as a POST to `/` or `/mcp`. An
    // unknown path is 404; a known path with the wrong method (e.g. `GET /mcp`)
    // is 405 — never silently dispatched as a tool call.
    //
    // The public deployment keeps exactly one spelling: `/` is the site root
    // there, and making a site root a JSON-RPC endpoint is a trap, so the
    // alias answers 405 alongside every other wrong-method request to it.
    let is_jsonrpc_path =
        req.path == "/mcp" || (req.path == "/" && ctx.mode.allows_root_jsonrpc_alias());
    if !is_jsonrpc_path && req.path != "/" {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "404 Not Found",
            r#"{"ok":false,"error":"Not found. Use /, /pkg/*, /api/mcp/document, /api/mcp/sync-reset, /api/mcp/server, /api/mcp/events, /api/file/save, /api/export/raster, /api/export/pdf, or /mcp."}"#,
            cors_origin,
        )?;
        return Ok(false);
    }
    if !is_jsonrpc_path || req.method != "POST" {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "405 Method Not Allowed",
            r#"{"ok":false,"error":"Method not allowed. POST a JSON-RPC message to /mcp."}"#,
            cors_origin,
        )?;
        return Ok(false);
    }
    // Token-authenticated graceful shutdown (`op stop`): same body-carried
    // token contract as the `--mcp-http` server. Managed mode accepts the
    // startup-handshake token here for lifecycle compatibility, but never as
    // an ordinary request header; local/online operator shutdown continues to
    // use only OPENPENCIL_MCP_TOKEN. A stale file, recycled pid, or random
    // client therefore cannot shut the daemon down.
    //
    // Online keeps this branch ONLY because the token comes from the
    // operator's process environment and never from an account credential —
    // it is the operations channel, not a client-reachable route. The
    // env-token check below is what makes that true: with no
    // `OPENPENCIL_MCP_TOKEN` set there is no token that satisfies it.
    let operator_token = crate::mcp_serve::headless_token_from_env();
    if ctx.mode.allows_generic_shutdown() || operator_token.is_some() {
        let managed_shutdown = matches!(ctx.mode, ServeMode::Managed)
            .then_some(managed_lifecycle_token.as_deref())
            .flatten()
            .and_then(|token| crate::mcp_serve::shutdown_request_id(&req.body, token));
        let operator_shutdown = operator_token
            .as_deref()
            .and_then(|token| crate::mcp_serve::shutdown_request_id(&req.body, token));
        if let Some(id) = managed_shutdown.or(operator_shutdown) {
            crate::mcp_serve::write_mcp_http_response_with_origin(
                stream,
                "200 OK",
                &crate::mcp_serve::shutdown_ok_response(&id),
                cors_origin,
            )?;
            return Ok(true);
        }
    }
    // `debug_screenshot` for `--serve-web`: the browser shell mirrors this
    // daemon's document, so the daemon can satisfy the live screenshot tool from
    // the same raster export path desktop live MCP uses. Keep this ahead of the
    // generic dispatch, whose headless debug tool can only report no live
    // canvas.
    #[cfg(feature = "mcp-debug-tools")]
    if let Some(response) = {
        let guard = state.lock().unwrap_or_else(|p| p.into_inner());
        crate::mcp_live::screenshot::maybe_serve(
            &req.body,
            op_mcp::debug_tools_enabled(),
            |shot_req| {
                let spec = crate::mcp_live::screenshot::capture_spec(&shot_req);
                crate::export::screenshot::capture(&guard.editor, &spec)
            },
        )
    } {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "200 OK",
            &response,
            cors_origin,
        )?;
        return Ok(false);
    }
    // A `/mcp` call that will mutate the document is admitted through the same
    // barrier the REST push uses. `tool_profile` already knows which tools
    // write, so the decision is made BEFORE dispatch — the alternative is
    // discovering it from an `EditorCommand` the tool has already produced,
    // which is after the point where refusing is still honest.
    let mcp_write = op_mcp::parse_tool_call(&req.body)
        .is_some_and(|call| crate::mcp_serve::tool_profile::tool_writes(&call.tool));
    let mcp_write_pass = match admit_mutation(ctx, mcp_write) {
        MutationAdmission::NotAWrite => None,
        MutationAdmission::Admitted(pass) => pass,
        MutationAdmission::ShuttingDown => {
            // A tools/call error envelope, not a transport failure: an MCP
            // client must be able to read the refusal and keep its session.
            let refusal = op_mcp::parse_tool_call(&req.body).map(|call| {
                op_mcp::tool_response_to_json(&op_mcp::ToolResponse::Err {
                    id: call.id.clone(),
                    code: op_mcp::ToolErrorCode::ToolFailed,
                    message: "shutting-down: this daemon is stopping and cannot accept writes"
                        .to_string(),
                })
            });
            crate::mcp_serve::write_mcp_http_response_with_origin(
                stream,
                "200 OK",
                refusal.as_deref().unwrap_or(SHUTTING_DOWN_REST_BODY),
                cors_origin,
            )?;
            return Ok(false);
        }
    };
    // JSON-RPC `/mcp` dispatch against the in-memory document. A mutating apply
    // bumps the sync version, broadcast to SSE subscribers so the browser shell
    // sees JSON-RPC-driven changes too.
    let response = {
        let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
        let before = guard.version;
        let mut applied_any = false;
        // The gateway. Snapshotted before dispatch because the applier only
        // borrows the editor, and `Copy` + the held state lock make the
        // snapshot exact for the whole message. Desktop refuses MCP document
        // mutations during a live session (`mcp_live.rs`); the daemon has to
        // give the same answer or an MCP client would fork the shared document
        // out from under the peers.
        let policy = guard.collab_policy();
        let mut refused: Option<op_editor_core::CollabGateReason> = None;
        // Mechanical passthrough only — this daemon (`--serve-web`/`op
        // start`) is a SEPARATE request loop from `mcp_live.rs`'s
        // `McpLiveServer` (desktop `--live-mcp`), not the same struct;
        // wiring canvas-generation indicators here (so a `batch_design`
        // call against a headless `op start` daemon also relays the
        // radar-scan to the browser shell) is tracked as follow-up
        // scope, not part of this pass.
        let response = crate::mcp_serve::process_message_with_applier_profiled(
            &mut guard.editor,
            &req.body,
            ctx.mcp_profile,
            |_tool_name, editor, cmd| {
                if let Err(reason) =
                    policy.check_command(cmd, op_editor_core::CollabEditSource::Mcp)
                {
                    refused = Some(reason);
                    return false;
                }
                let ok = editor.apply(cmd.clone());
                if ok {
                    crate::mcp_serve::normalize_mobile_screens_after_apply(editor);
                }
                applied_any |= ok;
                ok
            },
        )?
        .unwrap_or_default();
        if let Some(reason) = refused {
            // Same surfacing as the desktop live-MCP path: the tool acks
            // "not applied" and the panel explains why, so a refusal is never
            // a silent no-op.
            guard.editor.editor_ui.collab.set_notice(
                reason.notice_kind(),
                crate::design_agent_tools::reveal_now_millis(),
            );
            guard.collab.bump_seq();
        }
        if applied_any {
            guard.version += 1;
        }
        // Atomic bump+broadcast under the state lock (see the REST path) so SSE
        // version events stay monotonic across concurrent mutations.
        if guard.version != before || refused.is_some() {
            hub.broadcast(guard.sse_tick());
        }
        response
    };
    // Released only once the commit is visible in the shared state.
    drop(mcp_write_pass);
    let status = if response.is_empty() {
        "202 Accepted"
    } else {
        "200 OK"
    };
    crate::mcp_serve::write_mcp_http_response_with_origin(stream, status, &response, cors_origin)?;
    Ok(false)
}

/// The body a REST write gets once shutdown has closed the barrier.
const SHUTTING_DOWN_REST_BODY: &str = r#"{"ok":false,"error":"shutting-down","message":"this daemon is stopping and cannot accept writes"}"#;

/// Whether a mutation may proceed, and the pass that proves it is in flight.
pub(super) enum MutationAdmission<'a> {
    /// Not a document mutation — nothing to admit.
    NotAWrite,
    /// Admitted. `None` when this deployment runs no barrier (local/managed).
    Admitted(Option<super::tenant::WritePass<'a>>),
    /// The daemon is stopping and will not durably accept this write.
    ShuttingDown,
}

/// The single admission point for every document mutation.
///
/// Both the REST push and the `/mcp` write dispatch go through here, so a
/// mutation route added later cannot quietly skip the barrier — which is
/// exactly how `/mcp` came to be writing during shutdown while REST was
/// refused.
pub(super) fn admit_mutation<'a>(ctx: &ConnCtx<'a>, is_write: bool) -> MutationAdmission<'a> {
    if !is_write {
        return MutationAdmission::NotAWrite;
    }
    let Some(barrier) = ctx.write_barrier else {
        // Local and managed daemons have no flush to protect.
        return MutationAdmission::Admitted(None);
    };
    match barrier.enter() {
        Some(pass) => MutationAdmission::Admitted(Some(pass)),
        None => MutationAdmission::ShuttingDown,
    }
}

/// Stream Server-Sent Events to a subscribed client: write the SSE headers,
/// emit the current tick immediately (initial sync), then forward each
/// bump from `rx` as a `data: {"version":N,"collabSeq":M}` event. A periodic
/// heartbeat comment keeps the connection alive AND detects a disconnected
/// client (the write fails once the socket is gone). Returns when the client
/// disconnects (write error) or the hub is dropped.
pub(super) fn serve_sse<S: Write>(
    stream: &mut S,
    slot: &SseSlot,
    current: SseTick,
    cors_origin: Option<&str>,
) -> Result<()> {
    let cors_line = cors_origin
        .map(|origin| format!("Access-Control-Allow-Origin: {origin}\r\n"))
        .unwrap_or_default();
    let headers = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: keep-alive\r\n\
         {cors_line}\r\n"
    );
    stream
        .write_all(headers.as_bytes())
        .map_err(|e| WebCanvasError::Transport(format!("sse headers: {e}")))?;
    write_sse_event(stream, current)?;
    loop {
        // The slot holds only the newest tick, so coalescing is structural:
        // a burst of mutations behind a slow client collapses to one event
        // rather than queueing one entry per mutation.
        match slot.take_latest(SSE_HEARTBEAT) {
            Some(tick) => write_sse_event(stream, tick)?,
            None => {
                // SSE comment heartbeat — no-op for the client, but a failed
                // write here is how we notice it disconnected, which is also
                // what ends this loop and drops the slot.
                stream
                    .write_all(b": ping\n\n")
                    .map_err(|e| WebCanvasError::Transport(format!("sse heartbeat: {e}")))?;
                stream
                    .flush()
                    .map_err(|e| WebCanvasError::Transport(format!("sse flush: {e}")))?;
            }
        }
    }
}

/// Format + write one SSE `data:` event carrying both sequence numbers.
///
/// `version` stays the first field and keeps its exact spelling, so a client
/// written against the original payload keeps parsing this one.
pub(super) fn write_sse_event<S: Write>(stream: &mut S, tick: SseTick) -> Result<()> {
    let event = format!(
        "data: {{\"version\":{},\"collabSeq\":{}}}\n\n",
        tick.version, tick.collab_seq
    );
    stream
        .write_all(event.as_bytes())
        .map_err(|e| WebCanvasError::Transport(format!("sse write: {e}")))?;
    stream
        .flush()
        .map_err(|e| WebCanvasError::Transport(format!("sse flush: {e}")))
}
