// UNVERIFIED: needs EMSDK wasm32 build + browser; run tools/check-wasm-bundle.sh
//! Web Iconify bridge — the browser consumer of the icon picker's
//! `icon_picker_load_more_request` flag.
//!
//! TS (`icon-picker-dialog.tsx`) fetches `https://api.iconify.design`
//! DIRECTLY from the browser — the Iconify API is public + CORS-open —
//! so the web host goes direct too; no daemon route is needed (unlike
//! chat / model discovery, which proxy through the daemon).
//!
//! Pipeline mirrors the desktop `iconify_host.rs` worker:
//! `/search?query=…&limit=…&start=…` → group result ids by collection
//! → `{prefix}.json?icons=…` per collection → `parse_iconify_body`
//! (shared, platform-free) → [`IconPickerRemoteIcon`]s in search-result
//! order. The apply step mirrors `poll_iconify_job`: stale-query guard,
//! page-0 reset, dedupe-append, `total` / `next_start` bookkeeping.
//! Collection bodies are fetched sequentially (continuation-passing) —
//! result pages are ≤48 ids across a handful of collections, so the
//! extra round-trip latency is negligible and the code stays borrow-safe.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use op_editor_core::{IconPickerRemoteIcon, IconifyLoadMoreRequest};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

type InnerRc = Rc<RefCell<crate::Inner>>;
type DoneFn = Box<dyn FnOnce(Result<String, String>)>;

const ICONIFY_API: &str = "https://api.iconify.design";
/// Same budget as the desktop worker's reqwest client.
const FETCH_TIMEOUT_MS: u32 = 15_000;

/// One in-flight result page being assembled across the search +
/// per-collection fetches.
struct PendingPage {
    request: IconifyLoadMoreRequest,
    /// `"collection:name"` ids in search-result order.
    order: Vec<String>,
    /// Collections still to fetch, with their icon names.
    remaining: Vec<(String, Vec<String>)>,
    loaded: HashMap<String, IconPickerRemoteIcon>,
    total: usize,
    start: usize,
}

/// Drain a queued remote-search request (raised by the picker's
/// Load-more press). Called from the mousedown listener after the
/// `inner` borrow is released, like the other web drains.
pub(crate) fn drain_iconify_request(inner: &InnerRc) {
    let request = {
        let mut b = inner.borrow_mut();
        b.host
            .editor_state_mut()
            .editor_ui
            .icon_picker_load_more_request
            .take()
    };
    let Some(request) = request else {
        return;
    };
    let url = format!(
        "{ICONIFY_API}/search?query={}&limit={}&start={}",
        encode_component(&request.query),
        request.limit,
        request.start
    );
    let inner_cb = inner.clone();
    fetch_text(
        &url,
        Box::new(
            move |result| match result.and_then(|body| parse_search(&body, &request)) {
                Ok(page) => fetch_next_collection(inner_cb, page),
                Err(err) => apply_error(&inner_cb, &request, err),
            },
        ),
    );
}

/// Pop the next collection off the page and fetch its icon bodies;
/// when none remain, apply the assembled page.
fn fetch_next_collection(inner: InnerRc, mut page: PendingPage) {
    let Some((collection, names)) = page.remaining.pop() else {
        apply_page(&inner, page);
        return;
    };
    let url = format!(
        "{ICONIFY_API}/{}.json?icons={}",
        encode_component(&collection),
        names
            .iter()
            .map(|name| encode_component(name))
            .collect::<Vec<_>>()
            .join(",")
    );
    fetch_text(
        &url,
        Box::new(move |result| {
            match result.and_then(|body| parse_collection(&collection, &body)) {
                Ok(icons) => {
                    page.loaded.extend(icons);
                    fetch_next_collection(inner, page);
                }
                // Any collection failure fails the whole page — the
                // desktop worker behaves the same way.
                Err(err) => apply_error(&inner, &page.request, err),
            }
        }),
    );
}

// ---------------------------------------------------------------------
// Apply — mirrors `iconify_host.rs::poll_iconify_job`.
// ---------------------------------------------------------------------

