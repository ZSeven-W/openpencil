//! Daemon base-URL resolution shared by every module that talks to the
//! web-canvas daemon (`live_sync`, `codegen_web`, `web_chat`).
//!
//! When the page is served by the daemon itself (`op start --web` serves
//! the host page at `/` plus the bundle under `/pkg/*`), the daemon base
//! IS the page origin — deriving it from `window.location` makes the
//! shell work on any host/port the daemon was bound to (`--host`,
//! non-default port). The dev smoke page (`python3 -m http.server` from
//! the crate root, page under `/smoke/`) is NOT served by the daemon, so
//! it falls back to the default local daemon origin.
//!
//! The decision logic is a pure function (`resolve_daemon_base`) so the
//! origin/fallback rules are unit-testable without a DOM; only the thin
//! `daemon_base()` wrapper reads `window.location`.
#![allow(dead_code)]

/// Default daemon origin — `op start --web`'s default port, and the
/// target the dev smoke page polls. Sourced from the shared MCP-port
/// constants in `op-editor-core` so client and CLI can't drift.
pub const DEFAULT_DAEMON_BASE: &str = op_editor_core::DEFAULT_LOCAL_DAEMON_ORIGIN;

/// Pick the daemon base from the page's `location.origin` + `pathname`.
///
/// - An `http(s)` origin is used as-is (trailing `/` trimmed) — the page
///   was served over HTTP, and when the daemon serves the bundle that
///   origin is the daemon.
/// - A page under `/smoke/` is the dev smoke harness served by a plain
///   static server, not the daemon — fall back to the default.
/// - A missing / opaque origin (`file://` page → `"null"`, no window)
///   falls back to the default.
pub fn resolve_daemon_base(origin: Option<&str>, pathname: Option<&str>) -> String {
    let Some(origin) = origin else {
        return DEFAULT_DAEMON_BASE.to_string();
    };
    let origin = origin.trim().trim_end_matches('/');
    if !(origin.starts_with("http://") || origin.starts_with("https://")) {
        return DEFAULT_DAEMON_BASE.to_string();
    }
    if pathname.is_some_and(|p| p.starts_with("/smoke/")) {
        return DEFAULT_DAEMON_BASE.to_string();
    }
    origin.to_string()
}

