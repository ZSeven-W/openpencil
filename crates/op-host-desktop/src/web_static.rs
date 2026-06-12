//! Static serving for the web-canvas daemon (`--serve-web`) — the Rust
//! analog of the TS web app's production static hosting. Serves:
//!
//! - `GET /` (and `/index.html`) — the embedded host page
//!   (`web_static/index.html`, baked in via `include_str!` so serving works
//!   from any cwd). The page loads the wasm-bindgen JS glue from `/pkg/`
//!   and calls `mount('op')`; the hidden IME textarea is created by
//!   `mount()` itself, so the page only carries the `<canvas>`.
//! - `GET /pkg/<file>` — the wasm-bindgen output files from the resolved
//!   bundle directory, with correct MIME types (`application/wasm`,
//!   `text/javascript`).
//!
//! Bundle directory resolution order (first directory actually containing
//! `op_host_web.js` wins):
//! 1. `$OPENPENCIL_WEB_BUNDLE_DIR` — explicit override.
//! 2. `<exe_dir>/web-bundle` — deploy layout: the bundle shipped next to
//!    the `openpencil-desktop` executable.
//! 3. `<exe_dir>/../../crates/op-host-web/pkg` — dev layout: a cargo build
//!    runs from `target/<profile>/`, two levels under the repo root, where
//!    `tools/check-wasm-bundle.sh` writes the bundle.
//!
//! When no candidate has a bundle, `/` and `/pkg/*` answer a 404 help page
//! explaining how to build one (`tools/check-wasm-bundle.sh`).

use std::path::{Path, PathBuf};

/// Host page served at `/` (embedded so the daemon serves it from any cwd).
const INDEX_HTML: &str = include_str!("web_static/index.html");

/// 404 help page served when the wasm bundle cannot be found.
const MISSING_BUNDLE_HTML: &str = include_str!("web_static/missing_bundle.html");

/// The wasm-bindgen JS entry the host page imports; its presence marks a
/// directory as a usable bundle.
const BUNDLE_ENTRY_JS: &str = "op_host_web.js";

/// A fully-formed static HTTP reply (status + MIME + body bytes).
pub(crate) struct StaticReply {
    pub(crate) status: &'static str,
    pub(crate) content_type: &'static str,
    pub(crate) body: Vec<u8>,
}

/// Candidate bundle directories, in resolution priority order (see the
/// module docs). Pure w.r.t. the filesystem — existence is checked by
/// [`resolve_bundle_dir`].
pub(crate) fn bundle_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = std::env::var_os("OPENPENCIL_WEB_BUNDLE_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        // Deploy layout: bundle shipped next to the executable.
        candidates.push(exe_dir.join("web-bundle"));
        // Dev layout: target/<profile>/ is two levels under the repo root.
        candidates.push(
            exe_dir
                .join("..")
                .join("..")
                .join("crates")
                .join("op-host-web")
                .join("pkg"),
        );
    }
    candidates
}

/// First candidate directory that actually contains the wasm-bindgen JS
/// entry (`op_host_web.js`), or `None` when no bundle is built anywhere.
pub(crate) fn resolve_bundle_dir() -> Option<PathBuf> {
    bundle_dir_candidates()
        .into_iter()
        .find(|dir| dir.join(BUNDLE_ENTRY_JS).is_file())
}

/// MIME type for a bundle file, keyed on its extension. `.wasm` MUST be
/// `application/wasm` for `WebAssembly.instantiateStreaming` to accept it.
fn content_type_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "wasm" => "application/wasm",
        "js" | "mjs" => "text/javascript",
        "html" => "text/html; charset=utf-8",
        "json" | "map" => "application/json",
        "ts" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Handle a static `GET`. Returns `None` for paths this layer does not own
/// (the caller falls through to REST / SSE / JSON-RPC routing). The bundle
/// directory is a parameter (already resolved) so the routing is testable
/// without mutating process-global env.
pub(crate) fn handle_static_request(path: &str, bundle_dir: Option<&Path>) -> Option<StaticReply> {
    if path == "/" || path == "/index.html" {
        return Some(match bundle_dir {
            Some(_) => StaticReply {
                status: "200 OK",
                content_type: "text/html; charset=utf-8",
                body: INDEX_HTML.as_bytes().to_vec(),
            },
            None => missing_bundle_reply(),
        });
    }
    if let Some(file) = path.strip_prefix("/pkg/") {
        // Only flat, plain file names — the wasm-bindgen output is flat, so
        // any separator or dot-prefixed name is a traversal attempt, not a
        // bundle file.
        if file.is_empty()
            || file.contains('/')
            || file.contains('\\')
            || file.contains("..")
            || file.starts_with('.')
        {
            return Some(not_found_reply());
        }
        let Some(dir) = bundle_dir else {
            return Some(missing_bundle_reply());
        };
        return Some(match std::fs::read(dir.join(file)) {
            Ok(body) => StaticReply {
                status: "200 OK",
                content_type: content_type_for(file),
                body,
            },
            Err(_) => not_found_reply(),
        });
    }
    None
}