fn apply_page(inner: &InnerRc, mut page: PendingPage) {
    let mut b = inner.borrow_mut();
    let ui = &mut b.host.editor_state_mut().editor_ui;
    // Stale-query guard: a newer search reset the remote state.
    if ui.icon_picker_remote.query != page.request.query {
        return;
    }
    ui.icon_picker_remote.loading = false;
    if page.start == 0 {
        ui.icon_picker_remote.icons.clear();
    }
    for id in &page.order {
        if let Some(icon) = page.loaded.remove(id) {
            if !ui
                .icon_picker_remote
                .icons
                .iter()
                .any(|i| i.collection == icon.collection && i.name == icon.name)
            {
                ui.icon_picker_remote.icons.push(icon);
            }
        }
    }
    ui.icon_picker_remote.total = page.total;
    ui.icon_picker_remote.next_start = page.start + page.request.limit;
    ui.icon_picker_remote.error = None;
    b.host.mark_editor_state_dirty();
    let _ = b.repaint();
}

fn apply_error(inner: &InnerRc, request: &IconifyLoadMoreRequest, err: String) {
    let mut b = inner.borrow_mut();
    let ui = &mut b.host.editor_state_mut().editor_ui;
    if ui.icon_picker_remote.query != request.query {
        return;
    }
    ui.icon_picker_remote.loading = false;
    ui.icon_picker_remote.error = Some(err);
    b.host.mark_editor_state_dirty();
    let _ = b.repaint();
}

// ---------------------------------------------------------------------
// Parsing — lenient `serde_json::Value` walks (web_chat idiom), same
// field contract as the desktop's typed deserializers.
// ---------------------------------------------------------------------

fn parse_search(body: &str, request: &IconifyLoadMoreRequest) -> Result<PendingPage, String> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let ids: Vec<String> = value
        .get("icons")
        .and_then(|i| i.as_array())
        .ok_or_else(|| "malformed iconify search response".to_string())?
        .iter()
        .filter_map(|v| v.as_str())
        .map(str::to_string)
        .collect();
    let total = value.get("total").and_then(|t| t.as_u64()).unwrap_or(0) as usize;
    let start = value
        .get("start")
        .and_then(|s| s.as_u64())
        .map(|s| s as usize)
        .unwrap_or(request.start);
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for id in &ids {
        if let Some((collection, name)) = id.split_once(':') {
            grouped
                .entry(collection.to_string())
                .or_default()
                .push(name.to_string());
        }
    }
    Ok(PendingPage {
        request: request.clone(),
        order: ids,
        remaining: grouped.into_iter().collect(),
        loaded: HashMap::new(),
        total,
        start,
    })
}

fn parse_collection(
    collection: &str,
    body: &str,
) -> Result<HashMap<String, IconPickerRemoteIcon>, String> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let default_w = value
        .get("width")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32);
    let default_h = value
        .get("height")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32);
    let icons = value
        .get("icons")
        .and_then(|i| i.as_object())
        .ok_or_else(|| "malformed iconify collection response".to_string())?;
    let mut out = HashMap::new();
    for (name, icon) in icons {
        let Some(svg_body) = icon.get("body").and_then(|b| b.as_str()) else {
            continue;
        };
        let w = icon
            .get("width")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .or(default_w)
            .unwrap_or(24.0);
        let h = icon
            .get("height")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .or(default_h)
            .unwrap_or(24.0);
        if let Some(parsed) =
            op_editor_ui::widgets::icon_catalog::parse_iconify_body(svg_body, w, h)
        {
            let style = match parsed.style {
                op_editor_ui::widgets::icon_catalog::IconRenderStyle::Stroke => "stroke",
                op_editor_ui::widgets::icon_catalog::IconRenderStyle::Fill => "fill",
            };
            out.insert(
                format!("{collection}:{name}"),
                IconPickerRemoteIcon {
                    collection: collection.to_string(),
                    name: name.clone(),
                    width: parsed.width,
                    height: parsed.height,
                    style: style.to_string(),
                    d: parsed.d,
                },
            );
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// XHR plumbing.
// ---------------------------------------------------------------------

/// Fire a GET and hand the body (or an error) to `on_done` exactly
/// once. The callback is slot-wrapped so the synchronous failure paths
/// (XHR construction / open / send) still resolve it — a dropped
/// callback would strand the picker's loading row forever.
fn fetch_text(url: &str, on_done: DoneFn) {
    let slot: Rc<RefCell<Option<DoneFn>>> = Rc::new(RefCell::new(Some(on_done)));
    let resolve = |slot: &Rc<RefCell<Option<DoneFn>>>, result: Result<String, String>| {
        if let Some(done) = slot.borrow_mut().take() {
            done(result);
        }
    };
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        resolve(&slot, Err("XMLHttpRequest unavailable".to_string()));
        return;
    };
    if xhr.open_with_async("GET", url, true).is_err() {
        resolve(&slot, Err("iconify request open failed".to_string()));
        return;
    }
    xhr.set_timeout(FETCH_TIMEOUT_MS);
    let xhr_cb = xhr.clone();
    let slot_cb = slot.clone();
    // `once_into_js` self-cleans after the single firing (live_sync idiom).
    let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
        let status = xhr_cb.status().unwrap_or(0);
        let result = match xhr_cb.response_text() {
            Ok(Some(text)) if (200..300).contains(&status) => Ok(text),
            _ => Err(format!("iconify HTTP {status}")),
        };
        if let Some(done) = slot_cb.borrow_mut().take() {
            done(result);
        }
    });
    xhr.set_onloadend(Some(onloadend.unchecked_ref()));
    if xhr.send().is_err() {
        resolve(&slot, Err("iconify request send failed".to_string()));
    }
}

