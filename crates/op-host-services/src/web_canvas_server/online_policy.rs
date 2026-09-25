//! Which routes this daemon is allowed to serve, per deployment mode.
//!
//! The daemon grew up as a single-user, loopback-bound process: every route
//! trusted its one client with the operator's filesystem, the operator's
//! device-login session, and the operator's LAN. A public multi-account
//! deployment shares one process between mutually untrusting accounts, so
//! those routes have to be shut off — not by remembering to check at each
//! call site, but from one table a reviewer can read end to end.
//!
//! [`ServeMode`] is that table. `Local` and `Managed` answer exactly as the
//! daemon always has (every predicate is `true`), so this module cannot
//! change the behaviour of an existing deployment; `Online` is the only
//! mode that refuses anything.

/// How this daemon instance is deployed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ServeMode {
    /// `--serve-web <port>`: one local operator, loopback by default.
    #[default]
    Local,
    /// `--serve-web --managed`: spawned by a supervisor (the VS Code
    /// extension) that holds a per-instance token. Still one operator.
    Managed,
    /// `--serve-web --online`: public multi-account deployment. Every
    /// request carries a verified identity and is served against that
    /// account's own tenant.
    Online,
}

impl ServeMode {
    /// Stable name for the wire, read by the browser shell.
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Managed => "managed",
            Self::Online => "online",
        }
    }

    /// Whether the daemon is the SOLE sequencer for the document it serves.
    ///
    /// Online it is: every writer reaches the same in-memory document through
    /// the same version counter, and the SSE stream publishes the result to
    /// everyone. Locally it is not — the browser and the operator's file are
    /// two copies and the daemon arbitrates neither.
    pub const fn is_server_authoritative(self) -> bool {
        self.is_online()
    }

    pub const fn is_online(self) -> bool {
        matches!(self, Self::Online)
    }

    /// `/api/file/save` + `/api/file/open-recent` — they read and write
    /// paths on the daemon host's filesystem, which in a shared process
    /// means every account writing through the service account.
    pub const fn allows_local_file_routes(self) -> bool {
        !self.is_online()
    }

    /// `/api/ai/site-import` — Studio Home's "import this website". The
    /// fetch is SSRF-screened, but it is still the daemon host dialling an
    /// address a caller chose; a shared public deployment keeps that off
    /// until it has per-account egress accounting.
    pub const fn allows_site_import(self) -> bool {
        !self.is_online()
    }

    /// Whether a settings mutation may be written to the process-wide
    /// settings file. There is one such file for the whole process, so a
    /// shared deployment would let any account overwrite every other
    /// account's providers and credentials.
    pub const fn allows_settings_persistence(self) -> bool {
        !self.is_online()
    }

    /// The device-login proxy (`/api/auth/*`). The bridge runtime holds ONE
    /// device session per process, so proxying it in a shared deployment
    /// would sign every visitor in as whoever the service account is.
    pub const fn allows_device_login_proxy(self) -> bool {
        !self.is_online()
    }

    /// `GET /api/mcp/indicators`. The agent-indicator registry is a
    /// process-global table with no tenant dimension, so relaying it would
    /// show one account the shape of another account's design run.
    pub const fn allows_agent_indicator_relay(self) -> bool {
        !self.is_online()
    }

    /// `POST /` as an alias for `POST /mcp`. Harmless locally; on a public
    /// origin it makes the site root a JSON-RPC endpoint, so the online
    /// deployment keeps exactly one spelling.
    pub const fn allows_root_jsonrpc_alias(self) -> bool {
        !self.is_online()
    }

    /// The `openpencil/shutdown` JSON-RPC branch. Online keeps only the
    /// env-token operations channel that `op stop` uses; there is no
    /// generic client-reachable stop.
    pub const fn allows_generic_shutdown(self) -> bool {
        !self.is_online()
    }

    /// Whether `POST /api/mcp/sync-reset` may actually reset the document.
    ///
    /// The wasm shell posts a sync-reset on every mount, which locally means
    /// "the browser just booted, drop the transient document". Online that
    /// same post would wipe the document the returning account left behind,
    /// so the route answers with the already-reset shape and touches
    /// nothing. See [`ServeMode::sync_reset_is_noop`].
    pub const fn allows_document_reset(self) -> bool {
        !self.is_online()
    }

    /// Inverse spelling of [`Self::allows_document_reset`], for the route.
    pub const fn sync_reset_is_noop(self) -> bool {
        self.is_online()
    }

    /// Collaboration actions that resolve a caller-named socket address or
    /// enumerate the host's LAN (`StartLan` / `BeginDiscovery` /
    /// `JoinDiscovered` / `JoinAddress`). On a public origin those are an
    /// SSRF and an internal-network probe.
    pub const fn allows_caller_named_collab_network(self) -> bool {
        !self.is_online()
    }

    /// Whether relay collaboration may be offered at all.
    ///
    /// A relay session needs a device ticket, and the bridge runtime holds
    /// one per process — so an online daemon cannot mint per-account
    /// tickets. Availability is forced to unavailable and the panel stays a
    /// pure projection until M4 brings up in-service (Tier 1) sessions.
    pub const fn allows_relay_collaboration(self) -> bool {
        !self.is_online()
    }

    /// Whether a loaded document may publish its embedded image thumbnails
    /// into the renderer's thumbnail registry.
    ///
    /// `jian_ops_schema::image_thumbs` is a process-global `OnceLock` map
    /// that a document activation **replaces wholesale**, so in a shared
    /// process one account's document push would drop every other account's
    /// thumbnails — and, worse, hand them ids that now resolve to someone
    /// else's bytes. The minimal safe answer for M1 is to never publish:
    /// the online push path discards the pending seed right after parsing,
    /// which makes the activation a strict no-op, and image nodes paint
    /// their placeholder.
    ///
    /// The full fix is to key the registry by tenant, which means changing
    /// the registry's own signature in `vendor/jian`; that is deliberately
    /// out of M1's scope. Track it with the online image work in M4.
    pub const fn allows_image_thumb_registry(self) -> bool {
        !self.is_online()
    }
}