/// Read `window.location` and resolve the daemon base. Browser-only;
/// native test builds never call this (they test `resolve_daemon_base`).
pub fn daemon_base() -> String {
    let Some(window) = web_sys::window() else {
        return DEFAULT_DAEMON_BASE.to_string();
    };
    let location = window.location();
    let origin = location.origin().ok();
    let pathname = location.pathname().ok();
    resolve_daemon_base(origin.as_deref(), pathname.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_origin_is_used_as_base() {
        assert_eq!(
            resolve_daemon_base(Some("http://127.0.0.1:3100"), Some("/")),
            "http://127.0.0.1:3100"
        );
        assert_eq!(
            resolve_daemon_base(Some("http://192.168.1.7:4500"), Some("/index.html")),
            "http://192.168.1.7:4500"
        );
        assert_eq!(
            resolve_daemon_base(Some("https://design.example.com"), Some("/")),
            "https://design.example.com"
        );
    }

    #[test]
    fn trailing_slash_is_trimmed() {
        assert_eq!(
            resolve_daemon_base(Some("http://localhost:3100/"), Some("/")),
            "http://localhost:3100"
        );
    }

    #[test]
    fn smoke_page_falls_back_to_default() {
        // The dev smoke harness is served by `python3 -m http.server`
        // (not the daemon) — its origin must not be used as the base.
        assert_eq!(
            resolve_daemon_base(Some("http://localhost:8000"), Some("/smoke/step-1b.html")),
            DEFAULT_DAEMON_BASE
        );
    }

    #[test]
    fn missing_or_opaque_origin_falls_back_to_default() {
        assert_eq!(resolve_daemon_base(None, None), DEFAULT_DAEMON_BASE);
        // `file://` pages report the opaque origin "null".
        assert_eq!(
            resolve_daemon_base(Some("null"), Some("/")),
            DEFAULT_DAEMON_BASE
        );
        assert_eq!(resolve_daemon_base(Some(""), None), DEFAULT_DAEMON_BASE);
        assert_eq!(
            resolve_daemon_base(Some("file:///Users/x/index.html"), None),
            DEFAULT_DAEMON_BASE
        );
    }
}

/// The tenant this tab is viewing, when the page URL names one.
///
/// A visitor opens a shared document with `?tenant=<ownerId>` on the page
/// URL, and every daemon request this shell makes must carry the same
/// parameter — otherwise the poll, the push and the event stream would each
/// address a different document. Read once (the value cannot change without
/// a navigation, which reloads the shell) and appended by
/// [`with_tenant_param`], which every request path funnels through.
///
/// This is a REQUEST, not a credential: the daemon still resolves the
/// caller's own identity and checks it against the owner's access list, so a
/// hand-edited parameter buys nothing.
pub fn tenant_param() -> Option<String> {
    thread_local! {
        static TENANT: std::cell::OnceCell<Option<String>> = const {
            std::cell::OnceCell::new()
        };
    }
    TENANT.with(|cell| {
        cell.get_or_init(|| {
            let search = web_sys::window()?.location().search().ok()?;
            let query = search.strip_prefix('?').unwrap_or(&search);
            op_editor_core::share_routes::tenant_from_query(query).map(str::to_string)
        })
        .clone()
    })
}

/// Append the tab's tenant parameter to `url`, if it has one.
///
/// Idempotent: a URL that already carries the parameter is returned unchanged.
/// That is what lets both layers apply it safely — [`daemon_url`] stamps it at
/// construction, and the `live_sync` XHR helpers stamp it again on the way
/// out, so a caller that builds its own request cannot silently address the
/// wrong tenant.
///
/// Pure so the query-joining rule is testable without a DOM; the browser
/// wrapper is [`with_tenant_param`].
pub fn append_tenant_param(url: &str, tenant: Option<&str>) -> String {
    let Some(tenant) = tenant.filter(|value| !value.is_empty()) else {
        return url.to_string();
    };
    let parameter = op_editor_core::share_routes::TENANT_QUERY;
    if url_carries_parameter(url, parameter) {
        return url.to_string();
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{parameter}={tenant}")
}

/// Whether `url`'s query already names `parameter`.
fn url_carries_parameter(url: &str, parameter: &str) -> bool {
    let Some((_, query)) = url.split_once('?') else {
        return false;
    };
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .any(|(name, _)| name == parameter)
}

/// The single way to build a daemon URL.
///
/// `path` is an absolute daemon path (`/api/mcp/document`). Every request this
/// shell makes must go through here: a shared page addresses another account's
/// tenant with a query parameter, and a URL built by hand silently addresses
/// the caller's own — which is not an error the user would ever see, just the
/// wrong document.
pub fn daemon_url(path: &str) -> String {
    with_tenant_param(&format!("{}{path}", daemon_base()))
}

/// Browser wrapper over [`append_tenant_param`].
///
/// Applied inside the HTTP helpers rather than at each call site, so a route
/// added later cannot forget it and silently address the wrong document.
pub fn with_tenant_param(url: &str) -> String {
    append_tenant_param(url, tenant_param().as_deref())
}

#[cfg(test)]
mod tenant_param_tests {
    use super::append_tenant_param;

    #[test]
    fn a_url_without_a_query_gains_one() {
        assert_eq!(
            append_tenant_param("http://x/api/mcp/document", Some("userA")),
            "http://x/api/mcp/document?tenant=userA"
        );
    }

    #[test]
    fn a_url_that_already_has_a_query_is_extended() {
        assert_eq!(
            append_tenant_param("http://x/api/mcp/document?v=2", Some("userA")),
            "http://x/api/mcp/document?v=2&tenant=userA"
        );
    }

    #[test]
    fn no_tenant_leaves_the_url_exactly_as_it_was() {
        for tenant in [None, Some("")] {
            assert_eq!(
                append_tenant_param("http://x/api/mcp/document", tenant),
                "http://x/api/mcp/document"
            );
        }
    }
}

#[cfg(test)]
mod daemon_url_tests {
    use super::append_tenant_param;

    /// Every daemon path this shell requests, so the regression is asserted
    /// against the real route list rather than a sample.
    const DAEMON_PATHS: &[&str] = &[
        "/api/mcp/document",
        "/api/mcp/version",
        "/api/mcp/events",
        "/api/mcp/selection",
        "/api/mcp/server",
        "/api/mcp/sync-reset",
        "/api/mcp/indicators",
        "/api/ai/stream",
        "/api/ai/standard",
        "/api/ai/models",
        "/api/ai/image/search",
        "/api/ai/image/generate",
        "/api/ai/site-import",
        "/api/collab/state",
        "/api/collab/action",
        "/api/collab/presence",
        "/api/share/grant",
        "/api/share/revoke",
        "/api/share/list",
        "/api/settings/credentials",
        "/api/settings/credential-policy",
        "/api/export/pdf",
        "/api/export/raster",
        "/api/file/save",
        "/api/file/open-recent",
        "/api/file/unbind",
    ];

    #[test]
    fn every_daemon_path_carries_the_tenant_on_a_shared_page() {
        // The bug this pins: a URL built by hand addresses the caller's own
        // tenant, which is not an error anyone sees — just the wrong document.
        for path in DAEMON_PATHS {
            let url = append_tenant_param(&format!("http://host{path}"), Some("userA"));
            assert!(
                url.ends_with("?tenant=userA"),
                "{path} lost the tenant: {url}"
            );
        }
    }

    #[test]
    fn stamping_twice_does_not_duplicate_the_parameter() {
        // Both layers stamp — the builder at construction and the XHR helpers
        // on the way out — so idempotence is what makes belt-and-braces safe.
        let once = append_tenant_param("http://host/api/mcp/document", Some("userA"));
        let twice = append_tenant_param(&once, Some("userA"));
        assert_eq!(once, twice);
        assert_eq!(twice.matches("tenant=").count(), 1, "{twice}");
    }

    #[test]
    fn an_existing_unrelated_query_is_preserved() {
        let url = append_tenant_param("http://host/api/export/raster?scale=2", Some("userA"));
        assert_eq!(url, "http://host/api/export/raster?scale=2&tenant=userA");
    }

    #[test]
    fn a_url_already_naming_another_tenant_is_left_alone() {
        // The explicit value wins: re-stamping would silently redirect a
        // request that was deliberately addressed elsewhere.
        let url = append_tenant_param("http://host/api/mcp/document?tenant=userB", Some("userA"));
        assert_eq!(url, "http://host/api/mcp/document?tenant=userB");
    }

    #[test]
    fn a_tab_with_no_tenant_builds_plain_urls() {
        for path in DAEMON_PATHS {
            let url = append_tenant_param(&format!("http://host{path}"), None);
            assert_eq!(url, format!("http://host{path}"), "{path}");
        }
    }
}