/// Percent-encode a URL component (same table as the desktop host).
fn encode_component(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> IconifyLoadMoreRequest {
        IconifyLoadMoreRequest {
            query: "home".to_string(),
            start: 0,
            limit: 48,
        }
    }

    #[test]
    fn parse_search_groups_ids_by_collection_and_keeps_order() {
        let body = r#"{"icons":["mdi:home","ph:house","mdi:home-circle"],"total":120,"start":0}"#;
        let page = parse_search(body, &request()).expect("page");
        assert_eq!(page.order, vec!["mdi:home", "ph:house", "mdi:home-circle"]);
        assert_eq!(page.total, 120);
        assert_eq!(page.start, 0);
        let mut collections: Vec<(String, usize)> = page
            .remaining
            .iter()
            .map(|(c, names)| (c.clone(), names.len()))
            .collect();
        collections.sort();
        assert_eq!(
            collections,
            vec![("mdi".to_string(), 2), ("ph".to_string(), 1)]
        );
    }

    #[test]
    fn parse_search_defaults_mirror_the_desktop_worker() {
        // Missing total → 0; missing start → request.start.
        let body = r#"{"icons":["mdi:home"]}"#;
        let mut req = request();
        req.start = 48;
        let page = parse_search(body, &req).expect("page");
        assert_eq!(page.total, 0);
        assert_eq!(page.start, 48);
    }

    #[test]
    fn parse_search_rejects_malformed_payloads() {
        assert!(parse_search("not json", &request()).is_err());
        assert!(parse_search(r#"{"nope":true}"#, &request()).is_err());
    }

    #[test]
    fn parse_collection_extracts_path_data_with_size_fallbacks() {
        let body = r#"{"width":24,"height":24,"icons":{
            "home":{"body":"<path d=\"M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z\"/>"},
            "wide":{"body":"<path d=\"M0 0h32v32H0z\"/>","width":32,"height":32}
        }}"#;
        let icons = parse_collection("mdi", body).expect("icons");
        let home = icons.get("mdi:home").expect("home");
        assert_eq!(home.collection, "mdi");
        assert_eq!(home.name, "home");
        assert_eq!(home.width, 24.0);
        assert!(home.d.starts_with("M3 9"));
        let wide = icons.get("mdi:wide").expect("wide");
        assert_eq!(wide.width, 32.0);
    }

    #[test]
    fn encode_component_percent_encodes_reserved_bytes() {
        assert_eq!(encode_component("home icon"), "home%20icon");
        assert_eq!(encode_component("a+b/c"), "a%2Bb%2Fc");
        assert_eq!(encode_component("safe-_.~09AZ"), "safe-_.~09AZ");
    }
}