/// Plain 404 for a file missing from an otherwise-present bundle.
fn not_found_reply() -> StaticReply {
    StaticReply {
        status: "404 Not Found",
        content_type: "text/plain; charset=utf-8",
        body: b"Not found in the web bundle.".to_vec(),
    }
}

/// 404 help page: no bundle anywhere — tell the user how to build one and
/// which directories were searched.
fn missing_bundle_reply() -> StaticReply {
    let searched = bundle_dir_candidates()
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    StaticReply {
        status: "404 Not Found",
        content_type: "text/html; charset=utf-8",
        body: MISSING_BUNDLE_HTML
            .replace("<!--BUNDLE_CANDIDATES-->", &html_escape(&searched))
            .into_bytes(),
    }
}

/// Minimal HTML escaping for path text interpolated into the help page.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Write a static reply with its own Content-Type (binary-safe body) — the
/// JSON-only `write_mcp_http_response` cannot carry `application/wasm`.
pub(crate) fn write_static_response<S: std::io::Write>(
    stream: &mut S,
    reply: &StaticReply,
) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        reply.status,
        reply.content_type,
        reply.body.len()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|e| format!("http write: {e}"))?;
    stream
        .write_all(&reply.body)
        .map_err(|e| format!("http write: {e}"))?;
    stream.flush().map_err(|e| format!("http flush: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh temp bundle dir containing the wasm-bindgen entry + stub files.
    fn stub_bundle(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("op-web-bundle-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create stub bundle dir");
        std::fs::write(dir.join(BUNDLE_ENTRY_JS), "export default function(){}").expect("js stub");
        std::fs::write(dir.join("op_host_web_bg.wasm"), [0u8, 0x61, 0x73, 0x6d]).expect("wasm");
        dir
    }

    #[test]
    fn index_serves_embedded_host_page_when_bundle_present() {
        let dir = stub_bundle("index");
        let reply = handle_static_request("/", Some(&dir)).expect("static route");
        assert_eq!(reply.status, "200 OK");
        assert_eq!(reply.content_type, "text/html; charset=utf-8");
        let body = String::from_utf8(reply.body).expect("utf8");
        // The host page loads the glue and mounts on the canvas.
        assert!(body.contains("/pkg/op_host_web.js"), "{body}");
        assert!(body.contains("mount('op')"), "{body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pkg_wasm_gets_application_wasm_mime() {
        let dir = stub_bundle("wasm");
        let reply =
            handle_static_request("/pkg/op_host_web_bg.wasm", Some(&dir)).expect("static route");
        assert_eq!(reply.status, "200 OK");
        assert_eq!(reply.content_type, "application/wasm");
        assert_eq!(reply.body, vec![0u8, 0x61, 0x73, 0x6d]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pkg_js_gets_text_javascript_mime() {
        let dir = stub_bundle("js");
        let reply = handle_static_request("/pkg/op_host_web.js", Some(&dir)).expect("static route");
        assert_eq!(reply.status, "200 OK");
        assert_eq!(reply.content_type, "text/javascript");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_bundle_gets_helpful_404_page() {
        let reply = handle_static_request("/", None).expect("static route");
        assert_eq!(reply.status, "404 Not Found");
        assert_eq!(reply.content_type, "text/html; charset=utf-8");
        let body = String::from_utf8(reply.body).expect("utf8");
        assert!(body.contains("check-wasm-bundle.sh"), "{body}");
        assert!(body.contains("OPENPENCIL_WEB_BUNDLE_DIR"), "{body}");
    }

    #[test]
    fn pkg_traversal_attempts_are_rejected() {
        let dir = stub_bundle("traversal");
        for path in [
            "/pkg/../Cargo.toml",
            "/pkg/a/b.js",
            "/pkg/..%2fx", // no percent-decoding happens, but still flat-name-only
            "/pkg/.hidden",
            "/pkg/",
        ] {
            let reply = handle_static_request(path, Some(&dir)).expect("static route");
            assert_eq!(reply.status, "404 Not Found", "path {path} must not serve");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_pkg_file_in_present_bundle_is_plain_404() {
        let dir = stub_bundle("missing-file");
        let reply = handle_static_request("/pkg/nope.js", Some(&dir)).expect("static route");
        assert_eq!(reply.status, "404 Not Found");
        assert_eq!(reply.content_type, "text/plain; charset=utf-8");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_static_paths_fall_through() {
        assert!(handle_static_request("/api/mcp/document", None).is_none());
        assert!(handle_static_request("/mcp", None).is_none());
        assert!(handle_static_request("/favicon.ico", None).is_none());
    }

    #[test]
    fn write_static_response_emits_content_type_and_body() {
        let reply = StaticReply {
            status: "200 OK",
            content_type: "application/wasm",
            body: vec![1, 2, 3],
        };
        let mut out: Vec<u8> = Vec::new();
        write_static_response(&mut out, &reply).expect("write");
        let text = String::from_utf8_lossy(&out);
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"), "{text}");
        assert!(text.contains("Content-Type: application/wasm"), "{text}");
        assert!(text.contains("Content-Length: 3"), "{text}");
        assert!(out.ends_with(&[1, 2, 3]), "{text}");
    }
}
