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
/// target the dev smoke page polls.
pub const DEFAULT_DAEMON_BASE: &str = "http://127.0.0.1:3100";

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