/// Read the public origins this deployment answers for.
///
/// Shares `OPENPENCIL_WEB_ALLOWED_ORIGINS` with the existing sensitive-POST
/// guard so an operator configures the deployment's origin exactly once.
/// Entries that are not a bare `scheme://host[:port]` are dropped rather than
/// failing start-up: a typo must narrow the allowlist, never widen it.
pub fn allowed_origins_from_env() -> Vec<String> {
    std::env::var(super::origin_guard::WEB_ALLOWED_ORIGINS_ENV)
        .unwrap_or_default()
        .split(',')
        .filter_map(|value| super::origin_guard::parse_http_origin(value.trim()))
        .map(|origin| origin.origin().ascii_serialization())
        .collect()
}

/// The `Access-Control-Allow-Origin` an online deployment may emit.
///
/// Never `*`: online requests carry credentials (a session cookie or a bearer
/// token), and a wildcard plus credentials is exactly the combination that
/// lets any page on the internet read another account's document. An origin
/// that is not on the list gets no header at all, which is what makes the
/// browser refuse to hand the response to the calling script.
pub(super) fn online_cors_origin(allow: &[String], origin: Option<&str>) -> Option<String> {
    let origin = origin?;
    let normalized = super::origin_guard::parse_http_origin(origin)?
        .origin()
        .ascii_serialization();
    allow.contains(&normalized).then_some(origin.to_string())
}

/// Whether a cookie-authenticated write may proceed from this `Origin`.
///
/// The session cookie rides along automatically on any cross-site request the
/// browser makes, so on a public origin the cookie alone proves nothing about
/// who initiated the request — this is the CSRF boundary. A bearer token is
/// exempt: it is only ever attached by code that already holds it.
///
/// Fails closed on a missing `Origin` too. Every browser sends one on a
/// non-GET, and a non-browser client that does not should be using a token.
pub(super) fn cookie_write_origin_allowed(allow: &[String], origin: Option<&str>) -> bool {
    online_cors_origin(allow, origin).is_some()
}

/// The agent-indicator payload an online daemon relays instead of the
/// process-global registry.
///
/// Same shape `op_editor_core::agent_indicators::relay_json` emits for an
/// empty registry, so the browser's `parse_relay_json` accepts it and simply
/// paints nothing.
pub(super) const EMPTY_INDICATOR_RELAY: &str = r#"{"epoch":0,"active":false,"cursorAgent":null,"nodes":[],"frames":[],"previews":[],"reveals":[]}"#;

/// Paths the method-based REST scope rule does not decide.
///
/// - `/api/mcp/server` is the deployment health probe: a client has to be able
///   to discover the daemon exists before it can be told its scopes are
///   insufficient.
/// - `/mcp` and its `/` alias are JSON-RPC, where the MCP dispatch runs a
///   strictly better check — it knows which TOOL is being called, so a
///   read-only token can still call read tools. Applying the coarse
///   "POST means write" rule here would refuse those outright.
const SCOPE_EXEMPT_PATHS: &[&str] = &["/api/mcp/server", "/mcp", "/"];

/// Which scope a REST request needs, if any.
///
/// Method-based: `GET`/`HEAD`/`OPTIONS` read, everything else writes. Deriving
/// it from the method rather than a per-route table means a route added later
/// is covered by default, and covered in the strict direction.
pub(super) fn rest_scope_required(
    method: &str,
    path: &str,
) -> Option<super::tool_scopes::RestScope> {
    if SCOPE_EXEMPT_PATHS.contains(&path) {
        return None;
    }
    match method {
        "GET" | "HEAD" | "OPTIONS" => Some(super::tool_scopes::RestScope::Read),
        _ => Some(super::tool_scopes::RestScope::Write),
    }
}

/// A route the online deployment refuses outright.
///
/// Typed rather than a bare status so the refusal reads the same in a REST
/// body, in a test assertion, and in the route table above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineRouteRefusal {
    /// A route that reads or writes the daemon host's filesystem.
    LocalFileAccess,
    /// A collaboration action that opens a connection to something the
    /// caller named, or enumerates the host's network.
    CallerNamedNetwork,
}

impl OnlineRouteRefusal {
    /// Stable machine-readable code for a REST body.
    pub const fn code(self) -> &'static str {
        match self {
            Self::LocalFileAccess => "online-local-file-disabled",
            Self::CallerNamedNetwork => "online-network-action-disabled",
        }
    }

    pub const fn http_status(self) -> &'static str {
        "403 Forbidden"
    }
}

impl std::fmt::Display for OnlineRouteRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalFileAccess => {
                f.write_str("local file access is disabled on this deployment")
            }
            Self::CallerNamedNetwork => f.write_str(
                "collaboration actions that reach a caller-named address are disabled on this \
                 deployment",
            ),
        }
    }
}

impl std::error::Error for OnlineRouteRefusal {}

/// Render a refusal as the daemon's standard coded-error REST reply.
pub(super) fn refusal_reply(refusal: OnlineRouteRefusal) -> super::WebReply {
    super::WebReply {
        status: refusal.http_status(),
        body: serde_json::json!({
            "ok": false,
            "error": refusal.code(),
            "message": refusal.to_string(),
        })
        .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_and_managed_modes_refuse_nothing() {
        for mode in [ServeMode::Local, ServeMode::Managed] {
            assert!(mode.allows_local_file_routes(), "{mode:?}");
            assert!(mode.allows_settings_persistence(), "{mode:?}");
            assert!(mode.allows_device_login_proxy(), "{mode:?}");
            assert!(mode.allows_agent_indicator_relay(), "{mode:?}");
            assert!(mode.allows_root_jsonrpc_alias(), "{mode:?}");
            assert!(mode.allows_generic_shutdown(), "{mode:?}");
            assert!(mode.allows_document_reset(), "{mode:?}");
            assert!(!mode.sync_reset_is_noop(), "{mode:?}");
            assert!(mode.allows_caller_named_collab_network(), "{mode:?}");
            assert!(mode.allows_relay_collaboration(), "{mode:?}");
            assert!(mode.allows_image_thumb_registry(), "{mode:?}");
            assert!(!mode.is_online(), "{mode:?}");
        }
    }

    #[test]
    fn online_mode_refuses_every_shared_process_route() {
        let mode = ServeMode::Online;
        assert!(mode.is_online());
        assert!(!mode.allows_local_file_routes());
        assert!(!mode.allows_settings_persistence());
        assert!(!mode.allows_device_login_proxy());
        assert!(!mode.allows_agent_indicator_relay());
        assert!(!mode.allows_root_jsonrpc_alias());
        assert!(!mode.allows_generic_shutdown());
        assert!(!mode.allows_document_reset());
        assert!(mode.sync_reset_is_noop());
        assert!(!mode.allows_caller_named_collab_network());
        assert!(!mode.allows_relay_collaboration());
        assert!(!mode.allows_image_thumb_registry());
    }

    #[test]
    fn the_default_mode_is_the_unrestricted_local_daemon() {
        assert_eq!(ServeMode::default(), ServeMode::Local);
    }

    #[test]
    fn a_refusal_renders_a_coded_forbidden_body() {
        let reply = refusal_reply(OnlineRouteRefusal::LocalFileAccess);
        assert_eq!(reply.status, "403 Forbidden");
        let body: serde_json::Value = serde_json::from_str(&reply.body).expect("json");
        assert_eq!(body["ok"], false);
        assert_eq!(body["error"], "online-local-file-disabled");
        assert!(body["message"].as_str().is_some_and(|m| !m.is_empty()));
    }
}
